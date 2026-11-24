//! Writing the picked subset of an import.

use std::collections::BTreeSet;

use studio_web_api::project_manager::BulkUpdatesStats;

use super::super::model::{
    BindingId, Change, ComponentId, Config, CoreModelError, ExportId, ExportKind, Plugin, PluginId,
    Project, TemplateId, ViewKey,
};
use super::update::{Action, Update, order_of};

/// Apply the writes the selection asks for, in an order where each one's
/// ground has been prepared by the ones before it.
pub fn apply_updates(
    project: &Project,
    updates: &[Update],
    selection: &BTreeSet<String>,
    changes: &mut Vec<Change>,
) -> Result<(Project, BulkUpdatesStats), CoreModelError> {
    let mut picked: Vec<&Update> = updates
        .iter()
        .filter(|update| {
            update
                .object_change_keys
                .iter()
                .any(|key| selection.contains(key))
        })
        .collect();

    picked.sort_by_key(|update| order_of(&update.action));

    let mut state = State::default();
    let mut project = project.clone();

    for update in picked {
        project = apply(project, &update.action, &mut state, changes)?;
    }

    Ok((project, state.stats()))
}

#[derive(Default)]
struct State {
    /// Components already set or cleared outright. A config write that came
    /// along with a plugin change has nothing left to do to one of these.
    settled: BTreeSet<String>,
    plugins: BTreeSet<String>,
    components: BTreeSet<String>,
    templates: BTreeSet<String>,
    bindings: BTreeSet<String>,
}

impl State {
    fn stats(&self) -> BulkUpdatesStats {
        BulkUpdatesStats {
            plugins: self.plugins.len() as i32,
            components: self.components.len() as i32,
            templates: self.templates.len() as i32,
            bindings: self.bindings.len() as i32,
        }
    }
}

fn apply(
    project: Project,
    action: &Action,
    state: &mut State,
    changes: &mut Vec<Change>,
) -> Result<Project, CoreModelError> {
    match action {
        Action::PluginSet { id, plugin } => {
            let id = PluginId::new(id)?;
            state.plugins.insert(id.to_string());

            Ok(project.import_plugin(changes, id, Plugin::try_from(plugin)?))
        }

        Action::PluginClear { id } => {
            let id = PluginId::new(id)?;
            state.plugins.insert(id.to_string());

            Ok(project.clear_plugin(changes, &id)?.unwrap_or(project))
        }

        Action::ComponentSet {
            id,
            plugin,
            external,
            config,
        } => {
            let id = ComponentId::new(id)?;
            state.settled.insert(id.to_string());
            state.components.insert(format!(":{id}"));

            project.import_component(
                changes,
                id,
                PluginId::new(plugin)?,
                *external,
                config
                    .as_ref()
                    .map(|config| Config::new(config.clone().into_iter().collect())),
            )
        }

        Action::ComponentClear {
            template_id,
            component_id,
        } => {
            let view = view_of(template_id)?;
            let id = ComponentId::new(component_id)?;

            state.settled.insert(id.to_string());
            state.components.insert(format!(":{id}"));

            project.clear_components(changes, &view, &[id])
        }

        Action::ComponentResetConfig {
            template_id,
            component_id,
            config_id,
        } => {
            if template_id.is_empty() && state.settled.contains(component_id) {
                return Ok(project);
            }

            let view = view_of(template_id)?;
            let id = ComponentId::new(component_id)?;
            state.components.insert(format!("{template_id}:{id}"));

            project.import_reset_component_config(changes, &view, id, config_id)
        }

        Action::ComponentClearConfig {
            template_id,
            component_id,
            config_id,
        } => {
            if template_id.is_empty() && state.settled.contains(component_id) {
                return Ok(project);
            }

            let view = view_of(template_id)?;
            let id = ComponentId::new(component_id)?;
            state.components.insert(format!("{template_id}:{id}"));

            project.import_clear_component_config(changes, &view, id, config_id)
        }

        Action::BindingClear {
            template_id,
            binding_id,
        } => {
            let view = view_of(template_id)?;
            let id = BindingId::new(binding_id)?;
            state.bindings.insert(format!("{template_id}:{id}"));

            Ok(project
                .clear_binding(changes, &view, &id)?
                .unwrap_or(project))
        }

        Action::TemplateClearExport {
            template_id,
            export_kind,
            export_id,
        } => {
            let template = TemplateId::new(template_id)?;
            state.templates.insert(template.to_string());

            project.import_clear_template_export(
                changes,
                &template,
                ExportKind::parse(export_kind)?,
                &ExportId::new(export_id)?,
            )
        }
    }
}

fn view_of(template_id: &str) -> Result<ViewKey, CoreModelError> {
    if template_id.is_empty() {
        return Ok(ViewKey::Project);
    }

    Ok(ViewKey::Template(TemplateId::new(template_id)?))
}
