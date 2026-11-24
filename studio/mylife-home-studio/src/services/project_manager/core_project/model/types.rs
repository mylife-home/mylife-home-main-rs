use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use studio_web_api::component_model::{ConfigItem, ConfigType, Member, MemberType, PluginUsage};

use super::Collection;
use super::ids::{BindingId, ComponentId, ExportId, InstanceId, PluginId, TemplateId};

use super::error::CoreModelError;
use super::ids::IdKind;

/// Whether a plugin is offered in the designer's toolbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolboxDisplay {
    Show,
    Hide,
}

/// What a component is an instance of.
///
/// On the wire this is `{ type: "plugin" | "template", id: String }`, and the
/// id means a different thing in each case. As an enum the id is typed with it,
/// so a template id cannot be read as a plugin id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComponentDefinition {
    Plugin(PluginId),
    Template(TemplateId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    pub fn moved_by(self, delta: Position) -> Self {
        Self {
            x: self.x + delta.x,
            y: self.y + delta.y,
        }
    }
}

/// A component's configuration values, keyed by config name.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Config(HashMap<String, serde_json::Value>);

impl Config {
    pub fn new(values: HashMap<String, serde_json::Value>) -> Self {
        Self(values)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &serde_json::Value)> {
        self.0.iter()
    }

    pub fn get(&self, name: &str) -> Option<&serde_json::Value> {
        self.0.get(name)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.0.contains_key(name)
    }

    pub fn with_set(&self, name: &str, value: serde_json::Value) -> Self {
        let mut values = self.0.clone();
        values.insert(name.to_owned(), value);
        Self(values)
    }

    pub fn without(&self, name: &str) -> Self {
        let mut values = self.0.clone();
        values.remove(name);
        Self(values)
    }
}

#[derive(Debug, Clone)]
pub struct Component {
    pub definition: ComponentDefinition,
    pub position: Position,
    /// `None` for external components: their configuration lives on whatever
    /// instance actually owns them, not in this project.
    pub config: Option<Config>,
    pub external: bool,
}

impl Component {
    pub fn moved_by(&self, delta: Position) -> Arc<Self> {
        let mut component = self.clone();
        component.position = component.position.moved_by(delta);
        Arc::new(component)
    }

    pub fn with_config_value(
        &self,
        name: &str,
        value: serde_json::Value,
    ) -> Result<Arc<Self>, CoreModelError> {
        let config = self
            .config
            .as_ref()
            .ok_or_else(|| CoreModelError::ComponentHasNoConfig)?;

        // An exported config item is removed from the component's own map: the
        // template instance supplies it. Setting it here would resurrect a key
        // the export owns.
        if !config.contains(name) {
            return Err(CoreModelError::ConfigNotSettable {
                name: name.to_owned(),
            });
        }

        let mut component = self.clone();
        component.config = Some(config.with_set(name, value));
        Ok(Arc::new(component))
    }

    /// Hand a config item over to a template export, which then supplies it.
    pub fn without_config_value(&self, name: &str) -> Arc<Self> {
        let Some(config) = &self.config else {
            return Arc::new(self.clone());
        };

        let mut component = self.clone();
        component.config = Some(config.without(name));
        Arc::new(component)
    }

    pub fn with_config_value_unchecked(&self, name: &str, value: serde_json::Value) -> Arc<Self> {
        let config = self.config.clone().unwrap_or_default();

        let mut component = self.clone();
        component.config = Some(config.with_set(name, value));
        Arc::new(component)
    }
}

/// A binding carries no id of its own: `Binding::id` derives it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub source_component: ComponentId,
    pub source_state: String,
    pub target_component: ComponentId,
    pub target_action: String,
}

impl Binding {
    pub fn id(&self) -> Result<BindingId, CoreModelError> {
        BindingId::build(
            &self.source_component,
            &self.source_state,
            &self.target_component,
            &self.target_action,
        )
    }

    /// A binding that mentions `id` under its new name. `None` when it does
    /// not mention it, so untouched bindings keep their pointer.
    pub fn on_rename_component(&self, id: &ComponentId, new_id: &ComponentId) -> Option<Arc<Self>> {
        let source = &self.source_component == id;
        let target = &self.target_component == id;

        if !source && !target {
            return None;
        }

        let mut binding = self.clone();
        if source {
            binding.source_component = new_id.clone();
        }
        if target {
            binding.target_component = new_id.clone();
        }
        Some(Arc::new(binding))
    }

    pub fn mentions(&self, id: &ComponentId) -> bool {
        &self.source_component == id || &self.target_component == id
    }
}

#[derive(Debug, Clone)]
pub struct Plugin {
    pub name: String,
    pub module: String,
    pub usage: PluginUsage,
    pub version: String,
    pub description: Option<String>,
    pub members: HashMap<String, Member>,
    pub config: HashMap<String, ConfigItem>,
    pub instance_name: InstanceId,
    pub toolbox_display: ToolboxDisplay,
}

impl Plugin {
    /// `None` when the display already matches, so nothing is emitted.
    pub fn with_display(&self, display: ToolboxDisplay) -> Option<Arc<Self>> {
        if self.toolbox_display == display {
            return None;
        }

        let mut plugin = self.clone();
        plugin.toolbox_display = display;
        Some(Arc::new(plugin))
    }

    pub fn member(&self, name: &str) -> Result<&Member, CoreModelError> {
        self.members
            .get(name)
            .ok_or_else(|| CoreModelError::MemberNotFound {
                component: self.to_string(),
                member: name.to_owned(),
                member_type: "any",
            })
    }

    pub fn config_item(&self, name: &str) -> Result<&ConfigItem, CoreModelError> {
        self.config
            .get(name)
            .ok_or_else(|| CoreModelError::ConfigNotFound {
                owner: self.to_string(),
                name: name.to_owned(),
            })
    }

    /// The value a freshly created component starts a config item at.
    pub fn config_default(&self, name: &str) -> Result<serde_json::Value, CoreModelError> {
        default_config_value(&self.config_item(name)?.value_type)
    }

    /// Whether a value is one this plugin's config item accepts.
    pub fn check_config_value(
        &self,
        name: &str,
        value: &serde_json::Value,
    ) -> Result<(), CoreModelError> {
        let item = self.config_item(name)?;
        check_config_value(&self.to_string(), name, &item.value_type, value)
    }

    /// What differs between this plugin and another carrying the same id.
    ///
    /// Both the import and the validation pass ask this; they differ only in
    /// what they call the answer.
    pub fn differences(&self, other: &Plugin) -> PluginDifferences {
        PluginDifferences {
            config: differences(&self.config, &other.config, |mine, theirs| {
                mine.value_type == theirs.value_type && mine.description == theirs.description
            }),
            members: differences(&self.members, &other.members, |mine, theirs| {
                mine.member_type == theirs.member_type
                    && mine.value_type == theirs.value_type
                    && mine.description == theirs.description
            }),
        }
    }

    pub fn config_template(&self) -> Result<Config, CoreModelError> {
        let mut values = HashMap::new();

        for name in self.config.keys() {
            values.insert(name.clone(), self.config_default(name)?);
        }

        Ok(Config::new(values))
    }
}

impl std::fmt::Display for Plugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}.{}", self.instance_name, self.module, self.name)
    }
}

/// What one plugin has that another does not, named from the other's side:
/// `Added` is on the other and not here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Difference {
    Added,
    Changed,
    Removed,
}

#[derive(Debug, Clone, Default)]
pub struct PluginDifferences {
    pub config: BTreeMap<String, Difference>,
    pub members: BTreeMap<String, Difference>,
}

impl PluginDifferences {
    pub fn is_empty(&self) -> bool {
        self.config.is_empty() && self.members.is_empty()
    }
}

fn differences<T>(
    mine: &HashMap<String, T>,
    theirs: &HashMap<String, T>,
    same: impl Fn(&T, &T) -> bool,
) -> BTreeMap<String, Difference> {
    let mut found = BTreeMap::new();

    for (name, theirs) in theirs {
        match mine.get(name) {
            None => {
                found.insert(name.clone(), Difference::Added);
            }
            Some(mine) if !same(mine, theirs) => {
                found.insert(name.clone(), Difference::Changed);
            }
            Some(_) => {}
        }
    }

    for name in mine.keys() {
        if !theirs.contains_key(name) {
            found.insert(name.clone(), Difference::Removed);
        }
    }

    found
}

pub fn default_config_value(value_type: &ConfigType) -> Result<serde_json::Value, CoreModelError> {
    Ok(match value_type {
        ConfigType::String => serde_json::Value::String(String::new()),
        ConfigType::Bool => serde_json::Value::Bool(false),
        ConfigType::Integer | ConfigType::Float => serde_json::Value::from(0),
    })
}

pub fn check_config_value(
    owner: &str,
    name: &str,
    value_type: &ConfigType,
    value: &serde_json::Value,
) -> Result<(), CoreModelError> {
    let ok = match value_type {
        ConfigType::String => value.is_string(),
        ConfigType::Bool => value.is_boolean(),
        ConfigType::Integer => value.is_i64() || value.is_u64(),
        ConfigType::Float => value.is_number(),
    };

    if ok {
        return Ok(());
    }

    Err(CoreModelError::ConfigValueMismatch {
        owner: owner.to_owned(),
        name: name.to_owned(),
        expected: format!("{value_type:?}").to_lowercase(),
    })
}

/// A template is a view, plus the exports that let whatever instantiates it
/// reach inside.
#[derive(Debug, Clone, Default)]
pub struct Template {
    pub components: Collection<ComponentId, Component>,
    pub bindings: Collection<BindingId, Binding>,
    pub exports: TemplateExports,
}

impl Template {
    pub fn empty() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, Default)]
pub struct TemplateExports {
    pub config: HashMap<ExportId, ConfigExport>,
    pub members: HashMap<ExportId, MemberExport>,
}

impl TemplateExports {
    /// Whether any export points at this component. Deleting or renaming one
    /// has to consult this.
    pub fn mentions(&self, component: &ComponentId) -> bool {
        self.config
            .values()
            .any(|export| &export.component == component)
            || self
                .members
                .values()
                .any(|export| &export.component == component)
    }

    pub fn on_rename_component(&self, id: &ComponentId, new_id: &ComponentId) -> Option<Self> {
        if !self.mentions(id) {
            return None;
        }

        let mut exports = self.clone();

        for export in exports.config.values_mut() {
            if &export.component == id {
                export.component = new_id.clone();
            }
        }

        for export in exports.members.values_mut() {
            if &export.component == id {
                export.component = new_id.clone();
            }
        }

        Some(exports)
    }

    pub fn config_export(&self, id: &ExportId) -> Result<&ConfigExport, CoreModelError> {
        self.config
            .get(id)
            .ok_or_else(|| CoreModelError::not_found(IdKind::Export, id))
    }

    pub fn member_export(&self, id: &ExportId) -> Result<&MemberExport, CoreModelError> {
        self.members
            .get(id)
            .ok_or_else(|| CoreModelError::not_found(IdKind::Export, id))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigExport {
    pub component: ComponentId,
    pub config_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberExport {
    pub component: ComponentId,
    pub member: String,
}

/// Which half of a template's exports an operation is addressing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportKind {
    Config,
    Member,
}

impl ExportKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            ExportKind::Config => "config",
            ExportKind::Member => "member",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreModelError> {
        match value {
            "config" => Ok(ExportKind::Config),
            "member" => Ok(ExportKind::Member),
            _ => Err(CoreModelError::UnknownExportKind {
                value: value.to_owned(),
            }),
        }
    }
}

pub const fn member_type_name(member_type: MemberType) -> &'static str {
    match member_type {
        MemberType::Action => "action",
        MemberType::State => "state",
    }
}
