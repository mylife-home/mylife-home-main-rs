use common::components::{metadata, registry::RegistryHandle};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::Arc};
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

pub fn prepare_refresh(
    usage: Vec<ComponentUsage>,
    component_data: NewComponentData,
) -> Result<(Vec<project_manager::UiBreakingOperation>, Value), ProjectManagerActorError> {
    std::todo!()
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
