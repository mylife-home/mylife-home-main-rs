use std::{cell::RefCell, collections::HashMap, rc::Rc, str::FromStr, sync::Arc};

use common::components::metadata::Type;
use studio_web_api::project_manager;

use crate::services::project_manager::ui_project::project::ProjectOps;

#[derive(Debug)]
pub struct InvalidForDeployment;

pub type ValidationResult<T> = Result<T, InvalidForDeployment>;

pub trait ValidationErrExt {
    fn ok_or_validation_err(
        self,
        validation: &Validation,
        message: impl Into<String>,
    ) -> ValidationResult<()>;
}

impl ValidationErrExt for bool {
    fn ok_or_validation_err(
        self,
        validation: &Validation,
        message: impl Into<String>,
    ) -> ValidationResult<()> {
        if self {
            Ok(())
        } else {
            validation.add_error(message);
            Err(InvalidForDeployment)
        }
    }
}

#[derive(Debug, Clone)]
pub struct Validation {
    project: Arc<project_manager::UiProject>,
    errors: Rc<RefCell<Vec<project_manager::UiValidationError>>>,
    path: project_manager::UiElementPath,
    owner_template: Option<Arc<project_manager::UiTemplateData>>,
    bindings: Arc<HashMap<String, project_manager::UiTemplateInstanceBinding>>,
}

impl Validation {
    pub fn new(project: Arc<project_manager::UiProject>) -> Self {
        Self {
            project,
            errors: Rc::new(RefCell::new(Vec::new())),
            path: Vec::new(),
            owner_template: None,
            bindings: Arc::new(HashMap::new()),
        }
    }

    pub fn project(&self) -> &Arc<project_manager::UiProject> {
        &self.project
    }

    pub fn owner_template(&self) -> Option<&Arc<project_manager::UiTemplateData>> {
        self.owner_template.as_ref()
    }

    pub fn bindings(&self) -> &HashMap<String, project_manager::UiTemplateInstanceBinding> {
        &self.bindings
    }

    pub fn add_error(&self, message: impl Into<String>) {
        self.errors
            .borrow_mut()
            .push(project_manager::UiValidationError {
                message: message.into(),
                path: self.path.clone(),
            });
    }

    pub fn validate_id(&self, id: &Option<String>) -> ValidationResult<String> {
        let Some(id) = id else {
            self.add_error("L'identifiant n'est pas défini.");
            return Err(InvalidForDeployment);
        };

        if id.trim().is_empty() {
            self.add_error("L'identifiant est vide.");
            return Err(InvalidForDeployment);
        }

        Ok(id.clone())
    }

    pub fn validate_window_id(&self, window_id: &Option<String>) -> ValidationResult<String> {
        let window_id = self.validate_id(window_id)?;

        if !self.project.windows.contains_key(&window_id) {
            self.add_error(format!("La fenêtre '{}' n'existe pas.", window_id));
            return Err(InvalidForDeployment);
        }

        Ok(window_id)
    }

    pub fn validate_mandatory_resource(
        &self,
        resource: &Option<project_manager::Resource>,
    ) -> ValidationResult<project_manager::Resource> {
        let Some(resource) = resource else {
            self.add_error("La ressource n'est pas définie.");
            return Err(InvalidForDeployment);
        };

        if !self.project.resources.contains_key(&resource.0) {
            self.add_error(format!("La ressource '{}' n'existe pas.", resource.0));
            return Err(InvalidForDeployment);
        }

        Ok(resource.clone())
    }

    pub fn validate_optional_resource(
        &self,
        resource: &Option<project_manager::Resource>,
    ) -> ValidationResult<Option<project_manager::Resource>> {
        resource
            .as_ref()
            .map(|resource| self.validate_mandatory_resource(&Some(resource.clone())))
            .transpose()
    }

    pub fn validate_component_member(
        &self,
        component_id: &Option<String>,
        member_name: &Option<String>,
        member_type: project_manager::MemberType,
        expected_value_type: Option<&str>,
        optional: bool,
    ) -> ValidationResult<Option<Type>> {
        if component_id.is_none() && member_name.as_deref().is_none_or(str::is_empty) {
            if optional {
                return Ok(None);
            }
            self.add_error("Le composant n'est pas défini.");
            return Err(InvalidForDeployment);
        }

        let Some(member_name) = member_name.as_deref().filter(|name| !name.is_empty()) else {
            self.add_error("Le composant n'est pas défini.");
            return Err(InvalidForDeployment);
        };

        if component_id.is_none() && self.owner_template.is_none() {
            self.add_error("Le composant n'est pas défini.");
            return Err(InvalidForDeployment);
        }

        let value_type = if let Some(component_id) = component_id {
            if !self.project.components.contains_key(component_id) {
                self.add_error(format!("Le composant '{}' n'existe pas.", component_id));
                return Err(InvalidForDeployment);
            }

            self.project
                .find_component_member_value_type(component_id, member_name, member_type)
                .ok()
                .cloned()
        } else {
            self.owner_template.as_ref().and_then(|template| {
                template.exports.get(member_name).and_then(|export| {
                    (export.member_type == member_type).then(|| export.value_type.clone())
                })
            })
        };

        let Some(value_type) = value_type else {
            let member = match member_type {
                project_manager::MemberType::State => "L'état",
                project_manager::MemberType::Action => "L'action",
            };
            let suffix = component_id
                .as_deref()
                .map(|id| format!(" du composant '{}'", id))
                .unwrap_or_else(|| " de l'export template".to_owned());
            self.add_error(format!("{member} '{member_name}'{suffix} n'existe pas."));
            return Err(InvalidForDeployment);
        };

        if expected_value_type.is_some_and(|expected| expected != value_type) {
            let member = match member_type {
                project_manager::MemberType::State => "L'état",
                project_manager::MemberType::Action => "L'action",
            };
            let suffix = component_id
                .as_deref()
                .map(|id| format!(" du composant '{}'", id))
                .unwrap_or_else(|| " de l'export template".to_owned());
            self.add_error(format!(
                "{member} '{member_name}'{suffix} a un type incorrect : '{}'.",
                expected_value_type.unwrap()
            ));
            return Err(InvalidForDeployment);
        }

        let Ok(value_type) = Type::from_str(&value_type) else {
            self.add_error(format!("Le type '{}' est incorrect.", value_type));
            return Err(InvalidForDeployment);
        };

        Ok(Some(value_type))
    }

    pub fn resolve_component_member(
        &self,
        component_id: &Option<String>,
        member_name: &Option<String>,
    ) -> Option<(String, String)> {
        if let (Some(component_id), Some(member_name)) = (component_id, member_name) {
            return Some((component_id.clone(), member_name.clone()));
        }

        let member_name = member_name.as_ref()?;
        let binding = self.bindings.get(member_name)?;
        let (Some(component_id), Some(member_name)) =
            (binding.component_id.clone(), binding.member_name.clone())
        else {
            return None;
        };

        Some((component_id, member_name))
    }

    pub fn with_owner_template(
        &self,
        owner_template: Option<Arc<project_manager::UiTemplateData>>,
    ) -> Self {
        Self {
            owner_template,
            ..self.clone()
        }
    }

    pub fn with_bindings(
        &self,
        bindings: HashMap<String, project_manager::UiTemplateInstanceBinding>,
    ) -> Self {
        Self {
            bindings: Arc::new(bindings),
            ..self.clone()
        }
    }

    pub fn at(&self, node_type: &str, id: &str) -> Self {
        let mut path = self.path.clone();
        path.push(Arc::new(project_manager::UiElementPathNode {
            r#type: node_type.to_string(),
            id: id.to_string(),
        }));

        Self {
            path,
            ..self.clone()
        }
    }

    pub fn errors(&self) -> Vec<project_manager::UiValidationError> {
        self.errors.borrow().clone()
    }
}
