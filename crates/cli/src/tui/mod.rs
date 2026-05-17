pub mod app;
pub mod blocks;
pub mod ui;

use crate::client::JotClient;
use crate::error::CliError;
use crate::t;
use crate::tui::app::{App, ConfirmAction, Focus, InputContext, Mode, View};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::Duration;
use uuid::Uuid;

pub async fn run_tui() -> Result<(), CliError> {
    let client = JotClient::from_config();
    let mut app = App::new(client);

    match app.client.clone().get_boards().await {
        Ok(boards) => {
            app.boards = boards;
            app.status = t!("tui.msg.boardsLoaded");
        }
        Err(e) => {
            app.status = t!("tui.error.prefix", "msg" => e);
        }
    }

    if let Some(board_id) = app.current_board_id() {
        match app.client.clone().get_notes(board_id).await {
            Ok(notes) => app.notes = notes,
            Err(e) => app.status = t!("tui.error.loadNotes", "msg" => e),
        }
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = event_loop(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

async fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<(), CliError> {
    loop {
        terminal.draw(|f| ui::render(f, app))?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                let quit = matches!((&app.mode, code), (Mode::Normal, KeyCode::Char('q')));
                match app.mode.clone() {
                    Mode::Normal => handle_normal(app, code).await,
                    Mode::Input(ctx, _) => handle_input(app, code, ctx).await,
                    Mode::Confirm(action) => handle_confirm(app, code, action).await,
                }
                if quit || app.should_quit {
                    break;
                }
            }
        }

        if let Some(note_id) = app.pending_edit.take() {
            edit_in_editor(terminal, app, note_id).await?;
        }

        if let Some(block_id) = app.pending_block_edit.take() {
            edit_block_in_editor(terminal, app, block_id).await?;
        }
    }
    Ok(())
}

async fn edit_block_in_editor(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    block_id: Uuid,
) -> Result<(), CliError> {
    let note_id = match app.block_panel.note_id {
        Some(n) => n,
        None => return Ok(()),
    };
    let board_id = match app.block_panel.board_id {
        Some(b) => b,
        None => return Ok(()),
    };
    let original = app
        .block_panel
        .plaintexts
        .get(&block_id)
        .cloned()
        .unwrap_or_default();

    // Leave TUI
    disable_raw_mode().map_err(|e| CliError::Server(format!("raw mode: {e}")))?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .map_err(|e| CliError::Server(format!("leave screen: {e}")))?;
    terminal
        .show_cursor()
        .map_err(|e| CliError::Server(format!("show cursor: {e}")))?;

    let edited_result = crate::commands::block::edit_in_editor(&original);

    // Re-enter TUI
    enable_raw_mode().map_err(|e| CliError::Server(format!("raw mode: {e}")))?;
    execute!(
        terminal.backend_mut(),
        EnterAlternateScreen,
        EnableMouseCapture
    )
    .map_err(|e| CliError::Server(format!("enter screen: {e}")))?;
    terminal
        .clear()
        .map_err(|e| CliError::Server(format!("clear: {e}")))?;

    let edited = match edited_result {
        Ok(s) => s,
        Err(e) => {
            app.status = t!("tui.error.prefix", "msg" => e);
            return Ok(());
        }
    };

    if edited == original {
        app.status = t!("tui.msg.noChanges");
        return Ok(());
    }

    // Encrypt with note DEK, base64-encode, PATCH the block.
    let ciphertext = match app
        .client
        .clone()
        .encrypt_with_note_dek(board_id, note_id, edited.as_bytes())
        .await
    {
        Ok(c) => c,
        Err(e) => {
            app.status = t!("tui.error.prefix", "msg" => e);
            return Ok(());
        }
    };
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&ciphertext);
    if let Err(e) = app
        .client
        .clone()
        .patch_block_content_b64(block_id, &b64)
        .await
    {
        app.status = t!("tui.error.prefix", "msg" => e);
        return Ok(());
    }

    reload_block_panel(app, note_id, board_id).await;
    app.status = t!("tui.msg.noteSaved");
    Ok(())
}

async fn handle_normal(app: &mut App, code: KeyCode) {
    // Global shortcuts: jump to special views from anywhere
    match code {
        KeyCode::Char('p') => {
            app.view = View::Profile;
            app.focus = Focus::Boards;
            load_profile(app).await;
            return;
        }
        KeyCode::Char('t') => {
            app.view = View::Stats;
            app.focus = Focus::Boards;
            load_stats(app).await;
            return;
        }
        KeyCode::Char('v') => {
            app.view = View::Devices;
            app.focus = Focus::Boards;
            load_devices_data(app).await;
            return;
        }
        KeyCode::Esc => {
            if !matches!(
                app.view,
                View::MyBoards | View::SharedBoards | View::SharedNotes
            ) {
                app.view = View::MyBoards;
                app.focus = Focus::Boards;
                reload_boards(app).await;
            }
            return;
        }
        _ => {}
    }

    // Devices view: dedicated key handling
    if app.view == View::Devices {
        match code {
            KeyCode::Char('j') | KeyCode::Down => app.device_down(),
            KeyCode::Char('k') | KeyCode::Up => app.device_up(),
            KeyCode::Char('r') => app.start_rename_device(),
            KeyCode::Char('d') => app.start_delete_device(),
            _ => {}
        }
        return;
    }

    // Profile / Stats: read-only, no extra key handling
    if matches!(app.view, View::Profile | View::Stats) {
        return;
    }

    // Block-tree keybindings — fire when the right pane is focused on a
    // v1 text note and the BlockPanel has been loaded.
    if app.focus == Focus::Notes
        && matches!(app.view, View::MyBoards | View::SharedBoards)
        && app.block_panel.note_id.is_some()
        && app
            .notes
            .get(app.selected_note)
            .map(|n| n.note_type == "text" && n.schema_version >= 1)
            .unwrap_or(false)
        && handle_block_keys(app, code).await
    {
        return;
    }

    // Normal MyBoards / SharedBoards / SharedNotes handling
    match code {
        KeyCode::Char('q') => {}
        KeyCode::Char('n') => match app.focus {
            Focus::Boards => app.start_input_board(),
            Focus::Notes => app.start_input(),
        },
        KeyCode::Char('r') => match app.focus {
            Focus::Boards => {
                if let Some(id) = app.current_board_id() {
                    let current = app
                        .boards
                        .get(app.selected_board)
                        .map(|b| (b.id, b.name.clone()));
                    if let Some((bid, name)) = current {
                        if bid == id {
                            app.start_rename_board(id, &name);
                        }
                    }
                }
            }
            Focus::Notes => refresh(app).await,
        },
        KeyCode::Char('D') => {
            if app.focus == Focus::Boards {
                app.start_delete_board();
            }
        }
        KeyCode::Char('d') => {
            if app.focus == Focus::Notes {
                app.start_delete();
            }
        }
        KeyCode::Char('e') => {
            if app.focus == Focus::Notes {
                app.pending_edit = app.current_note_id();
            }
        }
        KeyCode::Char('S') => {
            app.cycle_view();
            load_view_data(app).await;
        }
        KeyCode::Tab => app.toggle_focus(),
        KeyCode::Char('j') | KeyCode::Down => match app.focus {
            Focus::Boards => {
                app.board_down();
                app.set_note_content(None);
                load_notes(app).await;
            }
            Focus::Notes => {
                app.note_down();
                app.set_note_content(None);
            }
        },
        KeyCode::Char('k') | KeyCode::Up => match app.focus {
            Focus::Boards => {
                app.board_up();
                app.set_note_content(None);
                load_notes(app).await;
            }
            Focus::Notes => {
                app.note_up();
                app.set_note_content(None);
            }
        },
        KeyCode::Enter if app.focus == Focus::Boards => {
            load_notes(app).await;
        }
        KeyCode::Enter if app.focus == Focus::Notes => {
            load_note_content(app).await;
        }
        KeyCode::PageDown => {
            app.content_scroll = app.content_scroll.saturating_add(10);
        }
        KeyCode::PageUp => {
            app.content_scroll = app.content_scroll.saturating_sub(10);
        }
        KeyCode::Char('X') => app.start_delete_account(),
        _ => {}
    }
}

/// Returns `true` if the key was consumed by the block dispatcher.
async fn handle_block_keys(app: &mut App, code: KeyCode) -> bool {
    let note_id = match app.block_panel.note_id {
        Some(n) => n,
        None => return false,
    };
    let board_id = match app.block_panel.board_id {
        Some(b) => b,
        None => return false,
    };

    let flat: Vec<jot_core::models::Block> =
        crate::tui::blocks::flatten_depth_first(&app.block_panel.blocks)
            .into_iter()
            .cloned()
            .collect();
    let flat_len = flat.len();
    let current = flat.get(app.block_panel.cursor).cloned();

    match code {
        KeyCode::Char('j') => {
            if app.block_panel.cursor + 1 < flat_len {
                app.block_panel.cursor += 1;
            }
            app.block_panel.pending = None;
            true
        }
        KeyCode::Char('k') => {
            if app.block_panel.cursor > 0 {
                app.block_panel.cursor -= 1;
            }
            app.block_panel.pending = None;
            true
        }
        KeyCode::Char('o') => {
            let (parent, position) = match current.as_ref() {
                Some(c) => (c.parent_block_id, Some(c.position + 0.5)),
                None => (None, None),
            };
            match app
                .client
                .clone()
                .create_block_encrypted(note_id, parent, position, "text", b"")
                .await
            {
                Ok(_) => reload_block_panel(app, note_id, board_id).await,
                Err(e) => app.status = t!("tui.error.prefix", "msg" => e),
            }
            app.block_panel.pending = None;
            true
        }
        KeyCode::Char('>') => {
            if let Some(c) = current.as_ref() {
                if let Err(e) = app.client.clone().indent_block(c.id).await {
                    app.status = t!("tui.error.prefix", "msg" => e);
                } else {
                    reload_block_panel(app, note_id, board_id).await;
                }
            }
            app.block_panel.pending = None;
            true
        }
        KeyCode::Char('<') => {
            if let Some(c) = current.as_ref() {
                if let Err(e) = app.client.clone().outdent_block(c.id).await {
                    app.status = t!("tui.error.prefix", "msg" => e);
                } else {
                    reload_block_panel(app, note_id, board_id).await;
                }
            }
            app.block_panel.pending = None;
            true
        }
        KeyCode::Char('d') => {
            if app.block_panel.pending == Some('d') {
                if let Some(c) = current.as_ref() {
                    if let Err(e) = app.client.clone().delete_block(c.id).await {
                        app.status = t!("tui.error.prefix", "msg" => e);
                    } else {
                        reload_block_panel(app, note_id, board_id).await;
                        let new_len =
                            crate::tui::blocks::flatten_depth_first(&app.block_panel.blocks).len();
                        if app.block_panel.cursor >= new_len && app.block_panel.cursor > 0 {
                            app.block_panel.cursor -= 1;
                        }
                    }
                }
                app.block_panel.pending = None;
            } else {
                app.block_panel.pending = Some('d');
            }
            true
        }
        KeyCode::Char('y') => {
            if app.block_panel.pending == Some('y') {
                if let Some(c) = current.as_ref() {
                    match arboard::Clipboard::new() {
                        Ok(mut cb) => {
                            if let Err(e) = cb.set_text(format!("(({}))", c.id)) {
                                app.status = t!("tui.error.prefix", "msg" => e.to_string());
                            } else {
                                app.status = format!("Yanked (({}))", c.id);
                            }
                        }
                        Err(e) => app.status = t!("tui.error.prefix", "msg" => e.to_string()),
                    }
                }
                app.block_panel.pending = None;
            } else {
                app.block_panel.pending = Some('y');
            }
            true
        }
        KeyCode::Char('z') => {
            app.block_panel.pending = Some('z');
            true
        }
        KeyCode::Char('a') if app.block_panel.pending == Some('z') => {
            if let Some(c) = current.as_ref() {
                if let Err(e) = app
                    .client
                    .clone()
                    .patch_block_collapse(c.id, !c.collapsed)
                    .await
                {
                    app.status = t!("tui.error.prefix", "msg" => e);
                } else {
                    reload_block_panel(app, note_id, board_id).await;
                }
            }
            app.block_panel.pending = None;
            true
        }
        KeyCode::Enter => {
            // Defer to the event loop so we can leave the TUI cleanly.
            if let Some(c) = current.as_ref() {
                app.pending_block_edit = Some(c.id);
            }
            app.block_panel.pending = None;
            true
        }
        _ => {
            app.block_panel.pending = None;
            false
        }
    }
}

async fn reload_block_panel(app: &mut App, note_id: Uuid, board_id: Uuid) {
    match crate::tui::blocks::load(&app.client, note_id, board_id).await {
        Ok((blocks, pts)) => {
            app.block_panel.blocks = blocks;
            app.block_panel.plaintexts = pts;
        }
        Err(e) => app.status = t!("tui.error.prefix", "msg" => e),
    }
}

async fn handle_input(app: &mut App, code: KeyCode, ctx: InputContext) {
    match code {
        KeyCode::Esc => app.cancel_mode(),
        KeyCode::Backspace => app.input_pop(),
        KeyCode::Enter => {
            let buf = if let Mode::Input(_, ref b) = app.mode {
                b.clone()
            } else {
                return;
            };
            app.cancel_mode();
            match ctx {
                InputContext::NewNote => {
                    if let Some(board_id) = app.current_board_id() {
                        match app.client.clone().create_note(board_id, &buf).await {
                            Ok(_) => {
                                app.status = t!("tui.msg.noteCreated");
                                load_notes(app).await;
                            }
                            Err(e) => app.status = t!("tui.error.prefix", "msg" => e),
                        }
                    }
                }
                InputContext::NewBoard => match app.client.clone().create_board(&buf).await {
                    Ok(_) => {
                        app.status = t!("tui.msg.boardCreated");
                        reload_boards(app).await;
                    }
                    Err(e) => app.status = t!("tui.error.prefix", "msg" => e),
                },
                InputContext::RenameBoard(id) => {
                    match app.client.clone().rename_board(id, &buf).await {
                        Ok(_) => {
                            app.status = t!("tui.msg.boardRenamed");
                            reload_boards(app).await;
                        }
                        Err(e) => app.status = t!("tui.error.prefix", "msg" => e),
                    }
                }
                InputContext::RenameDevice(id) => {
                    match app.client.clone().rename_device(id, &buf).await {
                        Ok(_) => {
                            app.status = t!("tui.msg.deviceRenamed");
                            load_devices_data(app).await;
                        }
                        Err(e) => app.status = t!("tui.error.prefix", "msg" => e),
                    }
                }
            }
        }
        KeyCode::Char(c) => app.input_push(c),
        _ => {}
    }
}

async fn handle_confirm(app: &mut App, code: KeyCode, action: ConfirmAction) {
    match code {
        KeyCode::Char('y') => {
            app.cancel_mode();
            match action {
                ConfirmAction::DeleteNote(id) => match app.client.clone().delete_note(id).await {
                    Ok(_) => {
                        app.status = t!("tui.msg.noteDeleted");
                        load_notes(app).await;
                    }
                    Err(e) => app.status = t!("tui.error.prefix", "msg" => e),
                },
                ConfirmAction::DeleteBoard(id) => match app.client.clone().delete_board(id).await {
                    Ok(_) => {
                        app.status = t!("tui.msg.boardDeleted");
                        reload_boards(app).await;
                    }
                    Err(e) => app.status = t!("tui.error.prefix", "msg" => e),
                },
                ConfirmAction::DeleteDevice(id) => {
                    match app.client.clone().delete_device(id).await {
                        Ok(_) => {
                            app.status = t!("tui.msg.deviceDeleted");
                            load_devices_data(app).await;
                        }
                        Err(e) => app.status = t!("tui.error.prefix", "msg" => e),
                    }
                }
                ConfirmAction::DeleteAccount => match app.client.clone().delete_account().await {
                    Ok(_) => {
                        let mut config = crate::config::Config::load();
                        config.token = None;
                        config.identity_id = None;
                        config.device_id = None;
                        let _ = config.save();
                        app.status = t!("tui.msg.accountDeleted");
                        app.should_quit = true;
                    }
                    Err(e) => app.status = t!("tui.error.prefix", "msg" => e),
                },
            }
        }
        KeyCode::Char('n') | KeyCode::Esc => app.cancel_mode(),
        _ => {}
    }
}

async fn load_notes(app: &mut App) {
    if let Some(board_id) = app.current_board_id() {
        match app.client.clone().get_notes(board_id).await {
            Ok(notes) => {
                app.notes = notes;
                app.selected_note = 0;
                app.set_note_content(None);
            }
            Err(e) => app.status = t!("tui.error.prefix", "msg" => e),
        }
    }
}

async fn load_note_content(app: &mut App) {
    if let Some(note_id) = app.current_note_id() {
        // If this is a block-structured text note, load its block tree instead
        // of (or in addition to) the flat blob content.
        let is_block_note = matches!(app.view, View::MyBoards | View::SharedBoards)
            && app
                .notes
                .get(app.selected_note)
                .map(|n| n.note_type == "text" && n.schema_version >= 1)
                .unwrap_or(false);

        if is_block_note {
            if let Some(board_id) = app.current_board_id() {
                app.loading_content = true;
                app.note_content = None;
                app.block_panel.clear();
                match crate::tui::blocks::load(&app.client, note_id, board_id).await {
                    Ok((blocks, plaintexts)) => {
                        app.block_panel.note_id = Some(note_id);
                        app.block_panel.board_id = Some(board_id);
                        app.block_panel.blocks = blocks;
                        app.block_panel.plaintexts = plaintexts;
                        app.block_panel.cursor = 0;
                        app.loading_content = false;
                        app.status = t!("tui.msg.noteLoaded");
                    }
                    Err(e) => {
                        app.loading_content = false;
                        app.status = t!("tui.error.prefix", "msg" => e);
                    }
                }
                return;
            }
        }

        app.block_panel.clear();
        app.loading_content = true;
        app.note_content = None;
        match app.client.clone().get_note_text(note_id).await {
            Ok(text) => {
                app.set_note_content(Some(text));
                app.status = t!("tui.msg.noteLoaded");
            }
            Err(e) => {
                app.loading_content = false;
                app.status = t!("tui.error.prefix", "msg" => e);
            }
        }
    }
}

async fn reload_boards(app: &mut App) {
    match app.client.clone().get_boards().await {
        Ok(boards) => {
            app.boards = boards;
        }
        Err(e) => app.status = t!("tui.error.prefix", "msg" => e),
    }
}

async fn load_profile(app: &mut App) {
    match app.client.clone().get_identity_me().await {
        Ok(info) => {
            app.identity_info = Some(info);
            app.status = t!("tui.msg.profileLoaded");
        }
        Err(e) => app.status = t!("tui.error.prefix", "msg" => e),
    }
}

async fn load_stats(app: &mut App) {
    match app.client.clone().get_boards().await {
        Ok(boards) => {
            let mut stats = Vec::new();
            for board in &boards {
                let count = match app.client.clone().get_notes(board.id).await {
                    Ok(notes) => notes.len(),
                    Err(_) => 0,
                };
                stats.push((board.name.clone(), count));
            }
            app.stats = stats;
            app.status = t!("tui.msg.statsLoaded");
        }
        Err(e) => app.status = t!("tui.error.prefix", "msg" => e),
    }
}

async fn load_devices_data(app: &mut App) {
    match app.client.clone().get_devices().await {
        Ok(devices) => {
            app.devices = devices;
            app.selected_device = 0;
            app.status = t!("tui.msg.devicesLoaded");
        }
        Err(e) => app.status = t!("tui.error.prefix", "msg" => e),
    }
}

async fn load_shared_boards(app: &mut App) {
    match app.client.clone().get_shared_boards().await {
        Ok(boards) => {
            app.shared_boards = boards;
            app.selected_shared_board = 0;
            app.status = t!("tui.msg.sharedBoardsLoaded");
        }
        Err(e) => app.status = t!("tui.error.prefix", "msg" => e),
    }
}

async fn load_shared_notes(app: &mut App) {
    match app.client.clone().get_shared_notes().await {
        Ok(notes) => {
            app.shared_notes = notes;
            app.selected_shared_note = 0;
            app.status = t!("tui.msg.sharedNotesLoaded");
        }
        Err(e) => app.status = t!("tui.error.prefix", "msg" => e),
    }
}

async fn load_view_data(app: &mut App) {
    match app.view {
        View::MyBoards => reload_boards(app).await,
        View::SharedBoards => load_shared_boards(app).await,
        View::SharedNotes => load_shared_notes(app).await,
        View::Profile => load_profile(app).await,
        View::Stats => load_stats(app).await,
        View::Devices => load_devices_data(app).await,
    }
}

async fn refresh(app: &mut App) {
    match app.client.clone().get_boards().await {
        Ok(boards) => {
            app.boards = boards;
            app.status = t!("tui.msg.refreshed");
        }
        Err(e) => {
            app.status = t!("tui.error.prefix", "msg" => e);
            return;
        }
    }
    load_notes(app).await;
}

async fn edit_in_editor(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    note_id: Uuid,
) -> Result<(), CliError> {
    // Ensure we have current content
    if app.note_content.is_none() {
        match app.client.clone().get_note_text(note_id).await {
            Ok(text) => app.note_content = Some(text),
            Err(e) => {
                app.status = t!("tui.error.loadNote", "msg" => e);
                return Ok(());
            }
        }
    }
    let original = app.note_content.clone().unwrap_or_default();

    // Write to temp file
    let tmp = tempfile::Builder::new()
        .suffix(".txt")
        .tempfile()
        .map_err(|e| CliError::Server(format!("tempfile error: {e}")))?;
    std::fs::write(tmp.path(), &original)
        .map_err(|e| CliError::Server(format!("write tempfile: {e}")))?;

    // Leave TUI
    disable_raw_mode().map_err(|e| CliError::Server(format!("raw mode: {e}")))?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .map_err(|e| CliError::Server(format!("leave screen: {e}")))?;
    terminal
        .show_cursor()
        .map_err(|e| CliError::Server(format!("show cursor: {e}")))?;

    // Launch editor: $VISUAL, then $EDITOR, then first candidate found in PATH
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| {
            for candidate in &["nvim", "vim", "nano", "vi"] {
                if std::process::Command::new("which")
                    .arg(candidate)
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false)
                {
                    return candidate.to_string();
                }
            }
            "vi".to_string()
        });
    std::process::Command::new(&editor)
        .arg(tmp.path())
        .status()
        .map_err(|e| CliError::Server(format!("editor launch failed: {e}")))?;

    // Read back
    let new_content = std::fs::read_to_string(tmp.path())
        .map_err(|e| CliError::Server(format!("read tempfile: {e}")))?;

    // Re-enter TUI
    enable_raw_mode().map_err(|e| CliError::Server(format!("raw mode: {e}")))?;
    execute!(
        terminal.backend_mut(),
        EnterAlternateScreen,
        EnableMouseCapture
    )
    .map_err(|e| CliError::Server(format!("enter screen: {e}")))?;
    terminal
        .clear()
        .map_err(|e| CliError::Server(format!("clear: {e}")))?;

    // Save if changed
    if new_content != original {
        match app.client.clone().update_note(note_id, &new_content).await {
            Ok(_) => {
                app.note_content = Some(new_content);
                app.status = t!("tui.msg.noteSaved");
            }
            Err(e) => app.status = t!("tui.error.saveNote", "msg" => e),
        }
    } else {
        app.status = t!("tui.msg.noChanges");
    }

    Ok(())
}
