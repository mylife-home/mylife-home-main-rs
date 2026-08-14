use axum::{
    Router,
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Response},
};
use rust_embed::Embed;

use super::AppState;
#[derive(Embed)]
#[folder = "../web-app/dist/"] // relative to crate root
struct StaticContent;

pub fn router() -> Router<AppState> {
    for path in StaticContent::iter() {
        tracing::debug!(path = path.as_ref(), "serving static file");
    }

    Router::new().fallback(static_handler)
}

async fn static_handler(uri: Uri) -> Response {
    // strip leading '/', map '' to index.html (root request)
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    match StaticContent::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            (
                [(header::CONTENT_TYPE, mime.as_ref())],
                content.data, // Cow<'static, [u8]>
            )
                .into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
