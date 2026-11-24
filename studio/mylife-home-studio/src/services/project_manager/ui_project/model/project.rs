use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use studio_web_api::component_model::MemberType;

use super::Collection;
use super::change::Change;
use super::control::{Control, ControlKind};
use super::error::UiModelError;
use super::ids::{
    ComponentId, ControlId, ExportId, IdKind, PluginId, ResourceId, StyleId, TemplateId,
    TemplateInstanceId, WindowId,
};
use super::properties::{ControlProperties, TemplateProperties, WindowProperties};
use super::types::{
    Binding, Component, DefaultWindow, MemberRef, Plugin, Resource, StyleDefinition, Template,
    TemplateInstance, Window, member_type_name,
};
use super::usage::{ComponentUsage, UsageCollector};
use super::view::{View, ViewKey};

/// The UI project.
///
/// Every field is an `Arc` underneath, so cloning a `Project` is seven `Arc`
/// bumps and operations can take `&self` and return a plain `Project`. What a
/// caller holds is never touched by an operation, which is what makes a failed
/// operation a no-op.
///
/// Fields are private: a `Project` cannot be assembled from maps whose keys
/// were never validated.
#[derive(Debug, Clone, Default)]
pub struct Project {
    resources: Collection<ResourceId, Resource>,
    styles: Collection<StyleId, StyleDefinition>,
    windows: Collection<WindowId, Window>,
    templates: Collection<TemplateId, Template>,
    default_window: Arc<DefaultWindow>,
    components: Collection<ComponentId, Component>,
    plugins: Collection<PluginId, Plugin>,
}

impl Project {
    pub fn empty() -> Self {
        Self::default()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_parts(
        resources: Collection<ResourceId, Resource>,
        styles: Collection<StyleId, StyleDefinition>,
        windows: Collection<WindowId, Window>,
        templates: Collection<TemplateId, Template>,
        default_window: Arc<DefaultWindow>,
        components: Collection<ComponentId, Component>,
        plugins: Collection<PluginId, Plugin>,
    ) -> Self {
        Self {
            resources,
            styles,
            windows,
            templates,
            default_window,
            components,
            plugins,
        }
    }

    pub fn resources(&self) -> &Collection<ResourceId, Resource> {
        &self.resources
    }

    pub fn styles(&self) -> &Collection<StyleId, StyleDefinition> {
        &self.styles
    }

    pub fn windows(&self) -> &Collection<WindowId, Window> {
        &self.windows
    }

    pub fn templates(&self) -> &Collection<TemplateId, Template> {
        &self.templates
    }

    pub fn default_window(&self) -> &Arc<DefaultWindow> {
        &self.default_window
    }

    pub fn components(&self) -> &Collection<ComponentId, Component> {
        &self.components
    }

    pub fn plugins(&self) -> &Collection<PluginId, Plugin> {
        &self.plugins
    }

    // -----------------------------------------------------------------------
    // Views
    // -----------------------------------------------------------------------

    pub fn views(&self) -> impl Iterator<Item = View> + '_ {
        let windows = self
            .windows
            .iter()
            .map(|(id, window)| View::Window(id.clone(), window.clone()));
        let templates = self
            .templates
            .iter()
            .map(|(id, template)| View::Template(id.clone(), template.clone()));

        windows.chain(templates)
    }

    pub fn view(&self, key: &ViewKey) -> Result<View, UiModelError> {
        match key {
            ViewKey::Window(id) => Ok(View::Window(
                id.clone(),
                self.windows.get_required(id)?.clone(),
            )),
            ViewKey::Template(id) => Ok(View::Template(
                id.clone(),
                self.templates.get_required(id)?.clone(),
            )),
        }
    }

    /// Put a batch of views back, emitting one change per view.
    ///
    /// Operations never write this loop themselves, which is the point: one
    /// place writes views back and one place emits their changes, so a batch
    /// that touches a view twice still emits one change for it.
    pub fn with_views_set(
        &self,
        changes: &mut Vec<Change>,
        views: impl IntoIterator<Item = View>,
    ) -> Self {
        let mut project = self.clone();

        for view in views {
            match view {
                View::Window(id, window) => {
                    project.windows = project.windows.set(id.clone(), window.clone());
                    changes.push(Change::SetWindow { id, window });
                }
                View::Template(id, template) => {
                    project.templates = project.templates.set(id.clone(), template.clone());
                    changes.push(Change::SetTemplate { id, template });
                }
            }
        }

        project
    }

    fn with_view_set(&self, changes: &mut Vec<Change>, view: View) -> Self {
        self.with_views_set(changes, [view])
    }

    /// Apply a change to every view, keeping only the ones that moved.
    pub fn with_updated_views(
        &self,
        changes: &mut Vec<Change>,
        mut update: impl FnMut(&View) -> Option<View>,
    ) -> Option<Self> {
        let updated: Vec<View> = self.views().filter_map(|view| update(&view)).collect();

        if updated.is_empty() {
            return None;
        }

        Some(self.with_views_set(changes, updated))
    }

    // -----------------------------------------------------------------------
    // Resources
    // -----------------------------------------------------------------------

    pub fn set_resource(
        &self,
        changes: &mut Vec<Change>,
        id: ResourceId,
        resource: Resource,
    ) -> Self {
        let resource = Arc::new(resource);

        let mut project = self.clone();
        project.resources = self.resources.set(id.clone(), resource.clone());

        changes.push(Change::SetResource { id, resource });

        project
    }

    pub fn clear_resource(&self, changes: &mut Vec<Change>, id: &ResourceId) -> Option<Self> {
        let resources = self.resources.removed(id)?;

        let mut project = self.clone();
        project.resources = resources;

        changes.push(Change::ClearResource { id: id.clone() });

        Some(
            project
                .on_change_resource(changes, id, None)
                .unwrap_or(project),
        )
    }

    pub fn rename_resource(
        &self,
        changes: &mut Vec<Change>,
        id: &ResourceId,
        new_id: ResourceId,
    ) -> Result<Option<Self>, UiModelError> {
        let Some(resources) = self.resources.renamed(id, &new_id)? else {
            return Ok(None);
        };

        let mut project = self.clone();
        project.resources = resources;

        changes.push(Change::RenameResource {
            id: id.clone(),
            new_id: new_id.clone(),
        });

        Ok(Some(
            project
                .on_change_resource(changes, id, Some(&new_id))
                .unwrap_or(project),
        ))
    }

    pub fn on_change_resource(
        &self,
        changes: &mut Vec<Change>,
        id: &ResourceId,
        new_id: Option<&ResourceId>,
    ) -> Option<Self> {
        self.with_updated_views(changes, |view| view.on_change_resource(id, new_id))
    }

    // -----------------------------------------------------------------------
    // Styles
    // -----------------------------------------------------------------------

    pub fn set_style(
        &self,
        changes: &mut Vec<Change>,
        id: StyleId,
        style: StyleDefinition,
    ) -> Self {
        let style = Arc::new(style);

        let mut project = self.clone();
        project.styles = self.styles.set(id.clone(), style.clone());

        changes.push(Change::SetStyle { id, style });

        project
    }

    pub fn clear_style(&self, changes: &mut Vec<Change>, id: &StyleId) -> Option<Self> {
        let styles = self.styles.removed(id)?;

        let mut project = self.clone();
        project.styles = styles;

        changes.push(Change::ClearStyle { id: id.clone() });

        Some(
            project
                .on_change_style(changes, id, None)
                .unwrap_or(project),
        )
    }

    pub fn rename_style(
        &self,
        changes: &mut Vec<Change>,
        id: &StyleId,
        new_id: StyleId,
    ) -> Result<Option<Self>, UiModelError> {
        let Some(styles) = self.styles.renamed(id, &new_id)? else {
            return Ok(None);
        };

        let mut project = self.clone();
        project.styles = styles;

        changes.push(Change::RenameStyle {
            id: id.clone(),
            new_id: new_id.clone(),
        });

        Ok(Some(
            project
                .on_change_style(changes, id, Some(&new_id))
                .unwrap_or(project),
        ))
    }

    pub fn on_change_style(
        &self,
        changes: &mut Vec<Change>,
        id: &StyleId,
        new_id: Option<&StyleId>,
    ) -> Option<Self> {
        self.with_updated_views(changes, |view| view.on_change_style(id, new_id))
    }

    // -----------------------------------------------------------------------
    // Windows
    // -----------------------------------------------------------------------

    pub fn new_window(
        &self,
        changes: &mut Vec<Change>,
        id: WindowId,
    ) -> Result<Self, UiModelError> {
        let window = Arc::new(Window::empty());
        let windows = self.windows.inserted(id.clone(), window.clone())?;

        let mut project = self.clone();
        project.windows = windows;

        changes.push(Change::SetWindow { id, window });

        Ok(project)
    }

    pub fn clone_window(
        &self,
        changes: &mut Vec<Change>,
        id: &WindowId,
        new_id: WindowId,
    ) -> Result<Self, UiModelError> {
        let window = self.windows.get_required(id)?.clone();
        let windows = self.windows.inserted(new_id.clone(), window.clone())?;

        let mut project = self.clone();
        project.windows = windows;

        changes.push(Change::SetWindow { id: new_id, window });

        Ok(project)
    }

    pub fn update_window(
        &self,
        changes: &mut Vec<Change>,
        id: WindowId,
        window: Arc<Window>,
    ) -> Result<Self, UiModelError> {
        let windows = self.windows.updated(id.clone(), window.clone())?;

        let mut project = self.clone();
        project.windows = windows;

        changes.push(Change::SetWindow { id, window });

        Ok(project)
    }

    pub fn set_window_properties(
        &self,
        changes: &mut Vec<Change>,
        id: WindowId,
        properties: WindowProperties,
    ) -> Result<Option<Self>, UiModelError> {
        let Some(window) = self.windows.get_required(&id)?.with_properties(properties) else {
            return Ok(None);
        };

        Ok(Some(self.update_window(changes, id, window)?))
    }

    pub fn clear_window(&self, changes: &mut Vec<Change>, id: &WindowId) -> Option<Self> {
        let windows = self.windows.removed(id)?;

        let mut project = self.clone();
        project.windows = windows;

        changes.push(Change::ClearWindow { id: id.clone() });

        Some(
            project
                .on_change_window(changes, id, None)
                .unwrap_or(project),
        )
    }

    pub fn rename_window(
        &self,
        changes: &mut Vec<Change>,
        id: &WindowId,
        new_id: WindowId,
    ) -> Result<Option<Self>, UiModelError> {
        let Some(windows) = self.windows.renamed(id, &new_id)? else {
            return Ok(None);
        };

        let mut project = self.clone();
        project.windows = windows;

        changes.push(Change::RenameWindow {
            id: id.clone(),
            new_id: new_id.clone(),
        });

        Ok(Some(
            project
                .on_change_window(changes, id, Some(&new_id))
                .unwrap_or(project),
        ))
    }

    /// Windows are referenced from control actions and from the default-window
    /// map, so both have to be walked.
    pub fn on_change_window(
        &self,
        changes: &mut Vec<Change>,
        id: &WindowId,
        new_id: Option<&WindowId>,
    ) -> Option<Self> {
        let updated = self.with_updated_views(changes, |view| view.on_change_window(id, new_id));
        let default_window = self.default_window.on_change_window(id, new_id);

        if updated.is_none() && default_window.is_none() {
            return None;
        }

        let mut project = updated.unwrap_or_else(|| self.clone());

        if let Some(default_window) = default_window {
            let default_window = Arc::new(default_window);
            project.default_window = default_window.clone();
            changes.push(Change::SetDefaultWindow(default_window));
        }

        Some(project)
    }

    pub fn set_default_window(
        &self,
        changes: &mut Vec<Change>,
        default_window: DefaultWindow,
    ) -> Self {
        let default_window = Arc::new(default_window);

        let mut project = self.clone();
        project.default_window = default_window.clone();

        changes.push(Change::SetDefaultWindow(default_window));

        project
    }

    // -----------------------------------------------------------------------
    // Templates
    // -----------------------------------------------------------------------

    pub fn new_template(
        &self,
        changes: &mut Vec<Change>,
        id: TemplateId,
    ) -> Result<Self, UiModelError> {
        let template = Arc::new(Template::empty());
        let templates = self.templates.inserted(id.clone(), template.clone())?;

        let mut project = self.clone();
        project.templates = templates;

        changes.push(Change::SetTemplate { id, template });

        Ok(project)
    }

    pub fn clone_template(
        &self,
        changes: &mut Vec<Change>,
        id: &TemplateId,
        new_id: TemplateId,
    ) -> Result<Self, UiModelError> {
        let template = self.templates.get_required(id)?.clone();
        let templates = self.templates.inserted(new_id.clone(), template.clone())?;

        let mut project = self.clone();
        project.templates = templates;

        changes.push(Change::SetTemplate {
            id: new_id,
            template,
        });

        Ok(project)
    }

    pub fn update_template(
        &self,
        changes: &mut Vec<Change>,
        id: TemplateId,
        template: Arc<Template>,
    ) -> Result<Self, UiModelError> {
        let templates = self.templates.updated(id.clone(), template.clone())?;

        let mut project = self.clone();
        project.templates = templates;

        changes.push(Change::SetTemplate { id, template });

        Ok(project)
    }

    pub fn clear_template(&self, changes: &mut Vec<Change>, id: &TemplateId) -> Option<Self> {
        let templates = self.templates.removed(id)?;

        let mut project = self.clone();
        project.templates = templates;

        changes.push(Change::ClearTemplate { id: id.clone() });

        Some(
            project
                .on_change_template(changes, id, None)
                .unwrap_or(project),
        )
    }

    pub fn rename_template(
        &self,
        changes: &mut Vec<Change>,
        id: &TemplateId,
        new_id: TemplateId,
    ) -> Result<Option<Self>, UiModelError> {
        let Some(templates) = self.templates.renamed(id, &new_id)? else {
            return Ok(None);
        };

        let mut project = self.clone();
        project.templates = templates;

        changes.push(Change::RenameTemplate {
            id: id.clone(),
            new_id: new_id.clone(),
        });

        Ok(Some(
            project
                .on_change_template(changes, id, Some(&new_id))
                .unwrap_or(project),
        ))
    }

    /// Renaming a template follows its instances; deleting one deletes them.
    pub fn on_change_template(
        &self,
        changes: &mut Vec<Change>,
        id: &TemplateId,
        new_id: Option<&TemplateId>,
    ) -> Option<Self> {
        self.with_updated_views(changes, |view| view.on_change_template(id, new_id))
    }

    pub fn set_template_properties(
        &self,
        changes: &mut Vec<Change>,
        id: TemplateId,
        properties: TemplateProperties,
    ) -> Result<Option<Self>, UiModelError> {
        let Some(template) = self
            .templates
            .get_required(&id)?
            .with_properties(properties)
        else {
            return Ok(None);
        };

        Ok(Some(self.update_template(changes, id, template)?))
    }

    // -----------------------------------------------------------------------
    // Template exports
    // -----------------------------------------------------------------------

    pub fn set_template_export(
        &self,
        changes: &mut Vec<Change>,
        id: TemplateId,
        export_id: ExportId,
        member_type: MemberType,
        value_type: impl Into<String>,
    ) -> Result<Self, UiModelError> {
        let template =
            self.templates
                .get_required(&id)?
                .with_export(export_id, member_type, value_type);

        self.update_template(changes, id, template)
    }

    pub fn clear_template_export(
        &self,
        changes: &mut Vec<Change>,
        id: TemplateId,
        export_id: &ExportId,
    ) -> Result<Option<Self>, UiModelError> {
        let Some(template) = self.templates.get_required(&id)?.without_export(export_id) else {
            return Ok(None);
        };

        Ok(Some(self.update_template(changes, id, template)?))
    }

    pub fn set_template_bulk_patterns(
        &self,
        changes: &mut Vec<Change>,
        id: TemplateId,
        patterns: &HashMap<ExportId, String>,
    ) -> Result<Option<Self>, UiModelError> {
        let Some(template) = self
            .templates
            .get_required(&id)?
            .with_bulk_patterns(patterns)?
        else {
            return Ok(None);
        };

        Ok(Some(self.update_template(changes, id, template)?))
    }

    // -----------------------------------------------------------------------
    // Controls
    //
    // Each of these is: read a view, change one element, put the view back.
    // `get_view` and `update_view` are the two ends, so an operation body is
    // only the part in the middle that differs.
    // -----------------------------------------------------------------------

    pub fn new_control(
        &self,
        changes: &mut Vec<Change>,
        view: &ViewKey,
        id: ControlId,
        x: i32,
        y: i32,
        kind: ControlKind,
    ) -> Result<Self, UiModelError> {
        let control = Arc::new(Control::new(x, y, kind));
        let view = self.view(view)?.with_control_inserted(id, control)?;

        Ok(self.with_view_set(changes, view))
    }

    pub fn clear_control(
        &self,
        changes: &mut Vec<Change>,
        view: &ViewKey,
        id: &ControlId,
    ) -> Result<Option<Self>, UiModelError> {
        let Some(view) = self.view(view)?.with_control_cleared(id) else {
            return Ok(None);
        };

        Ok(Some(self.with_view_set(changes, view)))
    }

    pub fn rename_control(
        &self,
        changes: &mut Vec<Change>,
        view: &ViewKey,
        id: &ControlId,
        new_id: &ControlId,
    ) -> Result<Option<Self>, UiModelError> {
        let Some(view) = self.view(view)?.with_control_renamed(id, new_id)? else {
            return Ok(None);
        };

        Ok(Some(self.with_view_set(changes, view)))
    }

    /// Copy a control, possibly into another view.
    ///
    /// Only the target view is written, and the offset applied to the copy
    /// depends on whether the two views are the same, which is one `ViewKey`
    /// comparison.
    pub fn clone_control(
        &self,
        changes: &mut Vec<Change>,
        source: &ViewKey,
        id: &ControlId,
        target: &ViewKey,
        new_id: ControlId,
    ) -> Result<Self, UiModelError> {
        let control = self
            .view(source)?
            .controls()
            .get_required(id)?
            .cloned_at(source == target);

        let view = self.view(target)?.with_control_inserted(new_id, control)?;

        Ok(self.with_view_set(changes, view))
    }

    pub fn set_control_properties(
        &self,
        changes: &mut Vec<Change>,
        view: &ViewKey,
        id: ControlId,
        properties: ControlProperties,
    ) -> Result<Option<Self>, UiModelError> {
        let view = self.view(view)?;

        let Some(control) = view
            .controls()
            .get_required(&id)?
            .with_properties(properties)
        else {
            return Ok(None);
        };

        let view = view.with_control_updated(id, control)?;

        Ok(Some(self.with_view_set(changes, view)))
    }

    // -----------------------------------------------------------------------
    // Template instances
    // -----------------------------------------------------------------------

    pub fn new_template_instance(
        &self,
        changes: &mut Vec<Change>,
        view: &ViewKey,
        id: TemplateInstanceId,
        template_id: TemplateId,
        x: i32,
        y: i32,
    ) -> Result<Self, UiModelError> {
        let template = self.templates.get_required(&template_id)?;
        let instance = Arc::new(TemplateInstance::new(template_id, template, x, y));
        let view = self.view(view)?.with_instance_inserted(id, instance)?;

        Ok(self.with_view_set(changes, view))
    }

    pub fn clear_template_instance(
        &self,
        changes: &mut Vec<Change>,
        view: &ViewKey,
        id: &TemplateInstanceId,
    ) -> Result<Option<Self>, UiModelError> {
        let Some(view) = self.view(view)?.with_instance_cleared(id) else {
            return Ok(None);
        };

        Ok(Some(self.with_view_set(changes, view)))
    }

    pub fn rename_template_instance(
        &self,
        changes: &mut Vec<Change>,
        view: &ViewKey,
        id: &TemplateInstanceId,
        new_id: &TemplateInstanceId,
    ) -> Result<Option<Self>, UiModelError> {
        let Some(view) = self.view(view)?.with_instance_renamed(id, new_id)? else {
            return Ok(None);
        };

        Ok(Some(self.with_view_set(changes, view)))
    }

    pub fn clone_template_instance(
        &self,
        changes: &mut Vec<Change>,
        view: &ViewKey,
        id: &TemplateInstanceId,
        new_id: TemplateInstanceId,
    ) -> Result<Self, UiModelError> {
        let view = self.view(view)?;
        let instance = view.instances().get_required(id)?.cloned_nearby();
        let view = view.with_instance_inserted(new_id, instance)?;

        Ok(self.with_view_set(changes, view))
    }

    pub fn move_template_instance(
        &self,
        changes: &mut Vec<Change>,
        view: &ViewKey,
        id: TemplateInstanceId,
        x: i32,
        y: i32,
    ) -> Result<Self, UiModelError> {
        let view = self.view(view)?;
        let instance = view.instances().get_required(&id)?.moved(x, y);
        let view = view.with_instance_updated(id, instance)?;

        Ok(self.with_view_set(changes, view))
    }

    /// Point an instance at another template. Its bindings are keyed by the old
    /// template's exports, so they are reset rather than migrated.
    pub fn set_template_instance_template(
        &self,
        changes: &mut Vec<Change>,
        view: &ViewKey,
        id: TemplateInstanceId,
        template_id: TemplateId,
    ) -> Result<Self, UiModelError> {
        let template = self.templates.get_required(&template_id)?;

        let view = self.view(view)?;
        let instance = view
            .instances()
            .get_required(&id)?
            .with_template(template_id, template);
        let view = view.with_instance_updated(id, instance)?;

        Ok(self.with_view_set(changes, view))
    }

    /// Bind an instance's exports.
    ///
    /// The two kinds of target are the reason this is the awkward operation: a
    /// component member is looked up in the project, an enclosing template's
    /// export is looked up in the view this instance sits in, and binding to
    /// the latter is only possible when that view is a template at all. All
    /// three fall out of matching one `MemberRef`, and the owner comes from the
    /// view key, so there is nothing for a caller to pass inconsistently.
    pub fn set_template_instance_bindings(
        &self,
        changes: &mut Vec<Change>,
        view: &ViewKey,
        id: TemplateInstanceId,
        bindings: HashMap<ExportId, Binding>,
    ) -> Result<Self, UiModelError> {
        let view = self.view(view)?;
        let instance = view.instances().get_required(&id)?;
        let template = self.templates.get_required(&instance.template_id)?;

        let mut new_bindings = instance.bindings.clone();

        for (export_id, binding) in bindings {
            if !new_bindings.contains_key(&export_id) {
                return Err(UiModelError::not_found(IdKind::Export, &export_id));
            }

            let Some(target) = binding else {
                return Err(UiModelError::BindingWithoutTarget {
                    export: export_id.to_string(),
                });
            };

            let export = template.exports.get_required(&export_id)?;

            let value_type = match &target {
                MemberRef::Component { id, member } => {
                    self.find_component_member_value_type(id, member, export.member_type)?
                }
                MemberRef::OwnerExport(owner_export) => {
                    let owner = view.as_template().ok_or_else(|| {
                        UiModelError::OwnerExportOutsideTemplate {
                            export: export_id.to_string(),
                        }
                    })?;

                    owner.export_value_type(owner_export, export.member_type)?
                }
            };

            if value_type != export.value_type {
                return Err(UiModelError::BindingTypeMismatch {
                    export: export_id.to_string(),
                    expected: export.value_type.clone(),
                    actual: value_type.to_owned(),
                });
            }

            new_bindings.insert(export_id, Some(target));
        }

        let instance = instance.with_bindings(new_bindings);
        let view = view.with_instance_updated(id, instance)?;

        Ok(self.with_view_set(changes, view))
    }

    // -----------------------------------------------------------------------
    // Components
    // -----------------------------------------------------------------------

    /// Replace the component/plugin set after a refresh.
    ///
    /// The replacement and its change are one operation, so there is no way to
    /// swap the component set without telling the designer about it.
    pub fn set_component_data(
        &self,
        changes: &mut Vec<Change>,
        components: Collection<ComponentId, Component>,
        plugins: Collection<PluginId, Plugin>,
    ) -> Self {
        let mut project = self.clone();
        project.components = components.clone();
        project.plugins = plugins.clone();

        changes.push(Change::SetComponentData {
            components,
            plugins,
        });

        project
    }

    pub fn find_component_member_value_type(
        &self,
        component_id: &ComponentId,
        member_name: &str,
        member_type: MemberType,
    ) -> Result<&str, UiModelError> {
        let component = self.components.get_required(component_id)?;

        let plugin = self
            .plugins
            .get(&component.plugin)
            .ok_or_else(|| UiModelError::not_found(IdKind::Plugin, &component.plugin))?;

        plugin
            .member(member_name, member_type)
            .map(|member| member.value_type.as_str())
            .ok_or_else(|| UiModelError::MemberNotFound {
                component: component_id.to_string(),
                member: member_name.to_owned(),
                member_type: member_type_name(member_type),
            })
    }

    pub fn collect_component_usage(&self) -> Vec<ComponentUsage> {
        let mut collector = UsageCollector::new();

        for view in self.views() {
            view.collect_component_usage(&mut collector);
        }

        collector.into_usage()
    }

    /// Drop every reference listed in `usage`.
    ///
    /// Read phase then write phase. During the read phase the accumulator is
    /// the only source for a view already touched, so two usages in the same
    /// view compose instead of overwriting each other. The write phase goes
    /// through `with_views_set`, so exactly one change is emitted per view
    /// however many usages it contained.
    pub fn clear_component_usage(
        &self,
        changes: &mut Vec<Change>,
        usage: &[ComponentUsage],
    ) -> Result<Option<Self>, UiModelError> {
        // BTreeMap, not HashMap: the emitted change order is then deterministic.
        let mut views: BTreeMap<ViewKey, View> = BTreeMap::new();

        for usage in usage {
            let key = usage.path.view_key()?;

            let view = match views.get(&key) {
                Some(view) => view.clone(),
                None => self.view(&key)?,
            };

            if let Some(view) = view.clear_component_usage(usage)? {
                views.insert(key, view);
            }
        }

        if views.is_empty() {
            return Ok(None);
        }

        Ok(Some(self.with_views_set(changes, views.into_values())))
    }

    // -----------------------------------------------------------------------
    // Initial state
    // -----------------------------------------------------------------------

    /// What a newly opened session has to be told.
    pub fn initial_changes(&self, changes: &mut Vec<Change>) {
        changes.push(Change::SetDefaultWindow(self.default_window.clone()));

        changes.push(Change::SetComponentData {
            components: self.components.clone(),
            plugins: self.plugins.clone(),
        });

        for (id, resource) in &self.resources {
            changes.push(Change::SetResource {
                id: id.clone(),
                resource: resource.clone(),
            });
        }

        for (id, style) in &self.styles {
            changes.push(Change::SetStyle {
                id: id.clone(),
                style: style.clone(),
            });
        }

        for (id, window) in &self.windows {
            changes.push(Change::SetWindow {
                id: id.clone(),
                window: window.clone(),
            });
        }

        for (id, template) in &self.templates {
            changes.push(Change::SetTemplate {
                id: id.clone(),
                template: template.clone(),
            });
        }
    }
}
