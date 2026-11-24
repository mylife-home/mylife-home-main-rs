use std::sync::Arc;

use super::Collection;
use super::control::Control;
use super::error::UiModelError;
use super::ids::{ControlId, ResourceId, StyleId, TemplateId, TemplateInstanceId, WindowId};
use super::path::{ElementPath, PathNode};
use super::types::{Template, TemplateInstance, Window};
use super::usage::{ComponentUsage, UsageCollector};

/// Identifies a view without carrying it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ViewKey {
    Window(WindowId),
    Template(TemplateId),
}

/// A window or a template, carrying its own id.
///
/// Carrying the id is what lets `Project::with_views_set` take a bare list of
/// views and put each one back where it belongs. Operations never pair a key
/// with a view by hand, so they cannot pair them wrongly.
#[derive(Debug, Clone)]
pub enum View {
    Window(WindowId, Arc<Window>),
    Template(TemplateId, Arc<Template>),
}

impl View {
    pub fn key(&self) -> ViewKey {
        match self {
            View::Window(id, _) => ViewKey::Window(id.clone()),
            View::Template(id, _) => ViewKey::Template(id.clone()),
        }
    }

    pub fn path(&self) -> ElementPath {
        ElementPath::new().child(match self {
            View::Window(id, _) => PathNode::Window(id.clone()),
            View::Template(id, _) => PathNode::Template(id.clone()),
        })
    }

    /// The template this view *is*, when it is one.
    ///
    /// This is what a binding onto an enclosing template's export resolves
    /// against. `set_template_instance_bindings` takes it from the view instead
    /// of from its caller, so there is no owner argument to pass wrongly.
    pub fn as_template(&self) -> Option<&Arc<Template>> {
        match self {
            View::Template(_, template) => Some(template),
            View::Window(..) => None,
        }
    }

    pub fn controls(&self) -> &Collection<ControlId, Control> {
        match self {
            View::Window(_, window) => &window.controls,
            View::Template(_, template) => &template.controls,
        }
    }

    pub fn instances(&self) -> &Collection<TemplateInstanceId, TemplateInstance> {
        match self {
            View::Window(_, window) => &window.templates,
            View::Template(_, template) => &template.templates,
        }
    }

    pub fn with_controls(&self, controls: Collection<ControlId, Control>) -> Self {
        match self {
            View::Window(id, window) => {
                let mut window = window.as_ref().clone();
                window.controls = controls;
                View::Window(id.clone(), Arc::new(window))
            }
            View::Template(id, template) => {
                let mut template = template.as_ref().clone();
                template.controls = controls;
                View::Template(id.clone(), Arc::new(template))
            }
        }
    }

    pub fn with_instances(
        &self,
        instances: Collection<TemplateInstanceId, TemplateInstance>,
    ) -> Self {
        match self {
            View::Window(id, window) => {
                let mut window = window.as_ref().clone();
                window.templates = instances;
                View::Window(id.clone(), Arc::new(window))
            }
            View::Template(id, template) => {
                let mut template = template.as_ref().clone();
                template.templates = instances;
                View::Template(id.clone(), Arc::new(template))
            }
        }
    }

    // -----------------------------------------------------------------------
    // Elements
    //
    // Eight one-line wrappers over `Collection`, and they are the whole reason
    // the actor's element handlers shrink: `get_view` / mutate the map / put it
    // back / `update_view` becomes one call that cannot drop the view or put it
    // back under the wrong id.
    // -----------------------------------------------------------------------

    pub fn with_control_inserted(
        &self,
        id: ControlId,
        control: Arc<Control>,
    ) -> Result<Self, UiModelError> {
        Ok(self.with_controls(self.controls().inserted(id, control)?))
    }

    pub fn with_control_updated(
        &self,
        id: ControlId,
        control: Arc<Control>,
    ) -> Result<Self, UiModelError> {
        Ok(self.with_controls(self.controls().updated(id, control)?))
    }

    pub fn with_control_cleared(&self, id: &ControlId) -> Option<Self> {
        Some(self.with_controls(self.controls().removed(id)?))
    }

    pub fn with_control_renamed(
        &self,
        id: &ControlId,
        new_id: &ControlId,
    ) -> Result<Option<Self>, UiModelError> {
        Ok(self
            .controls()
            .renamed(id, new_id)?
            .map(|controls| self.with_controls(controls)))
    }

    pub fn with_instance_inserted(
        &self,
        id: TemplateInstanceId,
        instance: Arc<TemplateInstance>,
    ) -> Result<Self, UiModelError> {
        Ok(self.with_instances(self.instances().inserted(id, instance)?))
    }

    pub fn with_instance_updated(
        &self,
        id: TemplateInstanceId,
        instance: Arc<TemplateInstance>,
    ) -> Result<Self, UiModelError> {
        Ok(self.with_instances(self.instances().updated(id, instance)?))
    }

    pub fn with_instance_cleared(&self, id: &TemplateInstanceId) -> Option<Self> {
        Some(self.with_instances(self.instances().removed(id)?))
    }

    pub fn with_instance_renamed(
        &self,
        id: &TemplateInstanceId,
        new_id: &TemplateInstanceId,
    ) -> Result<Option<Self>, UiModelError> {
        Ok(self
            .instances()
            .renamed(id, new_id)?
            .map(|instances| self.with_instances(instances)))
    }

    // -----------------------------------------------------------------------
    // Reactions to renames and deletions elsewhere
    // -----------------------------------------------------------------------

    pub fn on_change_resource(&self, id: &ResourceId, new_id: Option<&ResourceId>) -> Option<Self> {
        let controls = self
            .controls()
            .map_values(|_, control| control.on_change_resource(id, new_id));

        match self {
            View::Window(window_id, window) => {
                let background =
                    (window.background_resource.as_ref() == Some(id)).then(|| new_id.cloned());

                if controls.is_none() && background.is_none() {
                    return None;
                }

                let mut updated = window.as_ref().clone();
                if let Some(controls) = controls {
                    updated.controls = controls;
                }
                if let Some(background) = background {
                    updated.background_resource = background;
                }
                Some(View::Window(window_id.clone(), Arc::new(updated)))
            }
            View::Template(..) => controls.map(|controls| self.with_controls(controls)),
        }
    }

    pub fn on_change_style(&self, id: &StyleId, new_id: Option<&StyleId>) -> Option<Self> {
        let controls = self
            .controls()
            .map_values(|_, control| control.on_change_style(id, new_id));

        match self {
            View::Window(window_id, window) => {
                let style = window.style.on_change_style(id, new_id);

                if controls.is_none() && style.is_none() {
                    return None;
                }

                let mut updated = window.as_ref().clone();
                if let Some(controls) = controls {
                    updated.controls = controls;
                }
                if let Some(style) = style {
                    updated.style = style;
                }
                Some(View::Window(window_id.clone(), Arc::new(updated)))
            }
            View::Template(..) => controls.map(|controls| self.with_controls(controls)),
        }
    }

    pub fn on_change_window(&self, id: &WindowId, new_id: Option<&WindowId>) -> Option<Self> {
        let controls = self
            .controls()
            .map_values(|_, control| control.on_change_window(id, new_id))?;
        Some(self.with_controls(controls))
    }

    /// A renamed template is followed; a deleted one takes its instances with
    /// it.
    pub fn on_change_template(&self, id: &TemplateId, new_id: Option<&TemplateId>) -> Option<Self> {
        let instances = match new_id {
            Some(new_id) => self
                .instances()
                .map_values(|_, instance| instance.on_change_template(id, new_id))?,
            None => self
                .instances()
                .retained(|_, instance| &instance.template_id != id)?,
        };

        Some(self.with_instances(instances))
    }

    // -----------------------------------------------------------------------
    // Component usage
    // -----------------------------------------------------------------------

    pub fn collect_component_usage(&self, collector: &mut UsageCollector) {
        let path = self.path();

        for (id, control) in self.controls() {
            control.collect_component_usage(&path.child(PathNode::Control(id.clone())), collector);
        }

        for (id, instance) in self.instances() {
            instance.collect_component_usage(
                &path.child(PathNode::TemplateInstance(id.clone())),
                collector,
            );
        }
    }

    /// Clear one usage. The element the path points at decides what to clear;
    /// the match is exhaustive over `PathNode`, so there is no "unknown view
    /// element type" branch to write or to hit.
    pub fn clear_component_usage(
        &self,
        usage: &ComponentUsage,
    ) -> Result<Option<Self>, UiModelError> {
        match usage.path.element()? {
            PathNode::Control(id) => {
                let controls = self.controls();
                let control = controls.get_required(id)?;

                let Some(control) = control.clear_component_usage(usage) else {
                    return Ok(None);
                };

                Ok(Some(self.with_controls(controls.set(id.clone(), control))))
            }

            PathNode::TemplateInstance(id) => {
                let instances = self.instances();
                let instance = instances.get_required(id)?;

                let Some(instance) = instance.clear_component_usage(usage) else {
                    return Ok(None);
                };

                Ok(Some(
                    self.with_instances(instances.set(id.clone(), instance)),
                ))
            }

            // A path's second node is a control or an instance, by
            // construction. The compiler asks for this arm because the type
            // cannot say so; it is not reachable from a path the collector
            // built.
            node => Err(UiModelError::UnexpectedPathElement { node: node.kind() }),
        }
    }
}
