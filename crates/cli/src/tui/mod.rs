pub mod app;
pub mod ui;

use crate::client::JotClient;
use crate::error::CliError;
use crate::tui::app::{App, ConfirmAction, Focus, Mode};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::Duration;

pub async fn run_tui() -> Result<(), CliError> {
    let client = JotClient::from_config();
    let mut app = App::new(client);

    match app.client.clone().get_boards().await {
        Ok(boards) => {
            app.boards = boards;
            app.status = "Boards loaded.".to_string();
        }
        Err(e) => {
            app.status = format!("Error: {}", e);
        }
    }

    if let Some(board_id) = app.current_board_id() {
        match app.client.clone().get_notes(board_id).await {
            Ok(notes) => app.notes = notes,
            Err(e) => app.status = format!("Error loading notes: {}", e),
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
                    Mode::Input(_) => handle_input(app, code).await,
                    Mode::Confirm(action) => handle_confirm(app, code, action).await,
                }
                if quit {
                    break;
                }
            }
        }
    }
    Ok(())
}

async fn handle_normal(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('q') => {}
        KeyCode::Char('n') => app.start_input(),
        KeyCode::Char('d') => app.start_delete(),
        KeyCode::Tab => app.toggle_focus(),
        KeyCode::Char('r') => refresh(app).await,
        KeyCode::Char('j') | KeyCode::Down => match app.focus {
            Focus::Boards => {
                app.board_down();
                load_notes(app).await;
            }
            Focus::Notes => app.note_down(),
        },
        KeyCode::Char('k') | KeyCode::Up => match app.focus {
            Focus::Boards => {
                app.board_up();
                load_notes(app).await;
            }
            Focus::Notes => app.note_up(),
        },
        KeyCode::Enter if app.focus == Focus::Boards => {
            load_notes(app).await;
        }
        _ => {}
    }
}

async fn handle_input(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => app.cancel_mode(),
        KeyCode::Backspace => app.input_pop(),
        KeyCode::Enter => {
            let content = if let Mode::Input(ref buf) = app.mode {
                buf.clone()
            } else {
                return;
            };
            app.cancel_mode();
            if let Some(board_id) = app.current_board_id() {
                match app.client.clone().create_note(board_id, &content).await {
                    Ok(_) => {
                        app.status = "Note created.".to_string();
                        load_notes(app).await;
                    }
                    Err(e) => app.status = format!("Error: {}", e),
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
            let ConfirmAction::DeleteNote(id) = action;
            match app.client.clone().delete_note(id).await {
                Ok(_) => {
                    app.status = "Note deleted.".to_string();
                    load_notes(app).await;
                }
                Err(e) => app.status = format!("Error: {}", e),
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
            }
            Err(e) => app.status = format!("Error: {}", e),
        }
    }
}

async fn refresh(app: &mut App) {
    match app.client.clone().get_boards().await {
        Ok(boards) => {
            app.boards = boards;
            app.status = "Refreshed.".to_string();
        }
        Err(e) => {
            app.status = format!("Error: {}", e);
            return;
        }
    }
    load_notes(app).await;
}
