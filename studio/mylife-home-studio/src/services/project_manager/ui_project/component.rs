use common::components::{metadata, registry::RegistryHandle};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
    str::FromStr,
    sync::Arc,
};
use studio_web_api::project_manager::{self, CoreProject};

use crate::services::project_manager::{ProjectManagerActorError, ui_project::project::ProjectOps};

#[derive(Debug, Clone)]
pub struct NewComponentData {
    components: HashMap<String, project_manager::UiComponentData>,
    plugins: HashMap<String, project_manager::UiPluginData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServerData {
    components: HashMap<String, project_manager::UiComponentData>,
    plugins: HashMap<String, project_manager::UiPluginData>,
    usage_to_clear: Vec<ComponentUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentUsage {
    pub component_id: String,
    pub member_name: String,
    pub path: project_manager::UiElementPath,
}

#[derive(Debug, Clone)]
pub struct ComponentUsageCollector {
    usage: Rc<RefCell<Vec<ComponentUsage>>>,
    path: project_manager::UiElementPath,
}

impl ComponentUsageCollector {
    pub fn new() -> Self {
        Self {
            usage: Rc::new(RefCell::new(Vec::new())),
            path: Vec::new(),
        }
    }

    pub fn at(&self, node_type: impl Into<String>, id: impl Into<String>) -> Self {
        let mut path = self.path.clone();
        path.push(Arc::new(project_manager::UiElementPathNode {
            r#type: node_type.into(),
            id: id.into(),
        }));

        Self {
            path,
            ..self.clone()
        }
    }

    pub fn add_usage(&self, component_id: &str, member_name: &str) {
        let usage = ComponentUsage {
            component_id: component_id.to_string(),
            member_name: member_name.to_string(),
            path: self.path.clone(),
        };
        self.usage.borrow_mut().push(usage);
    }

    pub fn usage(&self) -> Vec<ComponentUsage> {
        self.usage.borrow().clone()
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

        let plugin_id = format!(
            "{}:{}.{}",
            instance,
            comp.plugin.module(),
            comp.plugin.name()
        );

        if !plugins.contains_key(&plugin_id) {
            plugins.insert(
                plugin_id.clone(),
                project_manager::UiPluginData {
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
                },
            );
        }

        components.insert(
            comp.component_id,
            project_manager::UiComponentData { plugin: plugin_id },
        );
    }

    Ok(NewComponentData {
        components,
        plugins,
    })
}

pub fn load_core_project_component_data(
    project_data: Arc<CoreProject>,
) -> Result<NewComponentData, ProjectManagerActorError> {
    // TODO: template instantiation
    // TODO: convert to
    //  pub components: Arc<HashMap<String, Arc<UiComponentData>>>,
    //  pub plugins: Arc<HashMap<String, Arc<UiPluginData>>>,
    std::todo!()
}

struct UsageHelper {
    component_usage: HashMap<String, Vec<ComponentUsage>>,
    plugin_member_usage: HashMap<(String, String), Vec<ComponentUsage>>,
}

impl UsageHelper {
    pub fn new(usage: &Vec<ComponentUsage>, project: &Arc<project_manager::UiProject>) -> Self {
        let mut component_usage = HashMap::new();
        let mut plugin_member_usage = HashMap::new();

        for item in usage {
            component_usage
                .entry(item.component_id.clone())
                .or_insert_with(Vec::new)
                .push(item.clone());

            let component = project
                .components
                .get(&item.component_id)
                .expect("invalid usage data");
            plugin_member_usage
                .entry((component.plugin.clone(), item.member_name.clone()))
                .or_insert_with(Vec::new)
                .push(item.clone());
        }

        Self {
            component_usage,
            plugin_member_usage,
        }
    }

    pub fn find_component_usage(&self, component_id: &str) -> Option<&Vec<ComponentUsage>> {
        self.component_usage.get(component_id)
    }

    pub fn find_plugin_member_usage(
        &self,
        plugin: &str,
        member_name: &str,
    ) -> Option<&Vec<ComponentUsage>> {
        self.plugin_member_usage
            .get(&(plugin.to_string(), member_name.to_string()))
    }
}

pub fn prepare_refresh(
    project: &Arc<project_manager::UiProject>,
    usage: Vec<ComponentUsage>,
    component_data: NewComponentData,
) -> Result<(Vec<project_manager::UiBreakingOperation>, Value), ProjectManagerActorError> {
    let usage_helper = UsageHelper::new(&usage, &project);
    let mut usage_to_clear = Vec::new();
    let mut breaking_operations = Vec::new();

    for (component_id, component) in project.components.iter() {
        let Some(usage) = usage_helper.find_component_usage(component_id) else {
            continue;
        };

        let Some(new_component) = component_data.components.get(component_id) else {
            // component removed
            breaking_operations.push(project_manager::UiBreakingOperation {
                operation: String::from("remove"),
                component_id: String::from(component_id),
                usage: usage.iter().map(|u| u.path.clone()).collect(),
            });

            for item in usage {
                usage_to_clear.push(item.clone());
            }

            continue;
        };

        let actual_plugin = project
            .plugins
            .get(&component.plugin)
            .expect("invalid plugin data");
        let new_plugin = component_data
            .plugins
            .get(&new_component.plugin)
            .expect("invalid plugin data");

        if !is_plugin_compatible(
            &usage_helper,
            &component.plugin,
            &actual_plugin,
            &new_plugin,
        ) {
            breaking_operations.push(project_manager::UiBreakingOperation {
                operation: String::from("update"),
                component_id: String::from(component_id),
                usage: usage.iter().map(|u| u.path.clone()).collect(),
            });

            for item in usage {
                usage_to_clear.push(item.clone());
            }
        }
    }

    let server_data = ServerData {
        usage_to_clear,
        plugins: component_data.plugins,
        components: component_data.components,
    };

    Ok((breaking_operations, serde_json::to_value(server_data)?))
}

fn is_plugin_compatible(
    usage_helper: &UsageHelper,
    actual_plugin_id: &str,
    actual_plugin: &project_manager::UiPluginData,
    new_plugin: &project_manager::UiPluginData,
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

fn is_plugin_same(
    actual_plugin: &project_manager::UiPluginData,
    new_plugin: &project_manager::UiPluginData,
) -> bool {
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
    {
        if actual_range.start() >= new_range.start() && actual_range.end() <= new_range.end() {
            return true;
        }
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

    return false;
}

pub fn apply_refresh(
    project: &Arc<project_manager::UiProject>,
    notifications: &mut Vec<project_manager::UpdateProjectNotification>,
    server_data: Value,
) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError> {
    let server_data: ServerData = serde_json::from_value(server_data)?;

    let mut project = project.as_ref().clone();
    project.plugins = Arc::new(
        server_data
            .plugins
            .into_iter()
            .map(|(id, plugin)| (id, Arc::new(plugin)))
            .collect(),
    );
    project.components = Arc::new(
        server_data
            .components
            .into_iter()
            .map(|(id, component)| (id, Arc::new(component)))
            .collect(),
    );
    let project = Arc::new(project);
    let project = project
        .clear_component_usage(notifications, &server_data.usage_to_clear)?
        .unwrap_or(project);

    Ok(project)
}
