pub mod auth;
pub mod board_shares;
pub mod boards;
pub mod devices;
pub mod export;
pub mod health;
pub mod identity;
pub mod invites;
pub mod link;
pub mod notes;
pub mod register;
pub mod shares;
pub mod spa;
pub mod ws;

use crate::state::AppState;
use axum::{
    routing::{delete, get, patch, post},
    Router,
};

pub fn build(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health::health))
        .route("/register", post(register::register))
        .route(
            "/invites",
            get(invites::list_invites).post(invites::create_invite),
        )
        .route("/invites/:token", delete(invites::revoke_invite))
        .route("/auth/register", post(auth::register_identity))
        .route("/auth/device", post(auth::register_device))
        .route("/link/init", post(link::init_link))
        .route("/link/:token", get(link::get_link))
        .route("/link/confirm", post(link::confirm_link))
        .route("/link/status/:token", get(link::link_status))
        .route(
            "/identity/me",
            get(identity::get_me).patch(identity::update_me),
        )
        .route("/identity/contacts", get(identity::get_recent_contacts))
        .route("/identity/lookup/:name", get(identity::lookup_by_name))
        .route("/notes", get(notes::list_notes).post(notes::create_note))
        .route("/notes/shared", get(shares::get_shared_with_me))
        .route(
            "/notes/:id",
            get(notes::get_note)
                .delete(notes::delete_note)
                .patch(notes::patch_note),
        )
        .route("/notes/:id/blob", get(notes::get_blob).put(notes::put_blob))
        .route(
            "/notes/:id/shares",
            get(shares::list_shares).post(shares::share_note),
        )
        .route(
            "/notes/:id/shares/:identity_id",
            delete(shares::delete_share),
        )
        .route(
            "/boards",
            get(boards::list_boards).post(boards::create_board),
        )
        .route(
            "/boards/shared",
            get(board_shares::get_boards_shared_with_me),
        )
        .route(
            "/boards/:id",
            patch(boards::patch_board).delete(boards::delete_board),
        )
        .route("/boards/:id/reorder", patch(boards::reorder_board))
        .route(
            "/boards/:id/shares",
            get(board_shares::list_board_shares).post(board_shares::share_board),
        )
        .route(
            "/boards/:id/shares/:identity_id",
            delete(board_shares::revoke_board_share),
        )
        .route("/export", get(export::export_data))
        .route("/devices", get(devices::list_devices))
        .route("/devices/:id", delete(devices::delete_device))
        .route("/devices/:id/rename", post(devices::rename_device))
        .route("/ws", get(ws::ws_handler))
        .fallback(spa::spa_handler)
        .with_state(state)
}
