//! Checking a project against what is running.
//!
//! The project is checked after it has been flattened, so every component here
//! is a component of a plugin and every binding joins two of them: the checks
//! never have to know templates existed.
//!
//! What the project is compared against arrives as a snapshot rather than
//! being looked up here, which is what lets this be a plain function over two
//! values. The actor gathers the snapshot; this decides what it means.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use studio_web_api::project_manager::core_validation::{
    BadExternalComponent, BindingMismatch, ChangeType, ComponentBadConfig, ExistingComponentId,
    InvalidBindingApi, Item, PluginChanged, PluginIdentity, Severity, VersionedPluginIdentity,
};

use super::model::{ComponentId, CoreModelError, Difference, InstanceId, Plugin, PluginId};
use super::resolver::ResolvedProject;

/// The capability an instance needs to carry the bindings a deploy creates.
pub const BINDINGS_API: &str = "bindings-api";

/// One component currently running.
#[derive(Debug, Clone)]
pub struct OnlineComponent {
    pub instance: InstanceId,
    pub plugin: Arc<Plugin>,
}

/// What the project is checked against: the plugins and components running
/// right now, and which instances can carry bindings.
#[derive(Debug, Default)]
pub struct Online {
    plugins: HashMap<PluginId, Arc<Plugin>>,
    components: HashMap<ComponentId, OnlineComponent>,
    bindings_instances: Vec<String>,
}

impl Online {
    pub fn new(
        plugins: impl IntoIterator<Item = (PluginId, Arc<Plugin>)>,
        components: impl IntoIterator<Item = (ComponentId, OnlineComponent)>,
        bindings_instances: Vec<String>,
    ) -> Self {
        Self {
            plugins: plugins.into_iter().collect(),
            components: components.into_iter().collect(),
            bindings_instances,
        }
    }

    fn plugin(&self, id: &PluginId) -> Option<&Arc<Plugin>> {
        self.plugins.get(id)
    }

    fn component(&self, id: &ComponentId) -> Option<&OnlineComponent> {
        self.components.get(id)
    }
}

/// How the two passes that use this differ.
#[derive(Debug, Clone, Copy)]
pub struct Options {
    /// How bad it is that the project no longer matches what is running. A
    /// deploy cannot proceed over one of these; a plain check only says so.
    pub online_severity: Severity,
    /// Only a deploy needs somewhere to put the bindings.
    pub check_bindings_instance: bool,
}

/// Everything wrong with the project, worst left for the caller to decide.
pub fn validate(
    project: &ResolvedProject,
    online: &Online,
    options: Options,
) -> Result<Vec<Item>, CoreModelError> {
    let mut items = Vec::new();

    changed_plugins(project, online, options.online_severity, &mut items)?;
    components_that_exist_elsewhere(project, online, options.online_severity, &mut items)?;
    external_components(project, online, options.online_severity, &mut items)?;

    if options.check_bindings_instance {
        bindings_instance(project, online, &mut items);
    }

    // These two are about the project on its own. Nothing an operation allows
    // can produce them, so they catch a project file edited by hand.
    component_configs(project, &mut items)?;
    binding_types(project, &mut items)?;

    Ok(items)
}

/// Plugins the project deploys that are gone, or no longer the same.
fn changed_plugins(
    project: &ResolvedProject,
    online: &Online,
    severity: Severity,
    items: &mut Vec<Item>,
) -> Result<(), CoreModelError> {
    let mut used: BTreeMap<PluginId, Vec<String>> = BTreeMap::new();

    for (id, component) in project.components() {
        if component.external {
            continue;
        }

        used.entry(component.plugin.clone())
            .or_default()
            .push(id.to_string());
    }

    for (plugin_id, impacts) in used {
        let plugin = project.plugins().get_required(&plugin_id)?;

        let Some(running) = online.plugin(&plugin_id) else {
            items.push(Item::PluginChanged(PluginChanged {
                severity,
                instance_name: plugin.instance_name.to_string(),
                module: plugin.module.clone(),
                name: plugin.name.clone(),
                change_type: ChangeType::Delete,
                config: HashMap::new(),
                members: HashMap::new(),
                impacts,
            }));
            continue;
        };

        let found = plugin.differences(running);

        if found.is_empty() {
            continue;
        }

        items.push(Item::PluginChanged(PluginChanged {
            severity,
            instance_name: plugin.instance_name.to_string(),
            module: plugin.module.clone(),
            name: plugin.name.clone(),
            change_type: ChangeType::Update,
            config: change_types(&found.config),
            members: change_types(&found.members),
            impacts,
        }));
    }

    Ok(())
}

/// Component ids this project would take over from another instance.
fn components_that_exist_elsewhere(
    project: &ResolvedProject,
    online: &Online,
    severity: Severity,
    items: &mut Vec<Item>,
) -> Result<(), CoreModelError> {
    for (id, component) in project.components() {
        let Some(running) = online.component(id) else {
            continue;
        };

        let plugin = project.plugin_of(component)?;

        if running.instance == plugin.instance_name {
            continue;
        }

        items.push(Item::ExistingComponentId(ExistingComponentId {
            severity,
            component_id: id.to_string(),
            project: identity(&plugin.instance_name, plugin),
            existing: identity(&running.instance, &running.plugin),
        }));
    }

    Ok(())
}

/// External components are someone else's to deploy, so the project only
/// borrows them: it has to agree with what is actually there.
fn external_components(
    project: &ResolvedProject,
    online: &Online,
    severity: Severity,
    items: &mut Vec<Item>,
) -> Result<(), CoreModelError> {
    for (id, component) in project.components() {
        if !component.external {
            continue;
        }

        let plugin = project.plugin_of(component)?;

        let Some(running) = online.component(id) else {
            items.push(Item::BadExternalComponent(BadExternalComponent {
                severity: Severity::Warning,
                component_id: id.to_string(),
                project: versioned_identity(&plugin.instance_name, plugin),
                existing: None,
            }));
            continue;
        };

        if running.instance == plugin.instance_name
            && running.plugin.module == plugin.module
            && running.plugin.name == plugin.name
            && running.plugin.version == plugin.version
        {
            continue;
        }

        // A component whose members still line up can be talked to, so the
        // difference is worth saying and not worth stopping for.
        let members = plugin.differences(&running.plugin).members;
        let severity = if members.is_empty() {
            Severity::Info
        } else {
            severity
        };

        items.push(Item::BadExternalComponent(BadExternalComponent {
            severity,
            component_id: id.to_string(),
            project: versioned_identity(&plugin.instance_name, plugin),
            existing: Some(versioned_identity(&running.instance, &running.plugin)),
        }));
    }

    Ok(())
}

/// Bindings have to be carried by exactly one instance.
fn bindings_instance(project: &ResolvedProject, online: &Online, items: &mut Vec<Item>) {
    if project.bindings().is_empty() || online.bindings_instances.len() == 1 {
        return;
    }

    items.push(Item::InvalidBindingApi(InvalidBindingApi {
        severity: Severity::Error,
        instance_names: online.bindings_instances.clone(),
    }));
}

/// Every config item a plugin declares, and nothing else.
fn component_configs(
    project: &ResolvedProject,
    items: &mut Vec<Item>,
) -> Result<(), CoreModelError> {
    for (id, component) in project.components() {
        if component.external {
            continue;
        }

        let plugin = project.plugin_of(component)?;
        let mut errors = HashMap::new();

        for (name, _) in component.config.iter() {
            if !plugin.config.contains_key(name) {
                errors.insert(name.clone(), String::from("Extra config key"));
            }
        }

        for name in plugin.config.keys() {
            let Some(value) = component.config.get(name) else {
                errors.insert(name.clone(), String::from("Missing config key"));
                continue;
            };

            if let Err(error) = plugin.check_config_value(name, value) {
                errors.insert(name.clone(), error.to_string());
            }
        }

        if errors.is_empty() {
            continue;
        }

        items.push(Item::ComponentBadConfig(ComponentBadConfig {
            severity: Severity::Error,
            component_id: id.to_string(),
            instance_name: plugin.instance_name.to_string(),
            module: plugin.module.clone(),
            name: plugin.name.clone(),
            config: errors,
        }));
    }

    Ok(())
}

/// Both ends of a binding have to exist and carry the same type.
fn binding_types(project: &ResolvedProject, items: &mut Vec<Item>) -> Result<(), CoreModelError> {
    for binding in project.bindings().values() {
        let source = project.component(&binding.source_component)?;
        let target = project.component(&binding.target_component)?;

        let source_type = project
            .plugin_of(source)?
            .members
            .get(&binding.source_state)
            .map(|member| member.value_type.clone());
        let target_type = project
            .plugin_of(target)?
            .members
            .get(&binding.target_action)
            .map(|member| member.value_type.clone());

        if source_type.is_some() && source_type == target_type {
            continue;
        }

        items.push(Item::BindingMismatch(BindingMismatch {
            severity: Severity::Error,
            source_component: binding.source_component.to_string(),
            source_state: binding.source_state.clone(),
            source_type,
            target_component: binding.target_component.to_string(),
            target_action: binding.target_action.clone(),
            target_type,
        }));
    }

    Ok(())
}

/// The same differences, named the way validation names them.
fn change_types(
    found: &std::collections::BTreeMap<String, Difference>,
) -> HashMap<String, ChangeType> {
    found
        .iter()
        .map(|(name, difference)| {
            let change = match difference {
                Difference::Added => ChangeType::Add,
                Difference::Changed => ChangeType::Update,
                Difference::Removed => ChangeType::Delete,
            };

            (name.clone(), change)
        })
        .collect()
}

fn identity(instance: &InstanceId, plugin: &Plugin) -> PluginIdentity {
    PluginIdentity {
        instance_name: instance.to_string(),
        module: plugin.module.clone(),
        name: plugin.name.clone(),
    }
}

fn versioned_identity(instance: &InstanceId, plugin: &Plugin) -> VersionedPluginIdentity {
    VersionedPluginIdentity {
        instance_name: instance.to_string(),
        module: plugin.module.clone(),
        name: plugin.name.clone(),
        version: plugin.version.clone(),
    }
}
