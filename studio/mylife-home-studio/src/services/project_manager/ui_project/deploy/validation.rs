use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::Arc;

use common::components::metadata::Type;
use studio_web_api::component_model::MemberType;
use studio_web_api::{project_manager, ui_model};

use super::super::model::{
    ExportId, Id, MemberRef, Project, ResourceId, Template, TemplateId, WindowId,
};

/// The element being deployed is not deployable. The reason is in the
/// `Validation` it was reported to, with the path it was found at.
#[derive(Debug)]
pub struct InvalidForDeployment;

pub type ValidationResult<T> = Result<T, InvalidForDeployment>;

/// A reference resolved to the component and member the running UI will talk
/// to. Both are plain strings because that is what the deployed definition
/// carries: the component lives in the core project, not in this one.
pub type Resolved = (String, String);

/// Where an `OwnerExport` reference resolves.
#[derive(Debug, Clone)]
enum Exports {
    /// Inside an instance: what the enclosing view bound each export to.
    Bound(Arc<HashMap<ExportId, Resolved>>),
    /// A template deployed on its own, to check it in isolation. Every export
    /// resolves to a placeholder, so a control bound to one is checked for
    /// everything except who it will eventually point at.
    Standalone,
}

/// The accumulator a deploy runs in.
///
/// It gathers every error instead of stopping at the first, which is what makes
/// deploy a different shape from a model operation: an operation fails with one
/// `UiModelError`, a deploy reports a list the designer can walk. Cloning one is
/// how the walk descends; the error list is shared, so a clone reports into the
/// same place.
#[derive(Clone)]
pub struct Validation<'a> {
    project: &'a Project,
    errors: Rc<RefCell<Vec<project_manager::UiValidationError>>>,
    path: project_manager::UiElementPath,
    /// The template that owns the element being checked, when inside one.
    owner_template: Option<Arc<Template>>,
    exports: Exports,
    template_stack: Vec<TemplateId>,
}

impl<'a> Validation<'a> {
    pub fn new(project: &'a Project) -> Self {
        Self {
            project,
            errors: Rc::new(RefCell::new(Vec::new())),
            path: Vec::new(),
            owner_template: None,
            exports: Exports::Standalone,
            template_stack: Vec::new(),
        }
    }

    pub fn project(&self) -> &'a Project {
        self.project
    }

    pub fn add_error(&self, message: impl Into<String>) {
        self.errors
            .borrow_mut()
            .push(project_manager::UiValidationError {
                message: message.into(),
                path: self.path.clone(),
            });
    }

    pub fn errors(&self) -> Vec<project_manager::UiValidationError> {
        self.errors.borrow().clone()
    }

    /// Descend one element. The path this builds is what the designer uses to
    /// point the user at the thing that is wrong.
    pub fn at(&self, node_type: &str, id: impl ToString) -> Self {
        let mut path = self.path.clone();
        path.push(Arc::new(project_manager::UiElementPathNode {
            r#type: node_type.to_owned(),
            id: id.to_string(),
        }));

        Self {
            path,
            ..self.clone()
        }
    }

    pub fn with_owner_template(&self, owner_template: Option<Arc<Template>>) -> Self {
        Self {
            owner_template,
            ..self.clone()
        }
    }

    pub fn with_bindings(&self, bindings: HashMap<ExportId, Resolved>) -> Self {
        Self {
            exports: Exports::Bound(Arc::new(bindings)),
            ..self.clone()
        }
    }

    /// Enter a template, refusing to enter one that is already on the stack.
    pub fn enter_template(&self, id: &TemplateId) -> ValidationResult<Self> {
        if self.template_stack.contains(id) {
            self.add_error(format!("Le template '{id}' est récursif."));
            return Err(InvalidForDeployment);
        }

        let mut validation = self.clone();
        validation.template_stack.push(id.clone());
        Ok(validation)
    }

    pub fn validate_window(&self, id: &Option<WindowId>) -> ValidationResult<String> {
        let Some(id) = id else {
            self.add_error("L'identifiant n'est pas défini.");
            return Err(InvalidForDeployment);
        };

        if !self.project.windows().contains(id) {
            self.add_error(format!("La fenêtre '{id}' n'existe pas."));
            return Err(InvalidForDeployment);
        }

        Ok(id.to_string())
    }

    /// An id the model keeps as a plain string, so it is the one place an empty
    /// id can still reach a deploy.
    pub fn validate_free_id(&self, id: &str) -> ValidationResult<String> {
        if id.trim().is_empty() {
            self.add_error("L'identifiant est vide.");
            return Err(InvalidForDeployment);
        }

        Ok(id.to_owned())
    }

    pub fn validate_resource(
        &self,
        id: &Option<ResourceId>,
    ) -> ValidationResult<ui_model::Resource> {
        let Some(id) = id else {
            self.add_error("La ressource n'est pas définie.");
            return Err(InvalidForDeployment);
        };

        if !self.project.resources().contains(id) {
            self.add_error(format!("La ressource '{id}' n'existe pas."));
            return Err(InvalidForDeployment);
        }

        Ok(ui_model::Resource(id.to_string()))
    }

    pub fn validate_optional_resource(
        &self,
        id: &Option<ResourceId>,
    ) -> ValidationResult<Option<ui_model::Resource>> {
        id.as_ref()
            .map(|id| self.validate_resource(&Some(id.clone())))
            .transpose()
    }

    /// Check that a reference points at something that exists, of the right
    /// member type and, where the caller knows it, the right value type.
    ///
    /// `optional` says whether an unbound reference is allowed here. The two
    /// kinds of reference resolve in different places: a component against the
    /// project, an owner export against the template being deployed.
    pub fn validate_member(
        &self,
        member: &Option<MemberRef>,
        member_type: MemberType,
        expected_value_type: Option<&str>,
        optional: bool,
    ) -> ValidationResult<Option<Type>> {
        let Some(member) = member else {
            if optional {
                return Ok(None);
            }

            self.add_error("Le composant n'est pas défini.");
            return Err(InvalidForDeployment);
        };

        let value_type = match member {
            MemberRef::Component { id, member } => {
                if member.is_empty() {
                    self.add_error("Le composant n'est pas défini.");
                    return Err(InvalidForDeployment);
                }

                if !self.project.components().contains(id) {
                    self.add_error(format!("Le composant '{id}' n'existe pas."));
                    return Err(InvalidForDeployment);
                }

                self.project
                    .find_component_member_value_type(id, member, member_type)
                    .ok()
                    .map(str::to_owned)
            }

            MemberRef::OwnerExport(export) => {
                let Some(template) = &self.owner_template else {
                    self.add_error("Le composant n'est pas défini.");
                    return Err(InvalidForDeployment);
                };

                template
                    .exports
                    .get(export)
                    .filter(|item| item.member_type == member_type)
                    .map(|item| item.value_type.clone())
            }
        };

        let (name, owner) = match member {
            MemberRef::Component { id, member } => {
                (member.as_str(), format!(" du composant '{id}'"))
            }
            MemberRef::OwnerExport(export) => {
                (export.as_str(), String::from(" de l'export template"))
            }
        };

        let kind = match member_type {
            MemberType::State => "L'état",
            MemberType::Action => "L'action",
        };

        let Some(value_type) = value_type else {
            self.add_error(format!("{kind} '{name}'{owner} n'existe pas."));
            return Err(InvalidForDeployment);
        };

        if let Some(expected) = expected_value_type
            && expected != value_type
        {
            self.add_error(format!(
                "{kind} '{name}'{owner} a un type incorrect : '{expected}'."
            ));
            return Err(InvalidForDeployment);
        }

        let Ok(value_type) = Type::from_str(&value_type) else {
            self.add_error(format!("Le type '{value_type}' est incorrect."));
            return Err(InvalidForDeployment);
        };

        Ok(Some(value_type))
    }

    /// What the deployed definition should point at. A component reference is
    /// itself; an owner export is whatever the enclosing instance bound it to.
    pub fn resolve(&self, member: &Option<MemberRef>) -> Option<Resolved> {
        match member.as_ref()? {
            MemberRef::Component { id, member } => Some((id.to_string(), member.clone())),

            MemberRef::OwnerExport(export) => match &self.exports {
                Exports::Bound(bindings) => bindings.get(export).cloned(),
                Exports::Standalone => Some((export.to_string(), export.to_string())),
            },
        }
    }
}
