pub mod auth;
pub mod error;
pub mod routes;
pub mod state;

#[cfg(test)]
pub mod test_helpers;

use axum::Router;
pub use error::ApiError;
pub use state::AppState;

pub fn build_router(state: AppState) -> Router {
    routes::build(state)
}
