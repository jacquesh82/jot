use crate::client::{
    BoardSummary, DeviceSummary, IdentityInfo, JotClient, NoteSummary, SharedBoardSummary,
    SharedNoteSummary,
};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum Focus {
    Boards,
    Notes,
}

#[derive(Debug, Clone, PartialEq)]
pub enum View {
    MyBoards,
    SharedBoards,
    SharedNotes,
    Profile,
    Stats,
    Devices,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputContext {
    NewNote,
    NewBoard,
    RenameBoard(Uuid),
    RenameDevice(Uuid),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConfirmAction {
    DeleteNote(Uuid),
    DeleteBoard(Uuid),
    DeleteDevice(Uuid),
    DeleteAccount,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Normal,
    Input(InputContext, String),
    Confirm(ConfirmAction),
}

pub struct App {
    pub boards: Vec<BoardSummary>,
    pub selected_board: usize,
    pub notes: Vec<NoteSummary>,
    pub selected_note: usize,
    pub focus: Focus,
    pub view: View,
    pub mode: Mode,
    pub status: String,
    pub client: JotClient,
    pub note_content: Option<String>,
    pub content_scroll: u16,
    pub loading_content: bool,
    pub should_quit: bool,
    pub pending_edit: Option<Uuid>,
    pub shared_boards: Vec<SharedBoardSummary>,
    pub selected_shared_board: usize,
    pub shared_notes: Vec<SharedNoteSummary>,
    pub selected_shared_note: usize,
    // Profile / Stats / Devices views
    pub identity_info: Option<IdentityInfo>,
    pub devices: Vec<DeviceSummary>,
    pub selected_device: usize,
    pub stats: Vec<(String, usize)>,
    #[allow(dead_code)]
    pub selected_stat: usize,
    pub server_url: String,
    pub current_device_id: Option<String>,
}

impl App {
    pub fn new(client: JotClient) -> Self {
        let cfg = crate::config::Config::load();
        Self {
            boards: vec![],
            selected_board: 0,
            notes: vec![],
            selected_note: 0,
            focus: Focus::Boards,
            view: View::MyBoards,
            mode: Mode::Normal,
            status: String::new(),
            client,
            note_content: None,
            content_scroll: 0,
            loading_content: false,
            should_quit: false,
            pending_edit: None,
            shared_boards: vec![],
            selected_shared_board: 0,
            shared_notes: vec![],
            selected_shared_note: 0,
            identity_info: None,
            devices: vec![],
            selected_device: 0,
            stats: vec![],
            selected_stat: 0,
            server_url: cfg.server_url().to_string(),
            current_device_id: cfg.device_id.clone(),
        }
    }

    pub fn set_note_content(&mut self, text: Option<String>) {
        self.note_content = text;
        self.content_scroll = 0;
        self.loading_content = false;
    }

    pub fn board_down(&mut self) {
        match self.view {
            View::MyBoards => {
                if !self.boards.is_empty() {
                    self.selected_board = (self.selected_board + 1).min(self.boards.len() - 1);
                }
            }
            View::SharedBoards => {
                if !self.shared_boards.is_empty() {
                    self.selected_shared_board =
                        (self.selected_shared_board + 1).min(self.shared_boards.len() - 1);
                }
            }
            View::SharedNotes => {
                if !self.shared_notes.is_empty() {
                    self.selected_shared_note =
                        (self.selected_shared_note + 1).min(self.shared_notes.len() - 1);
                }
            }
            View::Profile | View::Stats | View::Devices => {}
        }
    }

    pub fn board_up(&mut self) {
        match self.view {
            View::MyBoards => {
                if self.selected_board > 0 {
                    self.selected_board -= 1;
                }
            }
            View::SharedBoards => {
                if self.selected_shared_board > 0 {
                    self.selected_shared_board -= 1;
                }
            }
            View::SharedNotes => {
                if self.selected_shared_note > 0 {
                    self.selected_shared_note -= 1;
                }
            }
            View::Profile | View::Stats | View::Devices => {}
        }
    }

    pub fn device_down(&mut self) {
        if !self.devices.is_empty() {
            self.selected_device = (self.selected_device + 1).min(self.devices.len() - 1);
        }
    }

    pub fn device_up(&mut self) {
        if self.selected_device > 0 {
            self.selected_device -= 1;
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

    pub fn cycle_view(&mut self) {
        self.view = match self.view {
            View::MyBoards => View::SharedBoards,
            View::SharedBoards => View::SharedNotes,
            View::SharedNotes | View::Profile | View::Stats | View::Devices => View::MyBoards,
        };
        self.notes.clear();
        self.selected_note = 0;
        self.set_note_content(None);
    }

    pub fn start_input(&mut self) {
        self.mode = Mode::Input(InputContext::NewNote, String::new());
    }

    pub fn start_input_board(&mut self) {
        self.mode = Mode::Input(InputContext::NewBoard, String::new());
    }

    pub fn start_rename_board(&mut self, id: Uuid, current_name: &str) {
        self.mode = Mode::Input(InputContext::RenameBoard(id), current_name.to_string());
    }

    pub fn cancel_mode(&mut self) {
        self.mode = Mode::Normal;
    }

    pub fn input_push(&mut self, c: char) {
        if let Mode::Input(_, ref mut buf) = self.mode {
            buf.push(c);
        }
    }

    pub fn input_pop(&mut self) {
        if let Mode::Input(_, ref mut buf) = self.mode {
            buf.pop();
        }
    }

    pub fn current_board_id(&self) -> Option<Uuid> {
        match self.view {
            View::MyBoards => self.boards.get(self.selected_board).map(|b| b.id),
            View::SharedBoards => self
                .shared_boards
                .get(self.selected_shared_board)
                .map(|b| b.board_id),
            View::SharedNotes | View::Profile | View::Stats | View::Devices => None,
        }
    }

    pub fn current_note_id(&self) -> Option<Uuid> {
        match self.view {
            View::SharedNotes => self
                .shared_notes
                .get(self.selected_shared_note)
                .map(|n| n.note_id),
            View::MyBoards | View::SharedBoards => self.notes.get(self.selected_note).map(|n| n.id),
            View::Profile | View::Stats | View::Devices => None,
        }
    }

    pub fn current_device_id_val(&self) -> Option<Uuid> {
        self.devices.get(self.selected_device).map(|d| d.id)
    }

    pub fn start_delete_device(&mut self) {
        if let Some(id) = self.current_device_id_val() {
            self.mode = Mode::Confirm(ConfirmAction::DeleteDevice(id));
        }
    }

    pub fn start_rename_device(&mut self) {
        let info = self
            .devices
            .get(self.selected_device)
            .map(|d| (d.id, d.name.clone()));
        if let Some((id, name)) = info {
            self.mode = Mode::Input(InputContext::RenameDevice(id), name);
        }
    }

    pub fn start_delete(&mut self) {
        if let Some(id) = self.current_note_id() {
            self.mode = Mode::Confirm(ConfirmAction::DeleteNote(id));
        }
    }

    pub fn start_delete_board(&mut self) {
        if self.focus == Focus::Boards {
            if let Some(id) = self.current_board_id() {
                self.mode = Mode::Confirm(ConfirmAction::DeleteBoard(id));
            }
        }
    }

    pub fn start_delete_account(&mut self) {
        self.mode = Mode::Confirm(ConfirmAction::DeleteAccount);
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
        assert!(matches!(app.mode, Mode::Input(InputContext::NewNote, _)));
        app.input_push('h');
        app.input_push('i');
        if let Mode::Input(_, ref buf) = app.mode {
            assert_eq!(buf, "hi");
        }
        app.input_pop();
        if let Mode::Input(_, ref buf) = app.mode {
            assert_eq!(buf, "h");
        }
        app.cancel_mode();
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn mode_transition_new_board() {
        let mut app = make_app();
        app.start_input_board();
        assert!(matches!(app.mode, Mode::Input(InputContext::NewBoard, _)));
        app.cancel_mode();
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn mode_transition_rename_board() {
        let mut app = make_app();
        let id = Uuid::new_v4();
        app.start_rename_board(id, "Old Name");
        assert!(matches!(
            app.mode,
            Mode::Input(InputContext::RenameBoard(_), _)
        ));
        if let Mode::Input(InputContext::RenameBoard(bid), ref buf) = app.mode {
            assert_eq!(bid, id);
            assert_eq!(buf, "Old Name");
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
            snippet: None,
        }];
        app.start_delete();
        assert_eq!(app.mode, Mode::Confirm(ConfirmAction::DeleteNote(nid)));
        app.cancel_mode();
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn mode_transition_delete_board_confirm() {
        let mut app = make_app();
        let bid = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        app.boards = vec![BoardSummary {
            id: bid,
            name: "B".to_string(),
            position: 0,
        }];
        app.focus = Focus::Boards;
        app.start_delete_board();
        assert_eq!(app.mode, Mode::Confirm(ConfirmAction::DeleteBoard(bid)));
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

    #[test]
    fn cycle_view() {
        let mut app = make_app();
        assert_eq!(app.view, View::MyBoards);
        app.cycle_view();
        assert_eq!(app.view, View::SharedBoards);
        app.cycle_view();
        assert_eq!(app.view, View::SharedNotes);
        app.cycle_view();
        assert_eq!(app.view, View::MyBoards);
    }
}
