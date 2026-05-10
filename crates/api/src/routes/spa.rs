use axum::{
    http::{header::CONTENT_TYPE, Uri},
    response::IntoResponse,
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/../../spa/dist"]
struct Assets;

pub async fn spa_handler(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            (
                [(CONTENT_TYPE, mime.as_ref().to_owned())],
                content.data.into_owned(),
            )
                .into_response()
        }
        None => {
            let index = Assets::get("index.html")
                .expect("spa/dist/index.html not found — run scripts/build-spa.sh");
            (
                [(CONTENT_TYPE, "text/html".to_owned())],
                index.data.into_owned(),
            )
                .into_response()
        }
    }
}
