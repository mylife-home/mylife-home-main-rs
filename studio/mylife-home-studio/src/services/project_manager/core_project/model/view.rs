use std::sync::Arc;

use super::Collection;
use super::error::CoreModelError;
use super::ids::{BindingId, ComponentId, TemplateId};
use super::types::{Binding, Component};

/// Names a view without carrying it. `Project` is the project's own components
/// and bindings; the wire spells it as a null `templateId`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ViewKey {
    Project,
    Template(TemplateId),
}

impl ViewKey {
    /// The `templateId` field every core notification carries.
    pub fn template(&self) -> Option<&TemplateId> {
        match self {
            ViewKey::Project => None,
            ViewKey::Template(id) => Some(id),
        }
    }
}

/// The components and bindings of one view, with the key that says which view
/// they belong to.
///
/// Operations are written against this, so the same body runs over the project
/// and over a template. The key travels with the contents because
/// `Project::views` hands them out in a batch, where there is no parameter to
/// say which is which.
///
/// A template is a view plus its exports, and the exports stay on the
/// template: of the operations that work through a view, two consult them, and
/// both hold the key already.
#[derive(Debug, Clone)]
pub struct View {
    pub key: ViewKey,
    pub components: Collection<ComponentId, Component>,
    pub bindings: Collection<BindingId, Binding>,
}

impl View {
    pub fn with_components(&self, components: Collection<ComponentId, Component>) -> Self {
        Self {
            components,
            ..self.clone()
        }
    }

    pub fn with_bindings(&self, bindings: Collection<BindingId, Binding>) -> Self {
        Self {
            bindings,
            ..self.clone()
        }
    }

    /// Every binding that mentions this component, in either position.
    ///
    /// This walks the bindings rather than keeping an index per component.
    /// O(bindings) instead of O(1), on a collection that is small and only
    /// walked on delete and rename; in exchange there is no second structure
    /// that can fall out of step with the first.
    pub fn bindings_of(&self, component: &ComponentId) -> Vec<(BindingId, Arc<Binding>)> {
        self.bindings
            .iter()
            .filter(|(_, binding)| binding.mentions(component))
            .map(|(id, binding)| (id.clone(), binding.clone()))
            .collect()
    }

    pub fn with_component_inserted(
        &self,
        id: ComponentId,
        component: Arc<Component>,
    ) -> Result<Self, CoreModelError> {
        Ok(self.with_components(self.components.inserted(id, component)?))
    }

    pub fn with_component_updated(
        &self,
        id: ComponentId,
        component: Arc<Component>,
    ) -> Result<Self, CoreModelError> {
        Ok(self.with_components(self.components.updated(id, component)?))
    }

    pub fn with_component_cleared(&self, id: &ComponentId) -> Option<Self> {
        Some(self.with_components(self.components.removed(id)?))
    }

    pub fn with_binding_inserted(
        &self,
        id: BindingId,
        binding: Arc<Binding>,
    ) -> Result<Self, CoreModelError> {
        Ok(self.with_bindings(self.bindings.inserted(id, binding)?))
    }

    pub fn with_binding_cleared(&self, id: &BindingId) -> Option<Self> {
        Some(self.with_bindings(self.bindings.removed(id)?))
    }
}
