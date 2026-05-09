pub mod auth;
pub mod boards;
pub mod devices;
pub mod health;
pub mod link;
pub mod notes;
pub mod ws;

use axum::Router;
use crate::state::AppState;

pub fn build(_state: AppState) -> Router {
    Router::new()
}
