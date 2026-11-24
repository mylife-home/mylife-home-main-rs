//! What an import brings in: plugins, and the components that use them.

use std::collections::BTreeMap;
use std::sync::Arc;

use common::components::metadata::{self, PluginMetadata};
use studio_web_api::component_model::{self, ConfigItem, Member};
use studio_web_api::project_manager::{
    ComponentsImportType, ImportFromOnlineConfig, ImportFromProjectConfig,
};

use super::super::model::{
    ComponentId, Config, CoreModelError, InstanceId, Plugin, PluginId, ToolboxDisplay,
};
use super::super::resolver::ResolvedProject;

/// One plugin to take in.
#[derive(Debug, Clone)]
pub struct PluginImport {
    pub id: PluginId,
    pub plugin: Plugin,
}

/// One component to take in. Always lands on the project view.
#[derive(Debug, Clone)]
pub struct ComponentImport {
    pub id: ComponentId,
    pub plugin: PluginId,
    pub external: bool,
    /// `None` for an external component: its configuration belongs to whoever
    /// deploys it.
    pub config: Option<Config>,
}

#[derive(Debug, Default)]
pub struct ImportData {
    pub plugins: Vec<PluginImport>,
    pub components: Vec<ComponentImport>,
}

impl ImportData {
    pub fn plugin(&self, id: &PluginId) -> Option<&PluginImport> {
        self.plugins.iter().find(|plugin| &plugin.id == id)
    }

    pub fn component(&self, id: &ComponentId) -> Option<&ComponentImport> {
        self.components.iter().find(|component| &component.id == id)
    }
}

/// One component as the registry reports it.
#[derive(Debug, Clone)]
pub struct OnlineComponent {
    pub id: ComponentId,
    pub instance: InstanceId,
    pub plugin: Arc<PluginMetadata>,
}

/// What is running, as the import sees it.
#[derive(Debug, Default)]
pub struct OnlineSource {
    pub plugins: Vec<(InstanceId, Arc<PluginMetadata>)>,
    pub components: Vec<OnlineComponent>,
}

/// Take in what is running.
///
/// Components come in external: what they are configured with lives on the
/// instance that owns them, and is not published.
pub fn from_online(
    source: &OnlineSource,
    config: &ImportFromOnlineConfig,
) -> Result<ImportData, CoreModelError> {
    let mut plugins: BTreeMap<PluginId, PluginImport> = BTreeMap::new();
    let mut components = Vec::new();

    if config.import_plugins {
        for (instance, metadata) in &source.plugins {
            let import = plugin_from_metadata(instance, metadata)?;
            plugins.insert(import.id.clone(), import);
        }
    }

    if config.import_components {
        for component in &source.components {
            let import = plugin_from_metadata(&component.instance, &component.plugin)?;
            let plugin = import.id.clone();
            plugins.entry(plugin.clone()).or_insert(import);

            components.push(ComponentImport {
                id: component.id.clone(),
                plugin,
                external: true,
                config: None,
            });
        }
    }

    Ok(ImportData {
        plugins: plugins.into_values().collect(),
        components,
    })
}

/// Take in another project, flattened: what it would deploy, not how it was
/// drawn.
pub fn from_project(
    source: &ResolvedProject,
    config: &ImportFromProjectConfig,
) -> Result<ImportData, CoreModelError> {
    let mut plugins: BTreeMap<PluginId, PluginImport> = BTreeMap::new();
    let mut components = Vec::new();

    let mut take_plugin = |id: &PluginId| -> Result<(), CoreModelError> {
        if plugins.contains_key(id) {
            return Ok(());
        }

        plugins.insert(
            id.clone(),
            PluginImport {
                id: id.clone(),
                plugin: source.plugins().get_required(id)?.as_ref().clone(),
            },
        );

        Ok(())
    };

    if config.import_plugins {
        for id in source.plugins().keys() {
            take_plugin(id)?;
        }
    }

    if let Some(kind) = &config.import_components {
        // Taking them in as external means taking the ids only: their
        // configuration stays with the project that owns them.
        let external = matches!(kind, ComponentsImportType::External);

        for (id, component) in source.components() {
            if component.external {
                continue;
            }

            take_plugin(&component.plugin)?;

            components.push(ComponentImport {
                id: id.clone(),
                plugin: component.plugin.clone(),
                external,
                config: (!external).then(|| component.config.clone()),
            });
        }
    }

    Ok(ImportData {
        plugins: plugins.into_values().collect(),
        components,
    })
}

/// A plugin as the registry describes it, in the model's own terms.
pub fn plugin_from_metadata(
    instance: &InstanceId,
    metadata: &PluginMetadata,
) -> Result<PluginImport, CoreModelError> {
    Ok(PluginImport {
        id: PluginId::build(instance, metadata.module(), metadata.name())?,
        plugin: Plugin {
            name: metadata.name().to_owned(),
            module: metadata.module().to_owned(),
            usage: usage(metadata.usage()),
            version: metadata.version().to_owned(),
            description: metadata.description().map(String::from),
            members: metadata
                .members()
                .iter()
                .map(|(name, member)| (name.clone(), self::member(member)))
                .collect(),
            config: metadata
                .config()
                .iter()
                .map(|(name, item)| (name.clone(), config_item(item)))
                .collect(),
            instance_name: instance.clone(),
            toolbox_display: ToolboxDisplay::Show,
        },
    })
}

fn usage(usage: metadata::PluginUsage) -> component_model::PluginUsage {
    match usage {
        metadata::PluginUsage::Sensor => component_model::PluginUsage::Sensor,
        metadata::PluginUsage::Actuator => component_model::PluginUsage::Actuator,
        metadata::PluginUsage::Logic => component_model::PluginUsage::Logic,
        metadata::PluginUsage::Ui => component_model::PluginUsage::Ui,
    }
}

fn member(member: &metadata::Member) -> Member {
    Member {
        description: member.description().map(String::from),
        member_type: match member.member_type() {
            metadata::MemberType::State => component_model::MemberType::State,
            metadata::MemberType::Action => component_model::MemberType::Action,
        },
        value_type: member.value_type().to_string(),
    }
}

fn config_item(item: &metadata::ConfigItem) -> ConfigItem {
    ConfigItem {
        description: item.description().map(String::from),
        value_type: match item.value_type() {
            metadata::ConfigType::String => component_model::ConfigType::String,
            metadata::ConfigType::Bool => component_model::ConfigType::Bool,
            metadata::ConfigType::Integer => component_model::ConfigType::Integer,
            metadata::ConfigType::Float => component_model::ConfigType::Float,
        },
    }
}
