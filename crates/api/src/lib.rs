pub mod auth;
pub mod error;
pub mod openapi;
pub mod routes;
pub mod state;

#[cfg(test)]
pub mod test_helpers;
#[cfg(test)]
mod tests;

use axum::Router;
pub use error::ApiError;
pub use openapi::ApiDoc;
pub use state::AppState;

pub fn build_router(state: AppState) -> Router {
    // jsonwebtoken 10 selects its crypto backend lazily on first use.
    // Install rust_crypto explicitly so it wins even if aws_lc_rs is
    // activated transitively by another dependency in the binary.
    let _ = jsonwebtoken::crypto::rust_crypto::DEFAULT_PROVIDER.install_default();
    routes::build(state)
}
