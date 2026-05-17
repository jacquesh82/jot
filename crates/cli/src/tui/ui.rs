use crate::t;
use crate::tui::app::{App, ConfirmAction, Focus, InputContext, Mode, View};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(frame.area());

    match app.view {
        View::Profile => render_profile(frame, app, chunks[0]),
        View::Stats => render_stats(frame, app, chunks[0]),
        View::Devices => render_devices(frame, app, chunks[0]),
        _ => {
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
                .split(chunks[0]);
            let right_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
                .split(main_chunks[1]);
            render_left_pane(frame, app, main_chunks[0]);
            render_notes(frame, app, right_chunks[0]);
            render_content(frame, app, right_chunks[1]);
        }
    }
    render_status(frame, app, chunks[1]);
}

fn render_left_pane(frame: &mut Frame, app: &App, area: Rect) {
    match app.view {
        View::SharedNotes => render_shared_notes_placeholder(frame, area),
        View::Profile | View::Stats | View::Devices => {}
        _ => render_boards(frame, app, area),
    }
}

fn render_shared_notes_placeholder(frame: &mut Frame, area: Rect) {
    let paragraph = Paragraph::new("\u{2014} Shared Notes \u{2014}").block(
        Block::default()
            .borders(Borders::ALL)
            .title(t!("tui.title.boardsSharedNotes")),
    );
    frame.render_widget(paragraph, area);
}

fn render_boards(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == Focus::Boards;
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let (title, items, selected_idx) = match app.view {
        View::MyBoards => {
            let items: Vec<ListItem> = app
                .boards
                .iter()
                .map(|b| ListItem::new(b.name.clone()))
                .collect();
            (t!("tui.title.boardsMy"), items, app.selected_board)
        }
        View::SharedBoards => {
            let items: Vec<ListItem> = app
                .shared_boards
                .iter()
                .map(|b| {
                    let owner = b
                        .owner_friendly_name
                        .as_deref()
                        .unwrap_or(&b.owner_identity_id[..8.min(b.owner_identity_id.len())]);
                    ListItem::new(format!("{} ({})", b.board_name, owner))
                })
                .collect();
            (
                t!("tui.title.boardsShared"),
                items,
                app.selected_shared_board,
            )
        }
        View::SharedNotes | View::Profile | View::Stats | View::Devices => unreachable!(),
    };

    let has_items = !items.is_empty();
    let mut state = ListState::default();
    if has_items {
        state.select(Some(selected_idx));
    }

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(border_style),
        )
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        )
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, area, &mut state);
}

fn render_notes(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == Focus::Notes;
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let (items, selected_idx): (Vec<ListItem>, usize) = match app.view {
        View::SharedNotes => {
            let items = app
                .shared_notes
                .iter()
                .map(|n| {
                    let id_str = n.note_id.to_string();
                    let owner = n
                        .owner_friendly_name
                        .as_deref()
                        .unwrap_or(&n.owner_identity_id[..8.min(n.owner_identity_id.len())]);
                    let label = n
                        .snippet
                        .as_deref()
                        .filter(|s| !s.is_empty())
                        .unwrap_or(&id_str[..8]);
                    ListItem::new(format!("{} ({})", label, owner))
                })
                .collect();
            (items, app.selected_shared_note)
        }
        _ => {
            let items = app
                .notes
                .iter()
                .map(|n| {
                    let id_str = n.id.to_string();
                    let label = n
                        .snippet
                        .as_deref()
                        .filter(|s| !s.is_empty())
                        .unwrap_or(&id_str[..8]);
                    ListItem::new(label.to_string())
                })
                .collect();
            (items, app.selected_note)
        }
    };

    let has_items = !items.is_empty();
    let mut state = ListState::default();
    if has_items {
        state.select(Some(selected_idx));
    }

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(t!("tui.title.notes"))
                .border_style(border_style),
        )
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        )
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, area, &mut state);
}

fn render_content(frame: &mut Frame, app: &App, area: Rect) {
    // Gate: text notes with schema_version >= 1 render as a hierarchical block tree.
    // Voice/image notes and legacy (schema_version 0) notes keep the flat paragraph view.
    if matches!(app.view, View::MyBoards | View::SharedBoards) {
        if let Some(note) = app.notes.get(app.selected_note) {
            if note.note_type == "text" && note.schema_version >= 1 {
                crate::tui::blocks::render(frame, area, &app.block_panel);
                return;
            }
        }
    }

    let title = if app.loading_content {
        t!("tui.title.contentLoading")
    } else {
        match app.view {
            View::SharedNotes => app
                .shared_notes
                .get(app.selected_shared_note)
                .and_then(|n| n.snippet.as_deref().filter(|s| !s.is_empty()))
                .map(|s| s.to_string())
                .unwrap_or_else(|| t!("tui.title.content")),
            _ => app
                .notes
                .get(app.selected_note)
                .and_then(|n| n.snippet.as_deref().filter(|s| !s.is_empty()))
                .map(|s| s.to_string())
                .unwrap_or_else(|| t!("tui.title.content")),
        }
    };

    let text = match &app.note_content {
        Some(content) => content.clone(),
        None if app.loading_content => String::new(),
        None => t!("tui.hint.pressEnter"),
    };

    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title(title))
        .wrap(Wrap { trim: false })
        .scroll((app.content_scroll, 0));

    frame.render_widget(paragraph, area);
}

fn render_status(frame: &mut Frame, app: &App, area: Rect) {
    let text = match &app.mode {
        Mode::Normal => {
            let hint = match app.view {
                View::Profile => t!("tui.status.profileFocus"),
                View::Stats => t!("tui.status.statsFocus"),
                View::Devices => t!("tui.status.devicesFocus"),
                View::MyBoards => match app.focus {
                    Focus::Boards => t!("tui.status.boardsFocus"),
                    Focus::Notes => t!("tui.status.notesFocus"),
                },
                View::SharedBoards => match app.focus {
                    Focus::Boards => t!("tui.status.boardsSharedFocus"),
                    Focus::Notes => t!("tui.status.notesFocus"),
                },
                View::SharedNotes => match app.focus {
                    Focus::Boards => t!("tui.status.boardsSharedNotesFocus"),
                    Focus::Notes => t!("tui.status.notesFocus"),
                },
            };
            Line::from(vec![Span::raw(hint)])
        }
        Mode::Input(ctx, buf) => {
            let label = match ctx {
                InputContext::NewNote => t!("tui.prompt.newNote"),
                InputContext::NewBoard => t!("tui.prompt.newBoard"),
                InputContext::RenameBoard(_) => t!("tui.prompt.renameBoard"),
                InputContext::RenameDevice(_) => t!("tui.prompt.renameDevice"),
            };
            Line::from(vec![
                Span::styled(label, Style::default().fg(Color::Green)),
                Span::raw(buf.clone()),
                Span::styled("\u{258c}", Style::default().fg(Color::Green)),
            ])
        }
        Mode::Confirm(action) => {
            let msg = match action {
                ConfirmAction::DeleteNote(_) => t!("tui.confirm.deleteNote"),
                ConfirmAction::DeleteBoard(_) => t!("tui.confirm.deleteBoard"),
                ConfirmAction::DeleteDevice(_) => t!("tui.confirm.deleteDevice"),
                ConfirmAction::DeleteAccount => t!("tui.confirm.deleteAccount"),
            };
            Line::from(vec![Span::styled(msg, Style::default().fg(Color::Red))])
        }
    };

    frame.render_widget(Paragraph::new(text), area);
}

fn render_profile(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let name = app
        .identity_info
        .as_ref()
        .map(|i| i.friendly_name.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(&t!("tui.label.notSet"))
        .to_string();
    let identity_id = app
        .identity_info
        .as_ref()
        .map(|i| i.id.as_str())
        .unwrap_or(&t!("tui.label.notSet"))
        .to_string();

    let identity_text = format!(
        "{}: {}\n\n{}: {}",
        t!("tui.label.name"),
        name,
        t!("tui.label.identityId"),
        identity_id,
    );
    let identity = Paragraph::new(identity_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(t!("tui.title.identity")),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(identity, chunks[0]);

    let token_status = if app.client.token.is_some() {
        t!("tui.label.tokenPresent")
    } else {
        t!("tui.label.tokenAbsent")
    };
    let device_id = app
        .current_device_id
        .as_deref()
        .unwrap_or(&t!("tui.label.notSet"))
        .to_string();
    let device_text = format!(
        "{}: {}\n\n{}: {}\n\n{}: {}",
        t!("tui.label.deviceId"),
        device_id,
        t!("tui.label.server"),
        app.server_url,
        t!("tui.label.auth"),
        token_status,
    );
    let device = Paragraph::new(device_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(t!("tui.title.deviceInfo")),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(device, chunks[1]);
}

fn render_stats(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    let items: Vec<ListItem> = app
        .stats
        .iter()
        .map(|(name, count)| ListItem::new(format!("{:<32} {:>4}", name, count)))
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(t!("tui.title.stats")),
    );
    frame.render_widget(list, chunks[0]);

    let total_boards = app.stats.len();
    let total_notes: usize = app.stats.iter().map(|(_, c)| c).sum();
    let avg = if total_boards > 0 {
        total_notes as f64 / total_boards as f64
    } else {
        0.0
    };
    let summary = format!(
        "{}: {}\n\n{}: {}\n\n{}: {:.1}",
        t!("tui.label.totalBoards"),
        total_boards,
        t!("tui.label.totalNotes"),
        total_notes,
        t!("tui.label.avgNotesPerBoard"),
        avg,
    );
    let summary_widget = Paragraph::new(summary)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(t!("tui.title.statsSummary")),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(summary_widget, chunks[1]);
}

fn render_devices(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    let items: Vec<ListItem> = app
        .devices
        .iter()
        .map(|d| {
            let is_current = app.current_device_id.as_deref() == Some(d.id.to_string().as_str());
            let label = if is_current {
                format!("{} {}", d.name, t!("tui.label.current"))
            } else {
                d.name.clone()
            };
            ListItem::new(label)
        })
        .collect();

    let has_items = !items.is_empty();
    let mut state = ListState::default();
    if has_items {
        state.select(Some(app.selected_device));
    }

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(t!("tui.title.devices")),
        )
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        )
        .highlight_symbol("> ");
    frame.render_stateful_widget(list, chunks[0], &mut state);

    let detail = if let Some(d) = app.devices.get(app.selected_device) {
        format!(
            "{}: {}\n\n{}: {}\n\n{}: {}",
            t!("tui.label.name"),
            d.name,
            t!("tui.label.deviceId"),
            d.id,
            t!("tui.label.lastSeen"),
            d.last_seen,
        )
    } else {
        String::new()
    };
    let detail_widget = Paragraph::new(detail)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(t!("tui.title.deviceInfo")),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(detail_widget, chunks[1]);
}
