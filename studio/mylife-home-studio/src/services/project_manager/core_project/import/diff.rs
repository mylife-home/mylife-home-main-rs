//! What differs between a project and what is being imported.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use studio_web_api::project_manager::core_import_data::{
    ChangeType, ComponentChange, ComponentChangeConfigEntry, ObjectChange, PluginChange,
    VersionChange,
};

use super::super::model::{
    ComponentDefinition, ComponentId, Config, CoreModelError, Difference, Id, InstanceId, Plugin,
    PluginId, Project, ViewKey,
};
use super::load::{ComponentImport, ImportData};

/// Everything the import would change, as the designer lists it.
pub fn prepare_changes(
    imports: &ImportData,
    project: &Project,
) -> Result<Vec<ObjectChange>, CoreModelError> {
    let mut changes = plugin_changes(imports, project);
    changes.extend(component_changes(imports, project)?);

    dependencies(imports, &mut changes);

    Ok(changes)
}

fn plugin_changes(imports: &ImportData, project: &Project) -> Vec<ObjectChange> {
    let mut changes = Vec::new();

    for import in &imports.plugins {
        match project.plugins().get(&import.id) {
            None => changes.push(ObjectChange::Plugin(PluginChange {
                key: format!("plugin-set:{}", import.id),
                id: import.id.to_string(),
                change_type: ChangeType::Add,
                dependencies: Vec::new(),
                impacts: Vec::new(),
                instance_name: import.plugin.instance_name.to_string(),
                version: VersionChange {
                    before: String::new(),
                    after: import.plugin.version.clone(),
                },
                usage: Some(import.plugin.usage),
                config: everything_added(import.plugin.config.keys()),
                members: everything_added(import.plugin.members.keys()),
            })),

            Some(existing) if !same_plugin(existing, &import.plugin) => {
                let found = existing.differences(&import.plugin);

                changes.push(ObjectChange::Plugin(PluginChange {
                    key: format!("plugin-set:{}", import.id),
                    id: import.id.to_string(),
                    change_type: ChangeType::Update,
                    dependencies: Vec::new(),
                    impacts: Vec::new(),
                    instance_name: existing.instance_name.to_string(),
                    version: VersionChange {
                        before: existing.version.clone(),
                        after: import.plugin.version.clone(),
                    },
                    usage: (existing.usage != import.plugin.usage).then_some(import.plugin.usage),
                    config: change_types(&found.config),
                    members: change_types(&found.members),
                }));
            }

            Some(_) => {}
        }
    }

    let mut gone: Vec<(&PluginId, &Plugin)> = project
        .plugins()
        .iter()
        .filter(|(id, _)| imports.plugin(id).is_none())
        .map(|(id, plugin)| (id, plugin.as_ref()))
        .collect();
    gone.sort_by_key(|(id, _)| *id);

    for (id, plugin) in gone {
        changes.push(ObjectChange::Plugin(PluginChange {
            key: format!("plugin-clear:{id}"),
            id: id.to_string(),
            change_type: ChangeType::Delete,
            dependencies: Vec::new(),
            impacts: Vec::new(),
            instance_name: plugin.instance_name.to_string(),
            version: VersionChange {
                before: plugin.version.clone(),
                after: String::new(),
            },
            usage: None,
            config: HashMap::new(),
            members: HashMap::new(),
        }));
    }

    changes
}

/// A project deploys whole instances, so an instance the import touches is one
/// whose components the import is now the authority on: anything else on it
/// goes.
fn component_changes(
    imports: &ImportData,
    project: &Project,
) -> Result<Vec<ObjectChange>, CoreModelError> {
    let mut changes = Vec::new();

    let mut imported_by_instance: BTreeMap<InstanceId, BTreeSet<ComponentId>> = BTreeMap::new();

    for import in &imports.components {
        let Some(plugin) = imports.plugin(&import.plugin) else {
            continue;
        };

        imported_by_instance
            .entry(plugin.plugin.instance_name.clone())
            .or_default()
            .insert(import.id.clone());
    }

    let view = project.view(&ViewKey::Project)?;

    for import in &imports.components {
        let instance = imports
            .plugin(&import.plugin)
            .map(|plugin| plugin.plugin.instance_name.to_string())
            .unwrap_or_default();

        match view.components.get(&import.id) {
            None => changes.push(ObjectChange::Component(ComponentChange {
                key: format!("component-set:{}", import.id),
                id: import.id.to_string(),
                change_type: ChangeType::Add,
                dependencies: Vec::new(),
                impacts: Vec::new(),
                instance_name: instance,
                config: config_entries(None, import.config.as_ref()),
                external: Some(import.external),
                plugin_id: Some(import.plugin.to_string()),
            })),

            Some(existing) if !same_component(existing, import) => {
                let plugin = match &existing.definition {
                    ComponentDefinition::Plugin(id) => Some(id),
                    ComponentDefinition::Template(_) => None,
                };

                changes.push(ObjectChange::Component(ComponentChange {
                    key: format!("component-set:{}", import.id),
                    id: import.id.to_string(),
                    change_type: ChangeType::Update,
                    dependencies: Vec::new(),
                    impacts: Vec::new(),
                    instance_name: instance_of(project, existing)?,
                    config: config_entries(existing.config.as_ref(), import.config.as_ref()),
                    external: (existing.external != import.external).then_some(import.external),
                    plugin_id: (plugin != Some(&import.plugin)).then(|| import.plugin.to_string()),
                }));
            }

            Some(_) => {}
        }
    }

    let instances = project.instances();

    for (instance, imported) in &imported_by_instance {
        if !instances.contains_key(instance) {
            continue;
        }

        for (id, component) in &view.components {
            if imported.contains(id) {
                continue;
            }

            let ComponentDefinition::Plugin(plugin_id) = &component.definition else {
                continue;
            };

            if project.plugins().get_required(plugin_id)?.instance_name != *instance {
                continue;
            }

            changes.push(ObjectChange::Component(ComponentChange {
                key: format!("component-clear:{id}"),
                id: id.to_string(),
                change_type: ChangeType::Delete,
                dependencies: Vec::new(),
                impacts: Vec::new(),
                instance_name: instance.to_string(),
                config: HashMap::new(),
                external: None,
                plugin_id: None,
            }));
        }
    }

    Ok(changes)
}

/// A component to add or update needs its plugin changed first.
fn dependencies(imports: &ImportData, changes: &mut [ObjectChange]) {
    let plugin_keys: HashMap<String, String> = changes
        .iter()
        .filter_map(|change| match change {
            ObjectChange::Plugin(plugin) => Some((plugin.id.clone(), plugin.key.clone())),
            _ => None,
        })
        .collect();

    for change in changes {
        let ObjectChange::Component(component) = change else {
            continue;
        };

        if matches!(component.change_type, ChangeType::Delete) {
            continue;
        }

        let Some(import) = imports
            .components
            .iter()
            .find(|import| import.id.as_str() == component.id)
        else {
            continue;
        };

        if let Some(key) = plugin_keys.get(import.plugin.as_str()) {
            component.dependencies.push(key.clone());
        }
    }
}

/// The instance a component already in the project runs on. A component of a
/// template has none; the designer groups those under one heading.
fn instance_of(
    project: &Project,
    component: &super::super::model::Component,
) -> Result<String, CoreModelError> {
    match &component.definition {
        ComponentDefinition::Plugin(id) => Ok(project
            .plugins()
            .get_required(id)?
            .instance_name
            .to_string()),
        ComponentDefinition::Template(_) => Ok(String::from("<Templates>")),
    }
}

fn same_plugin(existing: &Plugin, imported: &Plugin) -> bool {
    existing.instance_name == imported.instance_name
        && existing.module == imported.module
        && existing.name == imported.name
        && existing.version == imported.version
}

fn same_component(existing: &super::super::model::Component, imported: &ComponentImport) -> bool {
    let ComponentDefinition::Plugin(plugin) = &existing.definition else {
        return false;
    };

    plugin == &imported.plugin
        && existing.external == imported.external
        && existing.config.is_some() == imported.config.is_some()
        && match (&existing.config, &imported.config) {
            (Some(existing), Some(imported)) => same_config(existing, imported),
            _ => true,
        }
}

fn same_config(existing: &Config, imported: &Config) -> bool {
    let existing: BTreeMap<_, _> = existing.iter().collect();
    let imported: BTreeMap<_, _> = imported.iter().collect();

    existing == imported
}

fn everything_added<'a>(names: impl Iterator<Item = &'a String>) -> HashMap<String, ChangeType> {
    names.map(|name| (name.clone(), ChangeType::Add)).collect()
}

fn change_types(found: &BTreeMap<String, Difference>) -> HashMap<String, ChangeType> {
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

fn config_entries(
    existing: Option<&Config>,
    imported: Option<&Config>,
) -> HashMap<String, ComponentChangeConfigEntry> {
    let mut entries = HashMap::new();

    let existing: BTreeMap<&String, &serde_json::Value> = existing
        .map(|config| config.iter().collect())
        .unwrap_or_default();
    let imported: BTreeMap<&String, &serde_json::Value> = imported
        .map(|config| config.iter().collect())
        .unwrap_or_default();

    for (name, value) in &imported {
        let change = match existing.get(name) {
            None => ChangeType::Add,
            Some(existing) if existing != value => ChangeType::Update,
            Some(_) => continue,
        };

        entries.insert(
            (*name).clone(),
            ComponentChangeConfigEntry {
                r#type: change,
                value: (*value).clone(),
            },
        );
    }

    for name in existing.keys() {
        if !imported.contains_key(name) {
            entries.insert(
                (*name).clone(),
                ComponentChangeConfigEntry {
                    r#type: ChangeType::Delete,
                    value: serde_json::Value::Null,
                },
            );
        }
    }

    entries
}
