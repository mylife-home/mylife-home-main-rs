//! Deploying a project onto the running instances.
//!
//! The deploy is planned before anything is changed. The plan goes back to the
//! designer, which shows it and asks; the apply step then carries out exactly
//! that plan, so what was shown is what happens.
//!
//! Deciding what to do is here and is a function of two values: the flattened
//! project, and what the instances currently hold. Doing it belongs to the
//! actor, which owns the connections.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use studio_web_api::core_model::{BindingConfig, ComponentConfig};
use studio_web_api::project_manager::{
    BindingDeployChange, ChangeType, ComponentDeployChange, DeployChanges,
};

use super::model::{BindingId, ComponentId, CoreModelError, Id, IdKind};
use super::resolver::ResolvedProject;

/// What the instances hold right now.
#[derive(Debug, Default)]
pub struct OnlineStore {
    /// The instance declaring `bindings-api`, when the project has bindings.
    pub bindings_instance: Option<String>,
    pub bindings: Vec<BindingConfig>,
    pub components: BTreeMap<String, Vec<ComponentConfig>>,
}

/// What a deploy to online would do.
#[derive(Debug)]
pub struct OnlinePlan {
    pub changes: DeployChanges,
    pub tasks: Vec<Task>,
}

/// Travels to the designer and back untouched between prepare and apply, so
/// it is serde-shaped rather than model-shaped.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub instance_name: String,
    /// Never an update: a component that changed is removed and added again.
    pub change_type: TaskChange,
    pub object: TaskObject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskChange {
    Add,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "objectType", rename_all = "camelCase")]
pub enum TaskObject {
    Component { id: String },
    Binding { id: String },
}

/// An instance is deployed whole from one project, so anything running there
/// that the project does not have goes. External components are another
/// project's, so they are left alone.
pub fn prepare_to_online(
    project: &ResolvedProject,
    store: &OnlineStore,
) -> Result<OnlinePlan, CoreModelError> {
    let mut changes = DeployChanges {
        components: Vec::new(),
        bindings: Vec::new(),
    };

    let mut bindings_delete = Vec::new();
    let mut bindings_add = Vec::new();
    let mut components_delete = Vec::new();
    let mut components_add = Vec::new();

    if !project.bindings().is_empty() {
        // Validation has already established there is exactly one.
        let Some(instance) = &store.bindings_instance else {
            return Err(CoreModelError::BindingsInstanceNeeded);
        };

        let online: BTreeSet<String> = store
            .bindings
            .iter()
            .map(|binding| {
                format!(
                    "{}:{}:{}:{}",
                    binding.source_component,
                    binding.source_state,
                    binding.target_component,
                    binding.target_action
                )
            })
            .collect();

        let wanted: BTreeSet<String> = project.bindings().keys().map(ToString::to_string).collect();

        for id in online.difference(&wanted) {
            changes.bindings.push(BindingDeployChange {
                r#type: ChangeType::Delete,
                instance_name: Some(instance.clone()),
                binding_id: id.clone(),
            });
            bindings_delete.push(Task {
                instance_name: instance.clone(),
                change_type: TaskChange::Delete,
                object: TaskObject::Binding { id: id.clone() },
            });
        }

        for id in wanted.difference(&online) {
            changes.bindings.push(BindingDeployChange {
                r#type: ChangeType::Add,
                instance_name: Some(instance.clone()),
                binding_id: id.clone(),
            });
            bindings_add.push(Task {
                instance_name: instance.clone(),
                change_type: TaskChange::Add,
                object: TaskObject::Binding { id: id.clone() },
            });
        }
    }

    for (instance, components) in project.instances()? {
        let instance_name = instance.to_string();
        let empty = Vec::new();
        let online = store.components.get(&instance_name).unwrap_or(&empty);

        let wanted: BTreeSet<String> = components.iter().map(ToString::to_string).collect();

        for config in online {
            if wanted.contains(&config.id) {
                continue;
            }

            changes.components.push(ComponentDeployChange {
                r#type: ChangeType::Delete,
                instance_name: instance_name.clone(),
                component_id: config.id.clone(),
            });
            components_delete.push(Task {
                instance_name: instance_name.clone(),
                change_type: TaskChange::Delete,
                object: TaskObject::Component {
                    id: config.id.clone(),
                },
            });
        }

        for id in &components {
            let wanted_config = component_config(project, id)?;
            let running = online.iter().find(|config| config.id == id.to_string());

            match running {
                Some(running) if same_component(&wanted_config, running) => continue,

                Some(_) => {
                    // No update on the wire: take it away and put it back.
                    changes.components.push(ComponentDeployChange {
                        r#type: ChangeType::Update,
                        instance_name: instance_name.clone(),
                        component_id: id.to_string(),
                    });
                    components_delete.push(Task {
                        instance_name: instance_name.clone(),
                        change_type: TaskChange::Delete,
                        object: TaskObject::Component { id: id.to_string() },
                    });
                    components_add.push(Task {
                        instance_name: instance_name.clone(),
                        change_type: TaskChange::Add,
                        object: TaskObject::Component { id: id.to_string() },
                    });
                }

                None => {
                    changes.components.push(ComponentDeployChange {
                        r#type: ChangeType::Add,
                        instance_name: instance_name.clone(),
                        component_id: id.to_string(),
                    });
                    components_add.push(Task {
                        instance_name: instance_name.clone(),
                        change_type: TaskChange::Add,
                        object: TaskObject::Component { id: id.to_string() },
                    });
                }
            }
        }
    }

    // Bindings come off first and go on last: a binding cannot outlive either
    // of the components it joins.
    let mut tasks = bindings_delete;
    tasks.extend(components_delete);
    tasks.extend(components_add);
    tasks.extend(bindings_add);

    Ok(OnlinePlan { changes, tasks })
}

fn same_component(wanted: &ComponentConfig, running: &ComponentConfig) -> bool {
    wanted.id == running.id && wanted.plugin == running.plugin && wanted.config == running.config
}

/// What a component looks like to the instance that runs it.
pub fn component_config(
    project: &ResolvedProject,
    id: &ComponentId,
) -> Result<ComponentConfig, CoreModelError> {
    let component = project.component(id)?;
    let plugin = project.plugin_of(component)?;

    Ok(ComponentConfig {
        id: id.to_string(),
        plugin: format!("{}.{}", plugin.module, plugin.name),
        config: component
            .config
            .iter()
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect(),
    })
}

/// A binding that only runs online is known by its id alone, so this has to
/// go backwards.
pub fn binding_config(id: &BindingId) -> Result<BindingConfig, CoreModelError> {
    let parts: Vec<&str> = id.as_str().split(':').collect();

    let [
        source_component,
        source_state,
        target_component,
        target_action,
    ] = parts.as_slice()
    else {
        return Err(CoreModelError::not_found(IdKind::Binding, id));
    };

    Ok(BindingConfig {
        source_component: (*source_component).to_owned(),
        source_state: (*source_state).to_owned(),
        target_component: (*target_component).to_owned(),
        target_action: (*target_action).to_owned(),
    })
}
