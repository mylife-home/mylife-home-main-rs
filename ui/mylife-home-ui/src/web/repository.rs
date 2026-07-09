use std::collections::HashMap;

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use common::components::{
    metadata::{MemberType, PluginUsage, Type},
    types::Value,
};
use thiserror::Error;

use super::{AppState, WebError};

pub fn router() -> Router<AppState> {
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
    let ids = state
        .registry
        .get_component_ids()
        .await?
        .into_iter()
        .map(|id| id.as_ref().clone())
        .collect();
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
