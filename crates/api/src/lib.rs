pub mod auth;
pub mod error;
pub mod routes;
pub mod state;

use axum::Router;
pub use error::ApiError;
pub use state::AppState;

pub fn build_router(state: AppState) -> Router {
    routes::build(state)
}
