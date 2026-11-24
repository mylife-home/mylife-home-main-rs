use common::components::{metadata, registry::RegistryHandle};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
    sync::Arc,
};
use studio_web_api::project_manager::{self, CoreProject};

use crate::services::project_manager::{
    ProjectManagerActorError,
    ui_project::model::{self, Collection, Component, ComponentId, Plugin, PluginId, UiModelError},
};

/// The component and plugin set to move the project onto.
#[derive(Debug, Clone)]
pub struct NewComponentData {
    components: Collection<ComponentId, Component>,
    plugins: Collection<PluginId, Plugin>,
}

impl NewComponentData {
    /// The set a refresh moves the project onto. `load_online_component_data`
    /// builds it from the registry, so this is the way to state one outright.
    #[allow(dead_code, reason = "only the tests build a set by hand")]
    pub fn new(
        components: Collection<ComponentId, Component>,
        plugins: Collection<PluginId, Plugin>,
    ) -> Self {
        Self {
            components,
            plugins,
        }
    }
}

/// What the client hands back untouched between `prepare_refresh` and the
/// apply call.
///
/// Wire-shaped on purpose: it crosses the network and comes back, so its JSON
/// is a compatibility surface rather than the model's business. `Arc` is
/// transparent to serde, so this serialises exactly as it did before.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServerData {
    components: Arc<HashMap<String, Arc<project_manager::UiComponentData>>>,
    plugins: Arc<HashMap<String, Arc<project_manager::UiPluginData>>>,
    usage_to_clear: Vec<ComponentUsage>,
}

/// One usage, as it travels through that JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentUsage {
    pub component_id: String,
    pub member_name: String,
    pub path: project_manager::UiElementPath,
}

impl From<&model::ComponentUsage> for ComponentUsage {
    fn from(value: &model::ComponentUsage) -> Self {
        Self {
            component_id: value.component_id.to_string(),
            member_name: value.member_name.clone(),
            path: (&value.path).into(),
        }
    }
}

impl TryFrom<&ComponentUsage> for model::ComponentUsage {
    type Error = UiModelError;

    fn try_from(value: &ComponentUsage) -> Result<Self, UiModelError> {
        Ok(model::ComponentUsage {
            component_id: ComponentId::new(&value.component_id)?,
            member_name: value.member_name.clone(),
            path: (&value.path).try_into()?,
        })
    }
}

pub async fn load_online_component_data() -> Result<NewComponentData, ProjectManagerActorError> {
    let registry = RegistryHandle::new()?;

    let registry_components = registry.get_components().await?;

    let mut components = HashMap::new();
    let mut plugins = HashMap::new();

    for comp in registry_components {
        let Some(instance) = comp.instance else {
            tracing::warn!(component = %comp.component_id, "component has no instance. this is considered a local component, but local 'studio' instance has no component. skipping.");
            continue;
        };

        // Built rather than formatted: the ':' separator and the `module.name`
        // shape live in one place now.
        let plugin_id = PluginId::build(&instance, comp.plugin.module(), comp.plugin.name())?;

        if !plugins.contains_key(&plugin_id) {
            plugins.insert(
                plugin_id.clone(),
                Arc::new(Plugin {
                    name: comp.plugin.name().to_owned(),
                    module: comp.plugin.module().to_owned(),
                    version: comp.plugin.version().to_owned(),
                    description: comp.plugin.description().map(String::from),
                    members: comp
                        .plugin
                        .members()
                        .iter()
                        .map(|(id, member)| {
                            (
                                id.clone(),
                                project_manager::Member {
                                    description: member.description().map(String::from),
                                    member_type: match member.member_type() {
                                        metadata::MemberType::State => {
                                            project_manager::MemberType::State
                                        }
                                        metadata::MemberType::Action => {
                                            project_manager::MemberType::Action
                                        }
                                    },
                                    value_type: member.value_type().to_string(),
                                },
                            )
                        })
                        .collect::<HashMap<_, _>>(),
                    instance_name: instance,
                }),
            );
        }

        components.insert(
            ComponentId::new(&comp.component_id)?,
            Arc::new(Component { plugin: plugin_id }),
        );
    }

    Ok(NewComponentData {
        components: components.into_iter().collect(),
        plugins: plugins.into_iter().collect(),
    })
}

pub fn load_core_project_component_data(
    _project_data: Arc<CoreProject>,
) -> Result<NewComponentData, ProjectManagerActorError> {
    // TODO: template instantiation
    std::todo!()
}

struct UsageHelper {
    component_usage: HashMap<ComponentId, Vec<model::ComponentUsage>>,
    plugin_member_usage: HashMap<(PluginId, String), Vec<model::ComponentUsage>>,
}

impl UsageHelper {
    pub fn new(
        usage: &[model::ComponentUsage],
        project: &model::Project,
    ) -> Result<Self, UiModelError> {
        let mut component_usage: HashMap<_, Vec<_>> = HashMap::new();
        let mut plugin_member_usage: HashMap<_, Vec<_>> = HashMap::new();

        for item in usage {
            component_usage
                .entry(item.component_id.clone())
                .or_default()
                .push(item.clone());

            // The usage came from this project, so the component is there. It
            // is an error rather than an `expect` because the type cannot say
            // so, and a wrong `server_data` can reach here from the client.
            let component = project.components().get_required(&item.component_id)?;

            plugin_member_usage
                .entry((component.plugin.clone(), item.member_name.clone()))
                .or_default()
                .push(item.clone());
        }

        Ok(Self {
            component_usage,
            plugin_member_usage,
        })
    }

    pub fn find_component_usage(
        &self,
        component_id: &ComponentId,
    ) -> Option<&Vec<model::ComponentUsage>> {
        self.component_usage.get(component_id)
    }

    pub fn find_plugin_member_usage(
        &self,
        plugin: &PluginId,
        member_name: &str,
    ) -> Option<&Vec<model::ComponentUsage>> {
        self.plugin_member_usage
            .get(&(plugin.clone(), member_name.to_owned()))
    }
}

fn breaking_operation(
    operation: &str,
    component_id: &ComponentId,
    usage: &[model::ComponentUsage],
) -> project_manager::UiBreakingOperation {
    project_manager::UiBreakingOperation {
        operation: String::from(operation),
        component_id: component_id.to_string(),
        usage: usage.iter().map(|item| (&item.path).into()).collect(),
    }
}

pub fn prepare_refresh(
    project: &model::Project,
    usage: &[model::ComponentUsage],
    component_data: NewComponentData,
) -> Result<(Vec<project_manager::UiBreakingOperation>, Value), ProjectManagerActorError> {
    let usage_helper = UsageHelper::new(usage, project)?;
    let mut usage_to_clear = Vec::new();
    let mut breaking_operations = Vec::new();

    for (component_id, component) in project.components() {
        let Some(usage) = usage_helper.find_component_usage(component_id) else {
            continue;
        };

        let Some(new_component) = component_data.components.get(component_id) else {
            // component removed
            breaking_operations.push(breaking_operation("remove", component_id, usage));
            usage_to_clear.extend(usage.iter().map(ComponentUsage::from));
            continue;
        };

        let actual_plugin = project.plugins().get_required(&component.plugin)?;
        let new_plugin = component_data.plugins.get_required(&new_component.plugin)?;

        if !is_plugin_compatible(&usage_helper, &component.plugin, actual_plugin, new_plugin) {
            breaking_operations.push(breaking_operation("update", component_id, usage));
            usage_to_clear.extend(usage.iter().map(ComponentUsage::from));
        }
    }

    let server_data = ServerData {
        usage_to_clear,
        plugins: component_data.plugins.to_wire(),
        components: component_data.components.to_wire(),
    };

    Ok((breaking_operations, serde_json::to_value(server_data)?))
}

fn is_plugin_compatible(
    usage_helper: &UsageHelper,
    actual_plugin_id: &PluginId,
    actual_plugin: &Plugin,
    new_plugin: &Plugin,
) -> bool {
    if is_plugin_same(actual_plugin, new_plugin) {
        return true;
    }

    for (member_name, actual_member) in &actual_plugin.members {
        if usage_helper
            .find_plugin_member_usage(actual_plugin_id, member_name)
            .is_none()
        {
            continue;
        }

        let Some(new_member) = new_plugin.members.get(member_name) else {
            return false;
        };

        if !is_type_compatible(&actual_member.value_type, &new_member.value_type) {
            return false;
        }
    }

    true
}

fn is_plugin_same(actual_plugin: &Plugin, new_plugin: &Plugin) -> bool {
    actual_plugin.module == new_plugin.module
        && actual_plugin.name == new_plugin.name
        && actual_plugin.version == new_plugin.version
}

fn is_type_compatible(actual: &str, new: &str) -> bool {
    if actual == new {
        return true;
    }

    let actual_type = metadata::Type::from_str(actual).expect("invalid actual type");
    let new_type = metadata::Type::from_str(new).expect("invalid new type");

    // type promotion: enum -> text
    if let metadata::Type::Enum(_) = &actual_type
        && let metadata::Type::Text = &new_type
    {
        return true;
    }

    // type promotion: range -> float
    if let metadata::Type::Range(_) = &actual_type
        && let metadata::Type::Float = &new_type
    {
        return true;
    }

    // type promotion: range -> bigger range
    if let metadata::Type::Range(actual_range) = &actual_type
        && let metadata::Type::Range(new_range) = &new_type
        && actual_range.start() >= new_range.start()
        && actual_range.end() <= new_range.end()
    {
        return true;
    }

    // type promotion: enum -> bigger enum
    if let metadata::Type::Enum(actual_enum) = &actual_type
        && let metadata::Type::Enum(new_enum) = &new_type
    {
        let new_values: HashSet<_> = new_enum.iter().cloned().collect();
        if actual_enum.iter().all(|value| new_values.contains(value)) {
            return true;
        }
    }

    false
}

/// The apply half, parsed out of the JSON before it touches the model.
///
/// Splitting it this way is what lets the operation itself be a plain model
/// call: deserialising and validating happens here and fails with the actor's
/// error, while `apply` can only fail with a `UiModelError`.
pub struct Refresh {
    components: Collection<ComponentId, Component>,
    plugins: Collection<PluginId, Plugin>,
    usage_to_clear: Vec<model::ComponentUsage>,
}

impl Refresh {
    pub fn parse(server_data: Value) -> Result<Self, ProjectManagerActorError> {
        let server_data: ServerData = serde_json::from_value(server_data)?;

        Ok(Self {
            components: server_data.components.as_ref().try_into()?,
            plugins: server_data.plugins.as_ref().try_into()?,
            usage_to_clear: server_data
                .usage_to_clear
                .iter()
                .map(model::ComponentUsage::try_from)
                .collect::<Result<Vec<_>, UiModelError>>()?,
        })
    }

    /// Replace the component data, then drop every reference the client told us
    /// is breaking.
    ///
    /// `set_component_data` emits its own change, so there is no way to swap
    /// the component set here without the designer hearing about it.
    pub fn apply(
        &self,
        project: &model::Project,
        changes: &mut Vec<model::Change>,
    ) -> Result<model::Project, UiModelError> {
        let project =
            project.set_component_data(changes, self.components.clone(), self.plugins.clone());

        Ok(project
            .clear_component_usage(changes, &self.usage_to_clear)?
            .unwrap_or(project))
    }
}
