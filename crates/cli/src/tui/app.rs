use crate::client::{BoardSummary, JotClient, NoteSummary};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum Focus {
    Boards,
    Notes,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConfirmAction {
    DeleteNote(Uuid),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Normal,
    Input(String),
    Confirm(ConfirmAction),
}

pub struct App {
    pub boards: Vec<BoardSummary>,
    pub selected_board: usize,
    pub notes: Vec<NoteSummary>,
    pub selected_note: usize,
    pub focus: Focus,
    pub mode: Mode,
    pub status: String,
    pub client: JotClient,
}

impl App {
    pub fn new(client: JotClient) -> Self {
        Self {
            boards: vec![],
            selected_board: 0,
            notes: vec![],
            selected_note: 0,
            focus: Focus::Boards,
            mode: Mode::Normal,
            status: String::new(),
            client,
        }
    }

    pub fn board_down(&mut self) {
        if !self.boards.is_empty() {
            self.selected_board = (self.selected_board + 1).min(self.boards.len() - 1);
        }
    }

    pub fn board_up(&mut self) {
        if self.selected_board > 0 {
            self.selected_board -= 1;
        }
    }

    pub fn note_down(&mut self) {
        if !self.notes.is_empty() {
            self.selected_note = (self.selected_note + 1).min(self.notes.len() - 1);
        }
    }

    pub fn note_up(&mut self) {
        if self.selected_note > 0 {
            self.selected_note -= 1;
        }
    }

    pub fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Boards => Focus::Notes,
            Focus::Notes => Focus::Boards,
        };
    }

    pub fn start_input(&mut self) {
        self.mode = Mode::Input(String::new());
    }

    pub fn cancel_mode(&mut self) {
        self.mode = Mode::Normal;
    }

    pub fn input_push(&mut self, c: char) {
        if let Mode::Input(ref mut buf) = self.mode {
            buf.push(c);
        }
    }

    pub fn input_pop(&mut self) {
        if let Mode::Input(ref mut buf) = self.mode {
            buf.pop();
        }
    }

    pub fn current_board_id(&self) -> Option<Uuid> {
        self.boards.get(self.selected_board).map(|b| b.id)
    }

    pub fn current_note_id(&self) -> Option<Uuid> {
        self.notes.get(self.selected_note).map(|n| n.id)
    }

    pub fn start_delete(&mut self) {
        if let Some(id) = self.current_note_id() {
            self.mode = Mode::Confirm(ConfirmAction::DeleteNote(id));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn make_app() -> App {
        App::new(JotClient::new(&Config::default()))
    }

    fn board(id: &str) -> BoardSummary {
        BoardSummary {
            id: Uuid::parse_str(id).unwrap_or_else(|_| Uuid::new_v4()),
            name: "Board".to_string(),
            position: 0,
        }
    }

    #[test]
    fn navigation_board_down_up() {
        let mut app = make_app();
        app.boards = vec![
            board("00000000-0000-0000-0000-000000000001"),
            board("00000000-0000-0000-0000-000000000002"),
        ];
        assert_eq!(app.selected_board, 0);
        app.board_down();
        assert_eq!(app.selected_board, 1);
        app.board_down(); // clamp at last
        assert_eq!(app.selected_board, 1);
        app.board_up();
        assert_eq!(app.selected_board, 0);
        app.board_up(); // clamp at 0
        assert_eq!(app.selected_board, 0);
    }

    #[test]
    fn mode_transition_normal_input_cancel() {
        let mut app = make_app();
        assert_eq!(app.mode, Mode::Normal);
        app.start_input();
        assert!(matches!(app.mode, Mode::Input(_)));
        app.input_push('h');
        app.input_push('i');
        if let Mode::Input(ref buf) = app.mode {
            assert_eq!(buf, "hi");
        }
        app.input_pop();
        if let Mode::Input(ref buf) = app.mode {
            assert_eq!(buf, "h");
        }
        app.cancel_mode();
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn mode_transition_delete_confirm() {
        let mut app = make_app();
        let nid = Uuid::new_v4();
        app.notes = vec![NoteSummary {
            id: nid,
            note_type: "text".to_string(),
            blob_key: "k".to_string(),
            color: "#FFF".to_string(),
            position: 0,
        }];
        app.start_delete();
        assert_eq!(app.mode, Mode::Confirm(ConfirmAction::DeleteNote(nid)));
        app.cancel_mode();
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn toggle_focus() {
        let mut app = make_app();
        assert_eq!(app.focus, Focus::Boards);
        app.toggle_focus();
        assert_eq!(app.focus, Focus::Notes);
        app.toggle_focus();
        assert_eq!(app.focus, Focus::Boards);
    }
}
