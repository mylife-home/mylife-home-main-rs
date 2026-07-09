use axum::{
    Router,
    extract::{Path, State},
    http::header,
    response::{IntoResponse, Response},
    routing::get,
};

use super::{AppState, WebError};

pub fn router() -> Router<AppState> {
    // wildcard capture: /resources/<hash...>
    Router::new().route("/{*hash}", get(resource))
}

async fn resource(
    State(state): State<AppState>,
    Path(hash): Path<String>,
) -> Result<Response, WebError> {
    let res = state.model.get_resource(&hash).await?;

    let headers = [
        (header::CONTENT_TYPE, res.mime()),
        (
            header::CACHE_CONTROL,
            "public, max-age=31557600, s-maxage=31557600", // 1 year
        ),
    ];

    Ok((headers, res.data().clone()).into_response())
}
