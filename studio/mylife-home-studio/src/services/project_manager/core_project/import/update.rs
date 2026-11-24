//! What each accepted change actually writes, including everything it drags
//! along.
//!
//! Removing a plugin takes the components using it, which takes their bindings
//! and the template exports pointing at them. Working that out here, once,
//! before the designer is asked, is what lets the answer be shown and then
//! carried out exactly.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};
use studio_web_api::project_manager;
use studio_web_api::project_manager::core_import_data::{
    BindingDeleteImpact, ChangeType, ComponentConfigImpact, ComponentDeleteImpact, Impact,
    ObjectChange, TemplateExportImpact,
};

use super::super::model::{
    Binding, BindingId, ComponentDefinition, ComponentId, CoreModelError, ExportKind, Id,
    NamingDryRun, Project, TemplateId, ViewKey,
};
use super::load::ImportData;

/// One write, and which accepted changes call for it.
///
/// Travels to the designer and back untouched, so it is serde-shaped.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Update {
    pub id: String,
    /// The changes that need this write. A write stays out unless one of them
    /// was picked.
    pub object_change_keys: Vec<String>,
    pub dependencies: Vec<String>,
    pub action: Action,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Action {
    PluginSet {
        id: String,
        plugin: project_manager::CorePluginData,
    },
    PluginClear {
        id: String,
    },
    ComponentSet {
        id: String,
        plugin: String,
        external: bool,
        config: Option<HashMap<String, serde_json::Value>>,
    },
    ComponentClear {
        template_id: String,
        component_id: String,
    },
    ComponentResetConfig {
        template_id: String,
        component_id: String,
        config_id: String,
    },
    ComponentClearConfig {
        template_id: String,
        component_id: String,
        config_id: String,
    },
    BindingClear {
        template_id: String,
        binding_id: String,
    },
    TemplateClearExport {
        template_id: String,
        export_kind: String,
        export_id: String,
    },
}

/// The order writes can be made in, each line saying what it waits for.
const ORDER: &[&str] = &[
    "binding-clear",          // nothing depends on it
    "plugin-set",             // before a component can point at it
    "component-set",          // before plugin-clear, in case the plugin moved
    "template-clear-export",  // nothing checks export data
    "component-clear-config", // nothing checks config data
    "component-reset-config", // needs the template and plugin writes above
    "component-clear",        // may be named by an export, so after the exports
    "plugin-clear",           // after the components using it are gone
];

pub fn order_of(action: &Action) -> usize {
    let name = match action {
        Action::BindingClear { .. } => "binding-clear",
        Action::PluginSet { .. } => "plugin-set",
        Action::ComponentSet { .. } => "component-set",
        Action::TemplateClearExport { .. } => "template-clear-export",
        Action::ComponentClearConfig { .. } => "component-clear-config",
        Action::ComponentResetConfig { .. } => "component-reset-config",
        Action::ComponentClear { .. } => "component-clear",
        Action::PluginClear { .. } => "plugin-clear",
    };

    ORDER.iter().position(|item| *item == name).expect("listed")
}

/// Work out every write the changes call for, and fill in what each change
/// drags along.
pub fn build_updates(
    imports: &ImportData,
    project: &Project,
    changes: &mut [ObjectChange],
) -> Result<Vec<Update>, CoreModelError> {
    let mut builder = Builder {
        project,
        updates: Vec::new(),
        index: HashMap::new(),
        current_key: String::new(),
    };

    for change in changes.iter() {
        builder.current_key = change.key().to_owned();
        builder.change(imports, change)?;
    }

    // A change that needs another one also needs its writes.
    for change in changes.iter() {
        for source in change.dependencies() {
            builder.copy_key(source, change.key());
        }
    }

    let updates = builder.updates;

    for change in changes.iter_mut() {
        let key = change.key().to_owned();
        *change.impacts_mut() = impacts(&key, change, &updates);
    }

    Ok(updates)
}

struct Builder<'a> {
    project: &'a Project,
    updates: Vec<Update>,
    index: HashMap<String, usize>,
    current_key: String,
}

impl Builder<'_> {
    fn change(
        &mut self,
        imports: &ImportData,
        change: &ObjectChange,
    ) -> Result<(), CoreModelError> {
        match change {
            ObjectChange::Component(component) => {
                let id = ComponentId::new(&component.id)?;

                match component.change_type {
                    ChangeType::Delete => {
                        let view = self.view_of(&id)?;
                        self.component_clear(&view, &id)?;
                    }
                    _ => {
                        if let Some(import) = imports.component(&id) {
                            self.component_set(import)?;
                        }
                    }
                }
            }

            ObjectChange::Plugin(plugin) => {
                let id = super::super::model::PluginId::new(&plugin.id)?;

                match plugin.change_type {
                    ChangeType::Delete => {
                        self.plugin_clear(&id)?;
                    }
                    _ => {
                        if let Some(import) = imports.plugin(&id) {
                            self.plugin_set(import, plugin)?;
                        }
                    }
                }
            }

            ObjectChange::Template(template) => {
                let template_id = TemplateId::new(&template.id)?;
                let kind = ExportKind::parse(&template.export_type)?;
                let export_id = super::super::model::ExportId::new(&template.export_id)?;

                // Removing the export is the root operation here, so the
                // component's own config goes back to its default rather than
                // being reset through the template.
                if let ExportKind::Config = kind {
                    let exports = &self.project.templates().get_required(&template_id)?.exports;
                    let export = exports.config_export(&export_id)?;
                    let component = export.component.clone();
                    let config = export.config_name.clone();

                    self.component_reset_config_direct(
                        &ViewKey::Template(template_id.clone()),
                        &component,
                        &config,
                    )?;
                }

                self.template_export_clear(&template_id, kind, &export_id)?;
            }
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // The writes
    // -----------------------------------------------------------------------

    fn plugin_set(
        &mut self,
        import: &super::load::PluginImport,
        change: &project_manager::core_import_data::PluginChange,
    ) -> Result<Vec<String>, CoreModelError> {
        let id = format!("plugin-set:{}", import.id);

        if self.index.contains_key(&id) {
            self.add_key(&id);
            return Ok(vec![id]);
        }

        let mut dependencies = Vec::new();

        if matches!(change.change_type, ChangeType::Update) {
            let usage = self
                .project
                .definition_usage(&ComponentDefinition::Plugin(import.id.clone()));

            for (name, change) in &change.config {
                for (view, component, _) in &usage {
                    dependencies.extend(match change {
                        ChangeType::Delete => self.component_clear_config(view, component, name)?,
                        _ => self.component_reset_config(view, component, name)?,
                    });
                }
            }

            for (name, change) in &change.members {
                for (view, component, _) in &usage {
                    dependencies.extend(match change {
                        ChangeType::Delete => self.component_clear_member(view, component, name)?,
                        // A member whose type moved breaks what is bound to it.
                        ChangeType::Update => {
                            self.component_update_member(view, component, name)?
                        }
                        ChangeType::Add => Vec::new(),
                    });
                }
            }
        }

        self.push(
            id.clone(),
            dependencies,
            Action::PluginSet {
                id: import.id.to_string(),
                plugin: project_manager::CorePluginData::from(&import.plugin),
            },
        );

        Ok(vec![id])
    }

    fn plugin_clear(
        &mut self,
        plugin: &super::super::model::PluginId,
    ) -> Result<Vec<String>, CoreModelError> {
        let id = format!("plugin-clear:{plugin}");

        if self.index.contains_key(&id) {
            self.add_key(&id);
            return Ok(vec![id]);
        }

        let mut dependencies = Vec::new();

        for (view, component, _) in self
            .project
            .definition_usage(&ComponentDefinition::Plugin(plugin.clone()))
        {
            dependencies.extend(self.component_clear(&view, &component)?);
        }

        self.push(
            id.clone(),
            dependencies,
            Action::PluginClear {
                id: plugin.to_string(),
            },
        );

        Ok(vec![id])
    }

    fn component_set(
        &mut self,
        import: &super::load::ComponentImport,
    ) -> Result<Vec<String>, CoreModelError> {
        // A new component must not collide with an id a template would
        // generate. One that would is left out rather than refused: the rest
        // of the import is still worth doing.
        if self
            .project
            .view(&ViewKey::Project)?
            .components
            .get(&import.id)
            .is_none()
        {
            let mut dry_run = NamingDryRun::new(self.project);
            dry_run.set_component(
                &ViewKey::Project,
                import.id.clone(),
                &ComponentDefinition::Plugin(import.plugin.clone()),
            );

            if dry_run.validate().is_err() {
                return Ok(Vec::new());
            }
        }

        let id = format!("component-set::{}", import.id);

        if !self.index.contains_key(&id) {
            self.push(
                id.clone(),
                Vec::new(),
                Action::ComponentSet {
                    id: import.id.to_string(),
                    plugin: import.plugin.to_string(),
                    external: import.external,
                    config: import
                        .config
                        .as_ref()
                        .map(|config| config.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
                },
            );
        } else {
            self.add_key(&id);
        }

        Ok(vec![id])
    }

    fn component_clear(
        &mut self,
        view: &ViewKey,
        component: &ComponentId,
    ) -> Result<Vec<String>, CoreModelError> {
        let id = format!("component-clear:{}:{component}", template_of(view));

        if self.index.contains_key(&id) {
            self.add_key(&id);
            return Ok(vec![id]);
        }

        let mut dependencies = Vec::new();

        for (binding_id, _) in self.project.view(view)?.bindings_of(component) {
            dependencies.extend(self.binding_clear(view, &binding_id)?);
        }

        for (kind, export_id) in self.exports_naming(view, component, None)? {
            dependencies.extend(self.template_export_clear_in(view, kind, &export_id)?);
        }

        self.push(
            id.clone(),
            dependencies,
            Action::ComponentClear {
                template_id: template_of(view),
                component_id: component.to_string(),
            },
        );

        Ok(vec![id])
    }

    fn component_clear_config(
        &mut self,
        view: &ViewKey,
        component: &ComponentId,
        config_id: &str,
    ) -> Result<Vec<String>, CoreModelError> {
        let id = format!(
            "component-clear-config:{}:{component}:{config_id}",
            template_of(view)
        );

        if self.index.contains_key(&id) {
            self.add_key(&id);
            return Ok(vec![id]);
        }

        let mut dependencies = Vec::new();

        for export_id in self.config_exports_of(view, component, Some(config_id))? {
            dependencies.extend(self.template_export_clear_in(
                view,
                ExportKind::Config,
                &export_id,
            )?);
        }

        self.push(
            id.clone(),
            dependencies,
            Action::ComponentClearConfig {
                template_id: template_of(view),
                component_id: component.to_string(),
                config_id: config_id.to_owned(),
            },
        );

        Ok(vec![id])
    }

    /// An exported config item is not the component's to reset: whoever
    /// instantiates the template supplies it, so they are what gets reset.
    fn component_reset_config(
        &mut self,
        view: &ViewKey,
        component: &ComponentId,
        config_id: &str,
    ) -> Result<Vec<String>, CoreModelError> {
        if let ViewKey::Template(template_id) = view {
            let exported = self
                .config_exports_of(view, component, Some(config_id))?
                .into_iter()
                .next();

            if let Some(export_id) = exported {
                let mut dependencies = Vec::new();

                for (usage_view, usage, _) in self
                    .project
                    .definition_usage(&ComponentDefinition::Template(template_id.clone()))
                {
                    dependencies.extend(self.component_reset_config(
                        &usage_view,
                        &usage,
                        export_id.as_str(),
                    )?);
                }

                return Ok(dependencies);
            }
        }

        self.component_reset_config_direct(view, component, config_id)
    }

    fn component_reset_config_direct(
        &mut self,
        view: &ViewKey,
        component: &ComponentId,
        config_id: &str,
    ) -> Result<Vec<String>, CoreModelError> {
        let id = format!(
            "component-reset-config:{}:{component}:{config_id}",
            template_of(view)
        );

        if self.index.contains_key(&id) {
            self.add_key(&id);
            return Ok(vec![id]);
        }

        self.push(
            id.clone(),
            Vec::new(),
            Action::ComponentResetConfig {
                template_id: template_of(view),
                component_id: component.to_string(),
                config_id: config_id.to_owned(),
            },
        );

        Ok(vec![id])
    }

    /// Not a write of its own: a member that moved or went breaks whatever was
    /// bound to it.
    fn component_update_member(
        &mut self,
        view: &ViewKey,
        component: &ComponentId,
        member: &str,
    ) -> Result<Vec<String>, CoreModelError> {
        let mut dependencies = Vec::new();

        for binding_id in self.bindings_with_member(view, component, member)? {
            dependencies.extend(self.binding_clear(view, &binding_id)?);
        }

        if let ViewKey::Template(template_id) = view {
            for export_id in self.member_exports_of(view, component, Some(member))? {
                for (usage_view, usage, _) in self
                    .project
                    .definition_usage(&ComponentDefinition::Template(template_id.clone()))
                {
                    dependencies.extend(self.component_update_member(
                        &usage_view,
                        &usage,
                        export_id.as_str(),
                    )?);
                }
            }
        }

        Ok(dependencies)
    }

    fn component_clear_member(
        &mut self,
        view: &ViewKey,
        component: &ComponentId,
        member: &str,
    ) -> Result<Vec<String>, CoreModelError> {
        let mut dependencies = Vec::new();

        for binding_id in self.bindings_with_member(view, component, member)? {
            dependencies.extend(self.binding_clear(view, &binding_id)?);
        }

        for export_id in self.member_exports_of(view, component, Some(member))? {
            dependencies.extend(self.template_export_clear_in(
                view,
                ExportKind::Member,
                &export_id,
            )?);
        }

        Ok(dependencies)
    }

    fn binding_clear(
        &mut self,
        view: &ViewKey,
        binding: &BindingId,
    ) -> Result<Vec<String>, CoreModelError> {
        let id = format!("binding-clear:{}:{binding}", template_of(view));

        if self.index.contains_key(&id) {
            self.add_key(&id);
            return Ok(vec![id]);
        }

        self.push(
            id.clone(),
            Vec::new(),
            Action::BindingClear {
                template_id: template_of(view),
                binding_id: binding.to_string(),
            },
        );

        Ok(vec![id])
    }

    fn template_export_clear_in(
        &mut self,
        view: &ViewKey,
        kind: ExportKind,
        export_id: &super::super::model::ExportId,
    ) -> Result<Vec<String>, CoreModelError> {
        let ViewKey::Template(template_id) = view else {
            return Ok(Vec::new());
        };

        let template_id = template_id.clone();
        self.template_export_clear(&template_id, kind, export_id)
    }

    fn template_export_clear(
        &mut self,
        template_id: &TemplateId,
        kind: ExportKind,
        export_id: &super::super::model::ExportId,
    ) -> Result<Vec<String>, CoreModelError> {
        let id = format!(
            "template-clear-export:{template_id}:{}:{export_id}",
            kind.as_str()
        );

        if self.index.contains_key(&id) {
            self.add_key(&id);
            return Ok(vec![id]);
        }

        let mut dependencies = Vec::new();

        for (usage_view, usage, _) in self
            .project
            .definition_usage(&ComponentDefinition::Template(template_id.clone()))
        {
            dependencies.extend(match kind {
                ExportKind::Config => {
                    self.component_clear_config(&usage_view, &usage, export_id.as_str())?
                }
                ExportKind::Member => {
                    self.component_clear_member(&usage_view, &usage, export_id.as_str())?
                }
            });
        }

        self.push(
            id.clone(),
            dependencies,
            Action::TemplateClearExport {
                template_id: template_id.to_string(),
                export_kind: String::from(kind.as_str()),
                export_id: export_id.to_string(),
            },
        );

        Ok(vec![id])
    }

    // -----------------------------------------------------------------------
    // Looking around the project
    // -----------------------------------------------------------------------

    fn view_of(&self, component: &ComponentId) -> Result<ViewKey, CoreModelError> {
        for view in self.project.views() {
            if view.components.contains(component) {
                return Ok(view.key);
            }
        }

        Err(CoreModelError::not_found(
            super::super::model::IdKind::Component,
            component,
        ))
    }

    fn bindings_with_member(
        &self,
        view: &ViewKey,
        component: &ComponentId,
        member: &str,
    ) -> Result<Vec<BindingId>, CoreModelError> {
        Ok(self
            .project
            .view(view)?
            .bindings_of(component)
            .into_iter()
            .filter(|(_, binding)| mentions_member(binding, component, member))
            .map(|(id, _)| id)
            .collect())
    }

    fn config_exports_of(
        &self,
        view: &ViewKey,
        component: &ComponentId,
        config_id: Option<&str>,
    ) -> Result<Vec<super::super::model::ExportId>, CoreModelError> {
        let ViewKey::Template(template_id) = view else {
            return Ok(Vec::new());
        };

        let exports = &self.project.templates().get_required(template_id)?.exports;

        let mut found: Vec<_> = exports
            .config
            .iter()
            .filter(|(_, export)| {
                &export.component == component
                    && config_id.is_none_or(|name| export.config_name == name)
            })
            .map(|(id, _)| id.clone())
            .collect();

        found.sort();
        Ok(found)
    }

    fn member_exports_of(
        &self,
        view: &ViewKey,
        component: &ComponentId,
        member: Option<&str>,
    ) -> Result<Vec<super::super::model::ExportId>, CoreModelError> {
        let ViewKey::Template(template_id) = view else {
            return Ok(Vec::new());
        };

        let exports = &self.project.templates().get_required(template_id)?.exports;

        let mut found: Vec<_> = exports
            .members
            .iter()
            .filter(|(_, export)| {
                &export.component == component && member.is_none_or(|name| export.member == name)
            })
            .map(|(id, _)| id.clone())
            .collect();

        found.sort();
        Ok(found)
    }

    /// Every export of either kind naming this component.
    fn exports_naming(
        &self,
        view: &ViewKey,
        component: &ComponentId,
        member: Option<&str>,
    ) -> Result<Vec<(ExportKind, super::super::model::ExportId)>, CoreModelError> {
        let mut found: Vec<_> = self
            .config_exports_of(view, component, None)?
            .into_iter()
            .map(|id| (ExportKind::Config, id))
            .collect();

        found.extend(
            self.member_exports_of(view, component, member)?
                .into_iter()
                .map(|id| (ExportKind::Member, id)),
        );

        Ok(found)
    }

    // -----------------------------------------------------------------------
    // Bookkeeping
    // -----------------------------------------------------------------------

    fn push(&mut self, id: String, dependencies: Vec<String>, action: Action) {
        self.index.insert(id.clone(), self.updates.len());
        self.updates.push(Update {
            id: id.clone(),
            object_change_keys: Vec::new(),
            dependencies,
            action,
        });

        self.add_key(&id);
    }

    /// Mark this write, and everything it waits for, as called for by the
    /// change being computed.
    fn add_key(&mut self, id: &str) {
        let Some(index) = self.index.get(id).copied() else {
            return;
        };

        let key = self.current_key.clone();

        if self.updates[index].object_change_keys.contains(&key) {
            return;
        }

        self.updates[index].object_change_keys.push(key);

        for dependency in self.updates[index].dependencies.clone() {
            self.add_key(&dependency);
        }
    }

    fn copy_key(&mut self, source: &str, target: &str) {
        for update in &mut self.updates {
            if update.object_change_keys.iter().any(|key| key == source)
                && !update.object_change_keys.iter().any(|key| key == target)
            {
                update.object_change_keys.push(target.to_owned());
            }
        }
    }
}

fn template_of(view: &ViewKey) -> String {
    match view {
        ViewKey::Project => String::new(),
        ViewKey::Template(id) => id.to_string(),
    }
}

fn mentions_member(binding: &Binding, component: &ComponentId, member: &str) -> bool {
    (&binding.source_component == component && binding.source_state == member)
        || (&binding.target_component == component && binding.target_action == member)
}

/// What accepting one change drags along, as the designer shows it.
///
/// The change's own write is not an impact: it is the thing being accepted.
fn impacts(key: &str, change: &ObjectChange, updates: &[Update]) -> Vec<Impact> {
    let mut config: BTreeMap<(String, String), ComponentConfigImpact> = BTreeMap::new();
    let mut found = Vec::new();
    let mut exports: BTreeMap<String, TemplateExportImpact> = BTreeMap::new();

    for update in updates {
        if !update.object_change_keys.iter().any(|item| item == key) {
            continue;
        }

        match &update.action {
            Action::PluginSet { .. } | Action::PluginClear { .. } | Action::ComponentSet { .. } => {
            }

            Action::ComponentClear {
                template_id,
                component_id,
            } => {
                if template_id.is_empty() && is_root(change, ChangeType::Delete, component_id) {
                    continue;
                }

                found.push(Impact::ComponentDelete(ComponentDeleteImpact {
                    template_id: template_id.clone(),
                    component_id: component_id.clone(),
                }));
            }

            Action::ComponentResetConfig {
                template_id,
                component_id,
                config_id,
            }
            | Action::ComponentClearConfig {
                template_id,
                component_id,
                config_id,
            } => {
                let entry = config
                    .entry((template_id.clone(), component_id.clone()))
                    .or_insert_with(|| ComponentConfigImpact {
                        template_id: template_id.clone(),
                        component_id: component_id.clone(),
                        config: HashMap::new(),
                    });

                let change = match update.action {
                    Action::ComponentClearConfig { .. } => ChangeType::Delete,
                    _ => ChangeType::Update,
                };

                entry.config.insert(config_id.clone(), change);
            }

            Action::BindingClear {
                template_id,
                binding_id,
            } => found.push(Impact::BindingDelete(BindingDeleteImpact {
                template_id: template_id.clone(),
                binding_id: binding_id.clone(),
            })),

            Action::TemplateClearExport {
                template_id,
                export_kind,
                export_id,
            } => {
                if is_root_template(change, template_id) {
                    continue;
                }

                let entry =
                    exports
                        .entry(template_id.clone())
                        .or_insert_with(|| TemplateExportImpact {
                            template_id: template_id.clone(),
                            config_export_deletes: Vec::new(),
                            member_export_deletes: Vec::new(),
                        });

                let list = match export_kind.as_str() {
                    "config" => &mut entry.config_export_deletes,
                    _ => &mut entry.member_export_deletes,
                };

                if !list.contains(export_id) {
                    list.push(export_id.clone());
                }
            }
        }
    }

    let mut all: Vec<Impact> = config.into_values().map(Impact::ComponentConfig).collect();
    all.extend(found);
    all.extend(exports.into_values().map(Impact::TemplateExport));
    all
}

fn is_root(change: &ObjectChange, change_type: ChangeType, id: &str) -> bool {
    match change {
        ObjectChange::Component(component) => {
            component.id == id
                && std::mem::discriminant(&component.change_type)
                    == std::mem::discriminant(&change_type)
        }
        _ => false,
    }
}

fn is_root_template(change: &ObjectChange, template_id: &str) -> bool {
    matches!(change, ObjectChange::Template(template) if template.id == template_id)
}
