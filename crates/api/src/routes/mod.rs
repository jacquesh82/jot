pub mod auth;
pub mod boards;
pub mod devices;
pub mod health;
pub mod link;
pub mod notes;
pub mod ws;

use crate::state::AppState;
use axum::{
    routing::{get, patch, post},
    Router,
};

pub fn build(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health::health))
        .route("/auth/register", post(auth::register_identity))
        .route("/auth/device", post(auth::register_device))
        .route("/link/init", post(link::init_link))
        .route("/link/:token", get(link::get_link))
        .route("/link/confirm", post(link::confirm_link))
        .route("/link/status/:token", get(link::link_status))
        .route("/notes", get(notes::list_notes).post(notes::create_note))
        .route(
            "/notes/:id",
            get(notes::get_note)
                .delete(notes::delete_note)
                .patch(notes::patch_note),
        )
        .route("/notes/:id/blob", get(notes::get_blob).put(notes::put_blob))
        .route(
            "/boards",
            get(boards::list_boards).post(boards::create_board),
        )
        .route(
            "/boards/:id",
            patch(boards::patch_board).delete(boards::delete_board),
        )
        .route("/boards/:id/reorder", patch(boards::reorder_board))
        .route("/devices", get(devices::list_devices))
        .route(
            "/devices/:id",
            axum::routing::delete(devices::delete_device),
        )
        .route("/devices/:id/rename", post(devices::rename_device))
        .route("/ws", get(ws::ws_handler))
        .with_state(state)
}
