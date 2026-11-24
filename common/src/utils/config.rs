use std::collections::HashMap;
use std::fmt::Display;
use std::str::FromStr;
use std::sync::OnceLock;

use regex::Regex;
use serde::de::{self, Deserialize, DeserializeOwned, Deserializer};

static CONFIG: OnceLock<HashMap<String, toml::Value>> = OnceLock::new();

/// Loads, parses and stores the config globally. Call once at startup. Panics on failure.
pub fn init(path: &str) {
    let raw = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("could not read config '{}': {}", path, e));
    let expanded = expand_env(&raw);
    let sections: HashMap<String, toml::Value> = toml::from_str(&expanded)
        .unwrap_or_else(|e| panic!("could not parse config '{}': {}", path, e));
    CONFIG.set(sections).expect("config already initialized");
}

/// Reads a section, deserialized into the caller's type. Panics if absent or malformed.
pub fn section<T: DeserializeOwned>(name: &str) -> T {
    let value = sections()
        .get(name)
        .unwrap_or_else(|| panic!("missing config section '{}'", name));
    value
        .clone()
        .try_into()
        .unwrap_or_else(|e| panic!("invalid config section '{}': {}", name, e))
}

/// Reads an optional setting, taking an empty value for an absent one.
///
/// A setting left to the environment expands to nothing when its variable is
/// unset, so a config written that way carries empty strings for everything
/// nobody set. Use it with `#[serde(default, deserialize_with = ...)]`.
pub fn empty_as_none<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    T::Err: Display,
{
    let Some(raw) = Option::<String>::deserialize(deserializer)? else {
        return Ok(None);
    };

    if raw.is_empty() {
        return Ok(None);
    }

    raw.parse().map(Some).map_err(de::Error::custom)
}

fn sections() -> &'static HashMap<String, toml::Value> {
    CONFIG.get().expect("config not initialized")
}

/// Expands `%{VAR}` and `%{VAR|default}` from the environment before parsing.
fn expand_env(raw: &str) -> String {
    let re = Regex::new(r"%\{([A-Za-z_][A-Za-z0-9_]*)(?:\|([^}]*))?\}").unwrap();
    re.replace_all(raw, |caps: &regex::Captures| {
        let var = &caps[1];
        let default = caps.get(2).map_or("", |m| m.as_str());
        std::env::var(var).unwrap_or_else(|_| default.to_owned())
    })
    .into_owned()
}
