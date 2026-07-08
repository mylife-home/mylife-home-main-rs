use std::{collections::HashMap};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Response},
    routing::get,
};
use common::{
    components::{
        metadata::{MemberType, PluginUsage, Type},
        registry::RegistryHandle,
        types::Value,
    },
    utils::{actors::HandleLookupError, config},
};
use rust_embed::Embed;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::oneshot;
use tokio::{io, net::TcpListener};

use crate::model::ModelHandle;

#[derive(Debug, Deserialize)]
struct WebConfig {
    listen_address: String,
}

#[derive(Debug, Clone)]
struct AppState {
    registry: RegistryHandle,
    model: ModelHandle,
}

#[derive(Debug)]
pub struct WebServer {
    shutdown: Option<oneshot::Sender<()>>,
    task: tokio::task::JoinHandle<()>,
}

#[derive(Debug, Error)]
pub enum WebServerError {
    #[error("failed to lookup actor handle: {0}")]
    HandleLookupError(#[from] HandleLookupError),
    #[error("bind error: {0}")]
    BindError(#[source] io::Error),
}

impl WebServer {
    pub async fn new() -> Result<Self, WebServerError> {
        let config: WebConfig = config::section("web");
        let state = AppState {
            registry: RegistryHandle::new()?,
            model: ModelHandle::new()?,
        };

        let app = Router::new()
            .nest("/repository", repository_router())
            .nest("/resources", resources_router())
            .merge(setup_static())
            .with_state(state);

        let listener = TcpListener::bind(config.listen_address)
            .await
            .map_err(WebServerError::BindError)?;

        let (tx, rx) = oneshot::channel();
        let task = tokio::spawn(async move {
            let server = axum::serve(listener, app).with_graceful_shutdown(async {
                let _ = rx.await;
            });

            if let Err(error) = server.await {
                tracing::error!(?error, "web server error");
            }
        });

        Ok(Self {
            shutdown: Some(tx),
            task,
        })
    }

    pub async fn terminate(mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }

        if let Err(error) = self.task.await {
            tracing::error!(?error, "could not join web server task");
        }
    }
}

#[derive(Embed)]
#[folder = "../web-app/dist/"] // relative to crate root
struct StaticContent;

fn setup_static() -> Router<AppState> {
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

// ----- repository routes -----------------------------------------------------

fn repository_router() -> Router<AppState> {
    Router::new()
        .route("/action/{component_id}/{action_name}", get(action))
        .route("/components", get(components))
        .route("/state/{component_id}", get(state))
}

#[derive(Debug, Error)]
enum ComponentActionError {
    #[error("component is not a UI component")]
    NotUi,
    #[error("action not found")]
    ActionNotFound,
    #[error("action type must be boolean")]
    ActionNotBool,
}

async fn action(
    State(state): State<AppState>,
    Path((component_id, action_name)): Path<(String, String)>,
) -> Result<(), WebError> {
    let info = state.registry.get_component(component_id.clone()).await?;
    if info.plugin.usage() != PluginUsage::Ui {
        return Err(ComponentActionError::NotUi.into());
    }

    let Some(member) = info.plugin.members().get(&action_name) else {
        return Err(ComponentActionError::ActionNotFound.into());
    };

    if member.member_type() != MemberType::Action {
        return Err(ComponentActionError::ActionNotFound.into());
    }

    if !matches!(member.value_type(), Type::Bool) {
        return Err(ComponentActionError::ActionNotBool.into());
    }

    // execute true then false
    state.registry.component_execute_action(
        component_id.clone(),
        action_name.clone(),
        Value::Bool(true),
    );
    state.registry.component_execute_action(
        component_id.clone(),
        action_name.clone(),
        Value::Bool(false),
    );

    tracing::debug!(component_id, action_name, "ran ui action on component");

    Ok(())
}

async fn components(State(state): State<AppState>) -> Result<Json<Vec<String>>, WebError> {
    let ids = state.registry.get_component_ids().await?.into_iter().map(|id| id.as_ref().clone()).collect();
    Ok(Json(ids))
}

async fn state(
    State(state): State<AppState>,
    Path(component_id): Path<String>,
) -> Result<Json<HashMap<String, serde_json::Value>>, WebError> {
    let info = state.registry.get_component(component_id).await?;

    let state = HashMap::from_iter(info.state.into_iter().map(|(key, value)| {
        (
            key,
            match value {
                None => serde_json::Value::Null,
                Some(Value::Range(value)) => serde_json::Value::Number(value.into()),
                Some(Value::Text(value)) => serde_json::Value::String(value),
                Some(Value::Float(value)) => serde_json::Value::Number(
                    serde_json::Number::from_f64(value).expect("could not translate number"),
                ),
                Some(Value::Bool(value)) => serde_json::Value::Bool(value),
                Some(Value::Enum(value)) => serde_json::Value::String(value),
                Some(Value::Complex) => panic!("complex unsupported"),
            },
        )
    }));

    Ok(Json(state))
}

// ----- resource routes -------------------------------------------------------

fn resources_router() -> Router<AppState> {
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

#[derive(Debug, Serialize)]
pub struct WebError {
    error: String,
}

impl<E: std::error::Error> From<E> for WebError {
    fn from(value: E) -> Self {
        WebError {
            error: format!("{}", value),
        }
    }
}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(self)).into_response()
    }
}
