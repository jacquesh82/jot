pub mod auth;
pub mod boards;
pub mod devices;
pub mod health;
pub mod link;
pub mod notes;
pub mod ws;

use crate::state::AppState;
use axum::{
    routing::{get, post},
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
        .with_state(state)
}
