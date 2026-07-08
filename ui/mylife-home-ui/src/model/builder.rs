use std::{collections::HashMap, sync::Arc};

use bytes::Bytes;
use serde_json::Value;
use thiserror::Error;
use web_api::model as api;

use crate::model::{RequiredComponentState, Resource, definition};

#[derive(Debug, Error)]
pub enum ModelBuildError {
    #[error("could not decode resource '{resource_id}': {error}")]
    ResourceDecodeError {
        resource_id: String,
        #[source]
        error: base64::DecodeError,
    },

    #[error("got reference to non existing resource '{0}'")]
    ResourceNotFound(String),

    #[error("could not serialize model")]
    ModelSerializationError(#[source] serde_json::Error),
}

/// Handle transient state while builder a new model from definition
#[derive(Debug, Default)]
pub struct ModelBuilder {
    pub model_hash: String,
    pub resources: HashMap<String, Resource>,
    pub required_component_states: Vec<RequiredComponentState>,
    resource_translation: HashMap<String, String>,
}

impl ModelBuilder {
    pub fn build(&mut self, definition: definition::Definition) -> Result<(), ModelBuildError> {
        for definition::DefinitionResource { id, mime, data } in definition.resources {
            // STANDARD = javascript base64
            use base64::{Engine, engine::general_purpose::STANDARD};

            let data = Bytes::from_owner(STANDARD.decode(data).map_err(|error| {
                ModelBuildError::ResourceDecodeError {
                    resource_id: id.clone(),
                    error,
                }
            })?);

            let len = data.len();
            let hash = self.set_resource(mime, data);
            self.resource_translation.insert(id.clone(), hash.clone());

            tracing::debug!(id, hash, len, "creating resource");
        }

        // serialize styles as a resource and get the hash
        let style_hash = {
            let data = Bytes::from_owner(Self::create_css(definition.styles));
            let len = data.len();
            let hash = self.set_resource("text/css", data);
            tracing::debug!(hash, len, "creating css");
            hash
        };

        let windows = definition
            .windows
            .into_iter()
            .map(|window| self.translate_window(window))
            .collect::<Result<Vec<_>, _>>()?;

        let model = api::Model {
            windows,
            default_window: api::DefaultWindow(definition.default_window),
            style_hash,
        };

        // serialize the model as a resource and get the hash
        let data = Bytes::from_owner(
            serde_json::to_vec(&model).map_err(ModelBuildError::ModelSerializationError)?,
        );
        let len = data.len();
        self.model_hash = self.set_resource("application/json", data);
        tracing::debug!(hash = self.model_hash, len, "creating resource from model");

        Ok(())
    }

    fn set_resource(&mut self, mime: impl Into<String>, data: Bytes) -> String {
        let hash = Self::compute_hash(&data);
        self.resources.insert(
            hash.clone(),
            Resource {
                mime: Arc::new(mime.into()),
                data: Arc::new(data),
            },
        );
        hash
    }

    fn compute_hash(data: &[u8]) -> String {
        // URL_SAFE_NO_PAD = javascript base64url
        use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

        let digest = md5::compute(data);
        URL_SAFE_NO_PAD.encode(digest.0)
    }

    fn translate_window(&self, window: definition::Window) -> Result<api::Window, ModelBuildError> {
        Ok(api::Window {
            id: window.id,
            style: self.translate_style(&window.style),
            height: window.height,
            width: window.width,
            background_resource: self.translate_resource(window.background_resource)?,
            controls: window
                .controls
                .into_iter()
                .map(|control| self.translate_control(control))
                .collect::<Result<Vec<_>, _>>()?,
        })
    }

    fn translate_control(
        &self,
        control: definition::Control,
    ) -> Result<api::Control, ModelBuildError> {
        let display = if let Some(display) = control.display {
            Some(api::ControlDisplay {
                component_id: display.component_id,
                component_state: display.component_state,
                default_resource: self.translate_resource(display.default_resource)?,
                map: display
                    .map
                    .into_iter()
                    .map(|item| {
                        Ok(api::ControlDisplayMapItem {
                            min: item.min,
                            max: item.max,
                            value: item.value,
                            resource: self.translate_resource(item.resource)?,
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            })
        } else {
            None
        };

        Ok(api::Control {
            id: control.id,
            style: self.translate_style(&control.style),
            height: control.height,
            width: control.width,
            x: control.x,
            y: control.y,
            display: display,
            text: control.text,
            primary_action: control.primary_action,
            secondary_action: control.secondary_action,
        })
    }

    fn translate_resource(
        &self,
        resource: Option<definition::Resource>,
    ) -> Result<Option<api::Resource>, ModelBuildError> {
        let Some(id) = resource else {
            return Ok(None);
        };

        // TODO: we should be more strict here
        // a resource should be either set or null, but not empty
        if id == "" {
            return Ok(None);
        }

        let Some(hash) = self.resource_translation.get(&id) else {
            return Err(ModelBuildError::ResourceNotFound(id));
        };

        Ok(Some(api::Resource(hash.clone())))
    }

    fn translate_style(&self, style: &definition::Style) -> api::Style {
        api::Style(style.iter().map(|id| format!("user-{}", id)).collect())
    }

    /// Generates CSS rules from a list of style definitions.
    ///
    /// Each style becomes a CSS class named `.user-{id}`, with its properties
    /// translated into standard CSS declarations. Rules are separated by a
    /// blank line.
    ///
    /// # Example
    ///
    /// Input styles:
    /// ```json
    /// [
    ///   { "id": "title-window", "properties": { "fontSize": "72px", "fontWeight": "bold" } }
    /// ]
    /// ```
    ///
    /// Produces:
    /// ```css
    /// .user-title-window {
    ///   font-size: 72px;
    ///   font-weight: bold;
    /// }
    ///
    /// TODO: review style layout: could be directly css key and css values as string directly
    /// ```
    pub fn create_css(styles: Vec<definition::DefinitionStyle>) -> String {
        let mut css_rules = Vec::with_capacity(styles.len());

        for style in styles {
            // Prefix avoids collisions with any predefined/built-in class names.
            let class_name = format!(".user-{}", style.id);

            // Sort keys for stable, readable, diffable output.
            let mut entries: Vec<_> = style.properties.into_iter().collect();
            entries.sort_by(|(a, _), (b, _)| a.cmp(b));

            let properties: Vec<String> = entries
                .into_iter()
                .map(|(key, value)| {
                    let css_key = Self::format_css_key(&key);
                    let css_value = Self::format_css_value(value);
                    format!("  {css_key}: {css_value};")
                })
                .collect();

            let rule = format!("{class_name} {{\n{}\n}}", properties.join("\n"));
            css_rules.push(rule);
        }

        css_rules.join("\n\n")
    }

    fn format_css_value(value: Value) -> String {
        match value {
            Value::String(s) => s,
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    i.to_string()
                } else if let Some(f) = n.as_f64() {
                    format!("{f}")
                } else {
                    n.to_string()
                }
            }
            Value::Bool(b) => b.to_string(),
            other => other.to_string(),
        }
    }

    fn format_css_key(input: &str) -> String {
        let mut result = String::with_capacity(input.len() + 4);

        for (i, c) in input.chars().enumerate() {
            if i > 0 && c.is_ascii_uppercase() {
                result.push('-');
            }
            result.extend(c.to_lowercase());
        }

        result
    }
}
