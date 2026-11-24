use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use studio_web_api::component_model::{Member, MemberType};

use super::Collection;
use super::Id;
use super::change::Change;
use super::ids::{BindingId, ComponentId, ExportId, InstanceId, PluginId, TemplateId};
use super::naming::NamingDryRun;
use super::types::{
    Binding, Component, ComponentDefinition, Config, ConfigExport, ExportKind, MemberExport,
    Plugin, Position, Template, TemplateExports, ToolboxDisplay, check_config_value,
    member_type_name,
};
use super::view::{View, ViewKey};

use super::error::CoreModelError;
use super::ids::IdKind;

/// How deep template nesting may go before we call the data malformed.
const MAX_DEFINITION_DEPTH: usize = 32;

/// The core project.
///
/// Its own components and bindings sit in `content`; a template holds a second
/// set of the same shape. `View` is what lets an operation be written once and
/// run against either.
#[derive(Debug, Clone, Default)]
pub struct Project {
    components: Collection<ComponentId, Component>,
    bindings: Collection<BindingId, Binding>,
    plugins: Collection<PluginId, Plugin>,
    templates: Collection<TemplateId, Template>,
}

impl Project {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_parts(
        components: Collection<ComponentId, Component>,
        bindings: Collection<BindingId, Binding>,
        plugins: Collection<PluginId, Plugin>,
        templates: Collection<TemplateId, Template>,
    ) -> Self {
        Self {
            components,
            bindings,
            plugins,
            templates,
        }
    }

    pub fn components(&self) -> &Collection<ComponentId, Component> {
        &self.components
    }

    pub fn bindings(&self) -> &Collection<BindingId, Binding> {
        &self.bindings
    }

    pub fn plugins(&self) -> &Collection<PluginId, Plugin> {
        &self.plugins
    }

    pub fn templates(&self) -> &Collection<TemplateId, Template> {
        &self.templates
    }

    // -----------------------------------------------------------------------
    // Views
    // -----------------------------------------------------------------------

    /// The project view first, then one per template, in id order.
    pub fn view_keys(&self) -> Vec<ViewKey> {
        let mut templates: Vec<&TemplateId> = self.templates.keys().collect();
        templates.sort();

        let mut keys = vec![ViewKey::Project];
        keys.extend(
            templates
                .into_iter()
                .map(|id| ViewKey::Template(id.clone())),
        );
        keys
    }

    pub fn views(&self) -> impl Iterator<Item = View> + '_ {
        std::iter::once(View {
            key: ViewKey::Project,
            components: self.components.clone(),
            bindings: self.bindings.clone(),
        })
        .chain(self.templates.iter().map(|(id, template)| View {
            key: ViewKey::Template(id.clone()),
            components: template.components.clone(),
            bindings: template.bindings.clone(),
        }))
    }

    pub fn view(&self, key: &ViewKey) -> Result<View, CoreModelError> {
        let (components, bindings) = match key {
            ViewKey::Project => (self.components.clone(), self.bindings.clone()),
            ViewKey::Template(id) => {
                let template = self.templates.get_required(id)?;
                (template.components.clone(), template.bindings.clone())
            }
        };

        Ok(View {
            key: key.clone(),
            components,
            bindings,
        })
    }

    /// Put a view back where its key says it belongs. Unlike the UI project
    /// this emits nothing: the core designer is told about individual
    /// components and bindings, so the operations emit as they go.
    ///
    /// A template's exports are not carried through a view, so they are read
    /// back here and left as they were. An operation that changes them says so
    /// separately.
    pub fn with_view_set(&self, view: View) -> Result<Self, CoreModelError> {
        let View {
            key,
            components,
            bindings,
        } = view;

        let mut project = self.clone();

        match key {
            ViewKey::Project => {
                project.components = components;
                project.bindings = bindings;
            }
            ViewKey::Template(id) => {
                let mut template = self.templates.get_required(&id)?.as_ref().clone();
                template.components = components;
                template.bindings = bindings;
                project.templates = project.templates.set(id, Arc::new(template));
            }
        }

        Ok(project)
    }

    // -----------------------------------------------------------------------
    // Instances and plugins
    // -----------------------------------------------------------------------

    /// Instances are not stored; they are the distinct `instance_name`s of the
    /// plugins. Deriving them means there is no second structure to keep in
    /// step with the plugin list.
    pub fn instances(&self) -> BTreeMap<InstanceId, Vec<PluginId>> {
        let mut instances: BTreeMap<InstanceId, Vec<PluginId>> = BTreeMap::new();

        for (id, plugin) in &self.plugins {
            instances
                .entry(plugin.instance_name.clone())
                .or_default()
                .push(id.clone());
        }

        for plugins in instances.values_mut() {
            plugins.sort();
        }

        instances
    }

    pub fn instance_plugins(&self, instance: &InstanceId) -> Result<Vec<PluginId>, CoreModelError> {
        self.instances()
            .remove(instance)
            .ok_or_else(|| CoreModelError::not_found(IdKind::Instance, instance))
    }

    /// Whether any component anywhere instantiates this definition.
    pub fn definition_used(&self, definition: &ComponentDefinition) -> bool {
        self.views().any(|view| {
            view.components
                .values()
                .any(|component| &component.definition == definition)
        })
    }

    /// Every component instantiating this definition, with the view it is in.
    pub fn definition_usage(
        &self,
        definition: &ComponentDefinition,
    ) -> Vec<(ViewKey, ComponentId, Arc<Component>)> {
        let mut usage = Vec::new();

        for view in self.views() {
            let key = view.key.clone();

            for (id, component) in &view.components {
                if &component.definition == definition {
                    usage.push((key.clone(), id.clone(), component.clone()));
                }
            }
        }

        usage.sort_by(|left, right| (&left.0, &left.1).cmp(&(&right.0, &right.1)));
        usage
    }

    // -----------------------------------------------------------------------
    // Resolving a definition
    //
    // A plugin answers these directly. A template answers them through its
    // exports, which point at one of its own components, whose definition may
    // be another template. The recursion is the reason these live on `Project`
    // rather than on the types.
    // -----------------------------------------------------------------------

    pub fn definition_member(
        &self,
        definition: &ComponentDefinition,
        name: &str,
    ) -> Result<Member, CoreModelError> {
        self.definition_member_at(definition, name, 0)
    }

    fn definition_member_at(
        &self,
        definition: &ComponentDefinition,
        name: &str,
        depth: usize,
    ) -> Result<Member, CoreModelError> {
        self.check_depth(depth)?;

        match definition {
            ComponentDefinition::Plugin(id) => {
                Ok(self.plugins.get_required(id)?.member(name)?.clone())
            }
            ComponentDefinition::Template(id) => {
                let template = self.templates.get_required(id)?;
                let export = template.exports.member_export(&ExportId::new(name)?)?;
                let component = template.components.get_required(&export.component)?;

                self.definition_member_at(&component.definition, &export.member, depth + 1)
            }
        }
    }

    pub fn definition_member_value_type(
        &self,
        definition: &ComponentDefinition,
        name: &str,
        member_type: MemberType,
    ) -> Result<String, CoreModelError> {
        let member = self.definition_member(definition, name)?;

        if member.member_type != member_type {
            return Err(CoreModelError::MemberNotFound {
                component: definition_name(definition),
                member: name.to_owned(),
                member_type: member_type_name(member_type),
            });
        }

        Ok(member.value_type)
    }

    pub fn definition_config_default(
        &self,
        definition: &ComponentDefinition,
        name: &str,
    ) -> Result<serde_json::Value, CoreModelError> {
        self.definition_config_default_at(definition, name, 0)
    }

    fn definition_config_default_at(
        &self,
        definition: &ComponentDefinition,
        name: &str,
        depth: usize,
    ) -> Result<serde_json::Value, CoreModelError> {
        self.check_depth(depth)?;

        match definition {
            ComponentDefinition::Plugin(id) => self.plugins.get_required(id)?.config_default(name),
            ComponentDefinition::Template(id) => {
                let template = self.templates.get_required(id)?;
                let export = template.exports.config_export(&ExportId::new(name)?)?;
                let component = template.components.get_required(&export.component)?;

                self.definition_config_default_at(
                    &component.definition,
                    &export.config_name,
                    depth + 1,
                )
            }
        }
    }

    pub fn definition_config_template(
        &self,
        definition: &ComponentDefinition,
    ) -> Result<Config, CoreModelError> {
        match definition {
            ComponentDefinition::Plugin(id) => self.plugins.get_required(id)?.config_template(),
            ComponentDefinition::Template(id) => {
                let template = self.templates.get_required(id)?;
                let mut values = HashMap::new();

                for export_id in template.exports.config.keys() {
                    values.insert(
                        export_id.to_string(),
                        self.definition_config_default(definition, export_id.as_str())?,
                    );
                }

                Ok(Config::new(values))
            }
        }
    }

    pub fn definition_check_config_value(
        &self,
        definition: &ComponentDefinition,
        name: &str,
        value: &serde_json::Value,
    ) -> Result<(), CoreModelError> {
        self.definition_check_config_value_at(definition, name, value, 0)
    }

    fn definition_check_config_value_at(
        &self,
        definition: &ComponentDefinition,
        name: &str,
        value: &serde_json::Value,
        depth: usize,
    ) -> Result<(), CoreModelError> {
        self.check_depth(depth)?;

        match definition {
            ComponentDefinition::Plugin(id) => {
                let plugin = self.plugins.get_required(id)?;
                let item = plugin.config_item(name)?;

                check_config_value(&plugin.to_string(), name, &item.value_type, value)
            }
            ComponentDefinition::Template(id) => {
                let template = self.templates.get_required(id)?;
                let export = template.exports.config_export(&ExportId::new(name)?)?;
                let component = template.components.get_required(&export.component)?;

                self.definition_check_config_value_at(
                    &component.definition,
                    &export.config_name,
                    value,
                    depth + 1,
                )
            }
        }
    }

    fn check_depth(&self, depth: usize) -> Result<(), CoreModelError> {
        if depth >= MAX_DEFINITION_DEPTH {
            return Err(CoreModelError::CircularTemplate {
                id: String::from("<nested too deep>"),
            });
        }

        Ok(())
    }

    fn check_definition_exists(
        &self,
        definition: &ComponentDefinition,
    ) -> Result<(), CoreModelError> {
        match definition {
            ComponentDefinition::Plugin(id) => self.plugins.get_required(id).map(|_| ()),
            ComponentDefinition::Template(id) => self.templates.get_required(id).map(|_| ()),
        }
    }

    // -----------------------------------------------------------------------
    // Toolbox
    // -----------------------------------------------------------------------

    pub fn set_plugin_display(
        &self,
        changes: &mut Vec<Change>,
        id: &PluginId,
        display: ToolboxDisplay,
    ) -> Result<Option<Self>, CoreModelError> {
        let Some(plugin) = self.plugins.get_required(id)?.with_display(display) else {
            return Ok(None);
        };

        let mut project = self.clone();
        project.plugins = self.plugins.set(id.clone(), plugin);

        changes.push(Change::SetPluginToolboxDisplay {
            id: id.clone(),
            display,
        });

        Ok(Some(project))
    }

    pub fn set_instance_display(
        &self,
        changes: &mut Vec<Change>,
        instance: &InstanceId,
        display: ToolboxDisplay,
    ) -> Result<Option<Self>, CoreModelError> {
        let mut project = self.clone();
        let mut changed = false;

        for id in self.instance_plugins(instance)? {
            if let Some(updated) = project.set_plugin_display(changes, &id, display)? {
                project = updated;
                changed = true;
            }
        }

        Ok(changed.then_some(project))
    }

    pub fn clear_plugin(
        &self,
        changes: &mut Vec<Change>,
        id: &PluginId,
    ) -> Result<Option<Self>, CoreModelError> {
        self.plugins.get_required(id)?;

        if self.definition_used(&ComponentDefinition::Plugin(id.clone())) {
            return Err(CoreModelError::in_use(IdKind::Plugin, id));
        }

        let Some(plugins) = self.plugins.removed(id) else {
            return Ok(None);
        };

        let mut project = self.clone();
        project.plugins = plugins;

        changes.push(Change::ClearPlugin { id: id.clone() });

        Ok(Some(project))
    }

    /// Drop every plugin of an instance that nothing uses. Plugins still in use
    /// are left alone, which is what makes this different from deleting one
    /// plugin by hand.
    pub fn clear_unused_instance_plugins(
        &self,
        changes: &mut Vec<Change>,
        instance: &InstanceId,
    ) -> Result<Option<Self>, CoreModelError> {
        let mut project = self.clone();
        let mut changed = false;

        for id in self.instance_plugins(instance)? {
            if self.definition_used(&ComponentDefinition::Plugin(id.clone())) {
                continue;
            }

            if let Some(updated) = project.clear_plugin(changes, &id)? {
                project = updated;
                changed = true;
            }
        }

        Ok(changed.then_some(project))
    }

    // -----------------------------------------------------------------------
    // Templates
    // -----------------------------------------------------------------------

    pub fn set_template(
        &self,
        changes: &mut Vec<Change>,
        id: TemplateId,
    ) -> Result<Self, CoreModelError> {
        let template = Arc::new(Template::empty());
        let templates = self.templates.inserted(id.clone(), template.clone())?;

        let mut project = self.clone();
        project.templates = templates;

        changes.push(Change::SetTemplate {
            id,
            exports: template.exports.clone(),
        });

        Ok(project)
    }

    pub fn clear_template(
        &self,
        changes: &mut Vec<Change>,
        id: &TemplateId,
    ) -> Result<Option<Self>, CoreModelError> {
        self.templates.get_required(id)?;

        if self.definition_used(&ComponentDefinition::Template(id.clone())) {
            return Err(CoreModelError::in_use(IdKind::Template, id));
        }

        let Some(templates) = self.templates.removed(id) else {
            return Ok(None);
        };

        let mut project = self.clone();
        project.templates = templates;

        // The template's own components and bindings go with it; the client
        // drops them on this one change.
        changes.push(Change::ClearTemplate { id: id.clone() });

        Ok(Some(project))
    }

    pub fn rename_template(
        &self,
        changes: &mut Vec<Change>,
        id: &TemplateId,
        new_id: TemplateId,
    ) -> Result<Option<Self>, CoreModelError> {
        let Some(templates) = self.templates.renamed(id, &new_id)? else {
            return Ok(None);
        };

        let mut project = self.clone();
        project.templates = templates;

        changes.push(Change::RenameTemplate {
            id: id.clone(),
            new_id: new_id.clone(),
        });

        // Every component that instantiates it now points at the new id.
        let old = ComponentDefinition::Template(id.clone());
        let new = ComponentDefinition::Template(new_id);

        for (view_key, component_id, component) in self.definition_usage(&old) {
            let mut updated = component.as_ref().clone();
            updated.definition = new.clone();
            let updated = Arc::new(updated);

            let view = project
                .view(&view_key)?
                .with_component_updated(component_id.clone(), updated.clone())?;
            project = project.with_view_set(view)?;

            changes.push(Change::SetComponent {
                view: view_key,
                id: component_id,
                component: updated,
            });
        }

        Ok(Some(project))
    }

    // -----------------------------------------------------------------------
    // Template exports
    // -----------------------------------------------------------------------

    pub fn set_template_export(
        &self,
        changes: &mut Vec<Change>,
        template_id: &TemplateId,
        kind: ExportKind,
        export_id: ExportId,
        component_id: &ComponentId,
        property: &str,
    ) -> Result<Self, CoreModelError> {
        let template = self.templates.get_required(template_id)?.clone();
        let component = template.components.get_required(component_id)?.clone();

        let mut exports = template.exports.clone();
        let mut project = self.clone();

        match kind {
            ExportKind::Member => {
                self.definition_member(&component.definition, property)?;

                if exports.members.contains_key(&export_id) {
                    return Err(CoreModelError::already_exists(IdKind::Export, &export_id));
                }

                exports.members.insert(
                    export_id,
                    MemberExport {
                        component: component_id.clone(),
                        member: property.to_owned(),
                    },
                );
            }

            ExportKind::Config => {
                self.definition_check_config_exists(&component.definition, property)?;

                if exports.config.contains_key(&export_id) {
                    return Err(CoreModelError::already_exists(IdKind::Export, &export_id));
                }

                // The same config item cannot be exported twice: the two
                // exports would both try to supply it.
                for (id, existing) in &exports.config {
                    if &existing.component == component_id && existing.config_name == property {
                        return Err(CoreModelError::ConfigAlreadyExported {
                            component: component_id.to_string(),
                            config_name: property.to_owned(),
                            export: id.to_string(),
                        });
                    }
                }

                exports.config.insert(
                    export_id.clone(),
                    ConfigExport {
                        component: component_id.clone(),
                        config_name: property.to_owned(),
                    },
                );

                // The exported item leaves the component's own config: from now
                // on whoever instantiates the template supplies it.
                // The export does not exist on the template yet, so the
                // default has to be read through the component it points at.
                let default = self.definition_config_default(&component.definition, property)?;

                let updated = component.without_config_value(property);

                let view = project
                    .view(&ViewKey::Template(template_id.clone()))?
                    .with_component_updated(component_id.clone(), updated.clone())?;
                project = project.with_view_set(view)?;

                changes.push(Change::SetComponent {
                    view: ViewKey::Template(template_id.clone()),
                    id: component_id.clone(),
                    component: updated,
                });

                // ... and every component instantiating this template gains it.
                let definition = ComponentDefinition::Template(template_id.clone());

                for (view_key, usage_id, usage) in self.definition_usage(&definition) {
                    let usage =
                        usage.with_config_value_unchecked(export_id.as_str(), default.clone());

                    let view = project
                        .view(&view_key)?
                        .with_component_updated(usage_id.clone(), usage.clone())?;
                    project = project.with_view_set(view)?;

                    changes.push(Change::SetComponent {
                        view: view_key,
                        id: usage_id,
                        component: usage,
                    });
                }
            }
        }

        // Read back from `project`, not from the snapshot above: the config
        // branch already wrote an updated component into the template.
        let mut template = project
            .templates
            .get_required(template_id)?
            .as_ref()
            .clone();
        template.exports = exports.clone();
        project.templates = project
            .templates
            .set(template_id.clone(), Arc::new(template));

        changes.push(Change::SetTemplate {
            id: template_id.clone(),
            exports,
        });

        Ok(project)
    }

    fn definition_check_config_exists(
        &self,
        definition: &ComponentDefinition,
        name: &str,
    ) -> Result<(), CoreModelError> {
        match definition {
            ComponentDefinition::Plugin(id) => {
                self.plugins.get_required(id)?.config_item(name).map(|_| ())
            }
            ComponentDefinition::Template(id) => self
                .templates
                .get_required(id)?
                .exports
                .config_export(&ExportId::new(name)?)
                .map(|_| ()),
        }
    }

    // -----------------------------------------------------------------------
    // Components
    // -----------------------------------------------------------------------

    pub fn set_component(
        &self,
        changes: &mut Vec<Change>,
        view_key: &ViewKey,
        id: ComponentId,
        definition: ComponentDefinition,
        position: Position,
    ) -> Result<Self, CoreModelError> {
        self.check_definition_exists(&definition)?;

        // A template cannot contain itself, directly or through another
        // template that contains it.
        if let (ViewKey::Template(owner), ComponentDefinition::Template(used)) =
            (view_key, &definition)
            && (owner == used || self.template_reaches(used, owner)?)
        {
            return Err(CoreModelError::CircularTemplate {
                id: used.to_string(),
            });
        }

        // Before the dry run, which would otherwise report a plain duplicate id
        // as a deploy-time collision. `rename_component` guards the same way.
        if self.view(view_key)?.components.contains(&id) {
            return Err(CoreModelError::already_exists(IdKind::Component, &id));
        }

        let mut dry_run = NamingDryRun::new(self);
        dry_run.set_component(view_key, id.clone(), &definition);
        dry_run.validate()?;

        let component = Arc::new(Component {
            config: Some(self.definition_config_template(&definition)?),
            definition,
            position,
            external: false,
        });

        let view = self
            .view(view_key)?
            .with_component_inserted(id.clone(), component.clone())?;

        changes.push(Change::SetComponent {
            view: view_key.clone(),
            id,
            component,
        });

        self.with_view_set(view)
    }

    /// Whether `from` contains, at any depth, a component instantiating `target`.
    fn template_reaches(
        &self,
        from: &TemplateId,
        target: &TemplateId,
    ) -> Result<bool, CoreModelError> {
        let mut seen = Vec::new();
        self.template_reaches_at(from, target, &mut seen)
    }

    fn template_reaches_at(
        &self,
        from: &TemplateId,
        target: &TemplateId,
        seen: &mut Vec<TemplateId>,
    ) -> Result<bool, CoreModelError> {
        if from == target {
            return Ok(true);
        }

        if seen.contains(from) {
            return Ok(false);
        }

        seen.push(from.clone());

        let template = self.templates.get_required(from)?;

        for component in template.components.values() {
            if let ComponentDefinition::Template(id) = &component.definition
                && self.template_reaches_at(id, target, seen)?
            {
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub fn move_components(
        &self,
        changes: &mut Vec<Change>,
        view_key: &ViewKey,
        ids: &[ComponentId],
        delta: Position,
    ) -> Result<Self, CoreModelError> {
        let mut view = self.view(view_key)?;

        for id in ids {
            let component = view.components.get_required(id)?.moved_by(delta);
            view = view.with_component_updated(id.clone(), component.clone())?;

            changes.push(Change::SetComponent {
                view: view_key.clone(),
                id: id.clone(),
                component,
            });
        }

        self.with_view_set(view)
    }

    pub fn configure_component(
        &self,
        changes: &mut Vec<Change>,
        view_key: &ViewKey,
        id: ComponentId,
        config_id: &str,
        value: serde_json::Value,
    ) -> Result<Self, CoreModelError> {
        let view = self.view(view_key)?;
        let component = view.components.get_required(&id)?;

        self.definition_check_config_value(&component.definition, config_id, &value)?;

        let component = component.with_config_value(config_id, value)?;
        let view = view.with_component_updated(id.clone(), component.clone())?;

        changes.push(Change::SetComponent {
            view: view_key.clone(),
            id,
            component,
        });

        self.with_view_set(view)
    }

    /// Rename a component, carrying its bindings and its template exports with
    /// it.
    ///
    /// Bindings have no id of their own: theirs is built from the component ids
    /// they join, so renaming a component re-keys every binding that touches
    /// it. Doing that here, in one operation, is what keeps a binding's id and
    /// its contents from disagreeing.
    pub fn rename_component(
        &self,
        changes: &mut Vec<Change>,
        view_key: &ViewKey,
        id: &ComponentId,
        new_id: ComponentId,
    ) -> Result<Self, CoreModelError> {
        let view = self.view(view_key)?;
        let component = view.components.get_required(id)?.clone();

        if view.components.contains(&new_id) {
            return Err(CoreModelError::already_exists(IdKind::Component, &new_id));
        }

        let mut dry_run = NamingDryRun::new(self);
        dry_run.rename_component(view_key, id, new_id.clone());
        dry_run.validate()?;

        let affected = view.bindings_of(id);

        let mut components = view
            .components
            .removed(id)
            .unwrap_or_else(|| view.components.clone());
        components = components.set(new_id.clone(), component);

        let mut bindings = view.bindings.clone();

        for (binding_id, _) in &affected {
            bindings = bindings.removed(binding_id).unwrap_or(bindings);

            changes.push(Change::ClearBinding {
                view: view_key.clone(),
                id: binding_id.clone(),
            });
        }

        changes.push(Change::RenameComponent {
            view: view_key.clone(),
            id: id.clone(),
            new_id: new_id.clone(),
        });

        let mut rebuilt = Vec::new();

        for (_, binding) in &affected {
            let binding = binding
                .on_rename_component(id, &new_id)
                .unwrap_or_else(|| binding.clone());
            let binding_id = binding.id()?;

            bindings = bindings.set(binding_id.clone(), binding.clone());
            rebuilt.push((binding_id, binding));
        }

        for (binding_id, binding) in rebuilt {
            changes.push(Change::SetBinding {
                view: view_key.clone(),
                id: binding_id,
                binding,
            });
        }

        let view = view.with_components(components).with_bindings(bindings);
        let mut project = self.with_view_set(view)?;

        // Exports point at component ids, so they follow the rename too. They
        // do not travel through a view, so this is a step of its own.
        if let ViewKey::Template(template_id) = view_key {
            let template = project.templates.get_required(template_id)?.clone();

            if let Some(exports) = template.exports.on_rename_component(id, &new_id) {
                let mut updated = template.as_ref().clone();
                updated.exports = exports.clone();
                project.templates = project
                    .templates
                    .set(template_id.clone(), Arc::new(updated));

                changes.push(Change::SetTemplate {
                    id: template_id.clone(),
                    exports,
                });
            }
        }

        Ok(project)
    }

    /// Delete components, with the bindings that touch them.
    ///
    /// Every component is checked before any is removed, so a selection that
    /// contains one undeletable component leaves the project untouched rather
    /// than half applied.
    pub fn clear_components(
        &self,
        changes: &mut Vec<Change>,
        view_key: &ViewKey,
        ids: &[ComponentId],
    ) -> Result<Self, CoreModelError> {
        let mut view = self.view(view_key)?;

        // Exports live on the template, not on a view, so they are read once
        // here rather than asked of the view.
        let exports = match view_key {
            ViewKey::Project => None,
            ViewKey::Template(id) => Some(self.templates.get_required(id)?.exports.clone()),
        };

        for id in ids {
            view.components.get_required(id)?;

            if let Some(exports) = &exports
                && let Some(export) = export_mentioning(exports, id)
            {
                return Err(CoreModelError::ComponentExported {
                    component: id.to_string(),
                    export,
                });
            }
        }

        for id in ids {
            for (binding_id, _) in view.bindings_of(id) {
                let cleared = view.with_binding_cleared(&binding_id);

                if let Some(updated) = cleared {
                    view = updated;

                    changes.push(Change::ClearBinding {
                        view: view_key.clone(),
                        id: binding_id,
                    });
                }
            }

            let cleared = view.with_component_cleared(id);

            if let Some(updated) = cleared {
                view = updated;

                changes.push(Change::ClearComponent {
                    view: view_key.clone(),
                    id: id.clone(),
                });
            }
        }

        self.with_view_set(view)
    }

    /// Copy components, and the bindings entirely between them, into another
    /// view.
    ///
    /// The copies are named `{{id}}-<source id>`: the placeholder is filled in
    /// with the id of whatever component instantiates the target template, so
    /// two instances of the template do not collide.
    pub fn copy_components(
        &self,
        changes: &mut Vec<Change>,
        source_key: &ViewKey,
        ids: &[ComponentId],
        target_key: &ViewKey,
    ) -> Result<(Self, usize, usize), CoreModelError> {
        if source_key == target_key {
            return Err(CoreModelError::CopyToSelf);
        }

        let source = self.view(source_key)?;
        let mut target = self.view(target_key)?;

        let mut dry_run = NamingDryRun::new(self);
        let mut copies = Vec::new();

        for id in ids {
            let component = source.components.get_required(id)?.clone();

            if component.external {
                return Err(CoreModelError::NotCopyable {
                    id: id.to_string(),
                    reason: "it is external",
                });
            }

            if matches!(component.definition, ComponentDefinition::Template(_)) {
                return Err(CoreModelError::NotCopyable {
                    id: id.to_string(),
                    reason: "it instantiates a template",
                });
            }

            let new_id = ComponentId::new(format!("{}-{id}", ComponentId::PLACEHOLDER))?;

            if target.components.contains(&new_id) {
                return Err(CoreModelError::already_exists(IdKind::Component, &new_id));
            }

            dry_run.set_component(target_key, new_id.clone(), &component.definition);
            copies.push((id.clone(), new_id, component));
        }

        dry_run.validate()?;

        let mut renames: HashMap<ComponentId, ComponentId> = HashMap::new();

        for (id, new_id, component) in &copies {
            let copy = Arc::new(Component {
                definition: component.definition.clone(),
                position: component.position,
                config: component.config.clone(),
                external: false,
            });

            target = target.with_component_inserted(new_id.clone(), copy.clone())?;
            renames.insert(id.clone(), new_id.clone());

            changes.push(Change::SetComponent {
                view: target_key.clone(),
                id: new_id.clone(),
                component: copy,
            });
        }

        let mut bindings_copied = 0;

        for (_, binding) in &source.bindings {
            let (Some(source_component), Some(target_component)) = (
                renames.get(&binding.source_component),
                renames.get(&binding.target_component),
            ) else {
                // Only bindings wholly inside the selection come along; a
                // binding with one end outside has nothing to attach to.
                continue;
            };

            let copy = Arc::new(Binding {
                source_component: source_component.clone(),
                source_state: binding.source_state.clone(),
                target_component: target_component.clone(),
                target_action: binding.target_action.clone(),
            });

            let id = copy.id()?;
            target = target.with_binding_inserted(id.clone(), copy.clone())?;
            bindings_copied += 1;

            changes.push(Change::SetBinding {
                view: target_key.clone(),
                id,
                binding: copy,
            });
        }

        Ok((self.with_view_set(target)?, copies.len(), bindings_copied))
    }

    // -----------------------------------------------------------------------
    // Bindings
    // -----------------------------------------------------------------------

    pub fn set_binding(
        &self,
        changes: &mut Vec<Change>,
        view_key: &ViewKey,
        binding: Binding,
    ) -> Result<Self, CoreModelError> {
        let view = self.view(view_key)?;

        if binding.source_component == binding.target_component {
            return Err(CoreModelError::BindingOnSelf);
        }

        let source = view.components.get_required(&binding.source_component)?;
        let target = view.components.get_required(&binding.target_component)?;

        let source_type = self.definition_member_value_type(
            &source.definition,
            &binding.source_state,
            MemberType::State,
        )?;
        let target_type = self.definition_member_value_type(
            &target.definition,
            &binding.target_action,
            MemberType::Action,
        )?;

        if source_type != target_type {
            return Err(CoreModelError::BindingMemberMismatch {
                source_type,
                target_type,
            });
        }

        let id = binding.id()?;
        let binding = Arc::new(binding);
        let view = view.with_binding_inserted(id.clone(), binding.clone())?;

        changes.push(Change::SetBinding {
            view: view_key.clone(),
            id,
            binding,
        });

        self.with_view_set(view)
    }

    pub fn clear_binding(
        &self,
        changes: &mut Vec<Change>,
        view_key: &ViewKey,
        id: &BindingId,
    ) -> Result<Option<Self>, CoreModelError> {
        let view = self.view(view_key)?;

        let Some(view) = view.with_binding_cleared(id) else {
            return Ok(None);
        };

        changes.push(Change::ClearBinding {
            view: view_key.clone(),
            id: id.clone(),
        });

        Ok(Some(self.with_view_set(view)?))
    }

    // -----------------------------------------------------------------------
    // Import
    //
    // What an accepted bulk update does to the project. The checks live in the
    // update builder, which worked out the whole cascade before the user was
    // shown it, so these write what they are told. They still fail on a thing
    // that is not there, because that the types cannot rule out.
    // -----------------------------------------------------------------------

    /// Take a plugin as it is, adding it or replacing what was there.
    pub fn import_plugin(&self, changes: &mut Vec<Change>, id: PluginId, plugin: Plugin) -> Self {
        let plugin = Arc::new(plugin);

        let mut project = self.clone();
        project.plugins = self.plugins.set(id.clone(), plugin.clone());

        changes.push(Change::SetPlugin { id, plugin });

        project
    }

    /// Take a component as it is, on the project view.
    ///
    /// A component inside a template is never imported: an import describes
    /// what runs, and a template is a design-time grouping.
    pub fn import_component(
        &self,
        changes: &mut Vec<Change>,
        id: ComponentId,
        plugin: PluginId,
        external: bool,
        config: Option<Config>,
    ) -> Result<Self, CoreModelError> {
        let view = self.view(&ViewKey::Project)?;

        let component = Arc::new(Component {
            definition: ComponentDefinition::Plugin(plugin),
            position: match view.components.get(&id) {
                // Keep where it already sits, so an import does not rearrange
                // what the user laid out.
                Some(existing) => existing.position,
                None => Position { x: 1, y: 1 },
            },
            config,
            external,
        });

        let view = view.with_components(view.components.set(id.clone(), component.clone()));

        changes.push(Change::SetComponent {
            view: ViewKey::Project,
            id,
            component,
        });

        self.with_view_set(view)
    }

    /// Put a config item back to the value a fresh component would have.
    pub fn import_reset_component_config(
        &self,
        changes: &mut Vec<Change>,
        view_key: &ViewKey,
        id: ComponentId,
        config_id: &str,
    ) -> Result<Self, CoreModelError> {
        let view = self.view(view_key)?;
        let component = view.components.get_required(&id)?;

        let default = self.definition_config_default(&component.definition, config_id)?;
        let component = component.with_config_value_unchecked(config_id, default);

        let view = view.with_component_updated(id.clone(), component.clone())?;

        changes.push(Change::SetComponent {
            view: view_key.clone(),
            id,
            component,
        });

        self.with_view_set(view)
    }

    /// Drop a config item the plugin no longer declares.
    pub fn import_clear_component_config(
        &self,
        changes: &mut Vec<Change>,
        view_key: &ViewKey,
        id: ComponentId,
        config_id: &str,
    ) -> Result<Self, CoreModelError> {
        let view = self.view(view_key)?;
        let component = view
            .components
            .get_required(&id)?
            .without_config_value(config_id);

        let view = view.with_component_updated(id.clone(), component.clone())?;

        changes.push(Change::SetComponent {
            view: view_key.clone(),
            id,
            component,
        });

        self.with_view_set(view)
    }

    /// Drop a template export whose component or member is gone.
    pub fn import_clear_template_export(
        &self,
        changes: &mut Vec<Change>,
        template_id: &TemplateId,
        kind: ExportKind,
        export_id: &ExportId,
    ) -> Result<Self, CoreModelError> {
        let mut template = self.templates.get_required(template_id)?.as_ref().clone();

        match kind {
            ExportKind::Config => {
                template.exports.config.remove(export_id);
            }
            ExportKind::Member => {
                template.exports.members.remove(export_id);
            }
        }

        let exports = template.exports.clone();

        let mut project = self.clone();
        project.templates = project
            .templates
            .set(template_id.clone(), Arc::new(template));

        changes.push(Change::SetTemplate {
            id: template_id.clone(),
            exports,
        });

        Ok(project)
    }

    // -----------------------------------------------------------------------
    // Initial state
    // -----------------------------------------------------------------------

    /// What a newly opened session has to be told.
    pub fn initial_changes(&self, changes: &mut Vec<Change>) {
        changes.push(Change::SetPlugins(self.plugins.clone()));

        for (id, template) in &self.templates {
            changes.push(Change::SetTemplate {
                id: id.clone(),
                exports: template.exports.clone(),
            });
        }

        for view in self.views() {
            let key = view.key.clone();

            for (id, component) in &view.components {
                changes.push(Change::SetComponent {
                    view: key.clone(),
                    id: id.clone(),
                    component: component.clone(),
                });
            }

            for (id, binding) in &view.bindings {
                changes.push(Change::SetBinding {
                    view: key.clone(),
                    id: id.clone(),
                    binding: binding.clone(),
                });
            }
        }
    }
}

fn export_mentioning(exports: &TemplateExports, component: &ComponentId) -> Option<String> {
    for (id, export) in &exports.config {
        if &export.component == component {
            return Some(id.to_string());
        }
    }

    for (id, export) in &exports.members {
        if &export.component == component {
            return Some(id.to_string());
        }
    }

    None
}

fn definition_name(definition: &ComponentDefinition) -> String {
    match definition {
        ComponentDefinition::Plugin(id) => id.to_string(),
        ComponentDefinition::Template(id) => id.to_string(),
    }
}
