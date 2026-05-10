use crate::tui::app::{App, Focus, Mode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(frame.area());

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(chunks[0]);

    render_boards(frame, app, main_chunks[0]);
    render_notes(frame, app, main_chunks[1]);
    render_status(frame, app, chunks[1]);
}

fn render_boards(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == Focus::Boards;
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let items: Vec<ListItem> = app
        .boards
        .iter()
        .map(|b| ListItem::new(b.name.clone()))
        .collect();

    let mut state = ListState::default();
    if !app.boards.is_empty() {
        state.select(Some(app.selected_board));
    }

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Boards")
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

    let items: Vec<ListItem> = app
        .notes
        .iter()
        .map(|n| ListItem::new(format!("[{}] {}", n.note_type, n.id)))
        .collect();

    let mut state = ListState::default();
    if !app.notes.is_empty() {
        state.select(Some(app.selected_note));
    }

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Notes")
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

fn render_status(frame: &mut Frame, app: &App, area: Rect) {
    let text = match &app.mode {
        Mode::Normal => Line::from(vec![Span::raw(
            "[q]uit  [n]ew  [d]elete  [Tab]switch  [r]efresh",
        )]),
        Mode::Input(buf) => Line::from(vec![
            Span::styled("New note: ", Style::default().fg(Color::Green)),
            Span::raw(buf.as_str()),
            Span::styled("\u{258c}", Style::default().fg(Color::Green)),
        ]),
        Mode::Confirm(_) => Line::from(vec![Span::styled(
            "Delete note? [y]es / [n]o",
            Style::default().fg(Color::Red),
        )]),
    };

    frame.render_widget(Paragraph::new(text), area);
}
