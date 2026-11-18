use std::{collections::HashMap, sync::Arc};

use studio_web_api::{project_manager, ui_model, utils::PartialField};

use crate::services::project_manager::{
    ProjectManagerActorError,
    ui_project::{
        control::ControlOps,
        template_instance::TemplateInstanceOps,
        utils::ArcMapOps,
        validation::{Validation, ValidationResult},
        view::{DeployPosition, View},
    },
};

pub trait TemplateOps {
    fn deploy(
        &self,
        validation: &Validation,
        position: &DeployPosition,
    ) -> ValidationResult<Vec<ui_model::Control>>;

    fn new() -> Arc<project_manager::UiTemplateData>;

    fn set_properties(
        &self,
        properties: project_manager::UiTemplateProperties,
    ) -> Option<Arc<project_manager::UiTemplateData>>;

    fn set_export(
        &self,
        export_id: String,
        member_type: project_manager::MemberType,
        value_type: String,
    ) -> Arc<project_manager::UiTemplateData>;

    fn clear_export(&self, export_id: String) -> Option<Arc<project_manager::UiTemplateData>>;

    fn set_bulk_patterns(
        &self,
        patterns: HashMap<String, String>,
    ) -> Result<Option<Arc<project_manager::UiTemplateData>>, ProjectManagerActorError>;

    fn prepare_bindings(
        &self,
    ) -> Arc<HashMap<String, Arc<project_manager::UiTemplateInstanceBinding>>>;

    fn get_export_value_type(
        &self,
        member_name: &str,
        member_type: project_manager::MemberType,
    ) -> Result<&String, ProjectManagerActorError>;

    fn set_exports(
        &self,
        exports: HashMap<String, Arc<project_manager::UiTemplateExport>>,
    ) -> Arc<project_manager::UiTemplateData>;
}

impl TemplateOps for Arc<project_manager::UiTemplateData> {
    fn deploy(
        &self,
        validation: &Validation,
        position: &DeployPosition,
    ) -> ValidationResult<Vec<ui_model::Control>> {
        let view = View::template(self.clone());

        view.deploy_controls(
            &validation.with_owner_template(Some(self.clone())),
            position,
        )
    }

    fn new() -> Arc<project_manager::UiTemplateData> {
        Arc::new(project_manager::UiTemplateData {
            height: 500,
            width: 500,
            exports: Arc::new(HashMap::new()),
            controls: Arc::new(HashMap::new()),
            templates: Arc::new(HashMap::new()),
        })
    }

    fn set_properties(
        &self,
        properties: project_manager::UiTemplateProperties,
    ) -> Option<Arc<project_manager::UiTemplateData>> {
        let mut template = self.as_ref().clone();
        let mut changed = false;

        if let PartialField::Value(height) = properties.height {
            template.height = height;
            changed = true;
        }
        if let PartialField::Value(width) = properties.width {
            template.width = width;
            changed = true;
        }

        if changed {
            Some(Arc::new(template))
        } else {
            None
        }
    }

    fn set_export(
        &self,
        export_id: String,
        member_type: project_manager::MemberType,
        value_type: String,
    ) -> Arc<project_manager::UiTemplateData> {
        let export = Arc::new(project_manager::UiTemplateExport {
            bulk_pattern: None,
            description: None,
            member_type,
            value_type,
        });

        let mut exports = self.exports.as_ref().clone();
        exports.insert(export_id.clone(), export);
        self.set_exports(exports)
    }

    fn clear_export(&self, export_id: String) -> Option<Arc<project_manager::UiTemplateData>> {
        let mut exports = self.exports.as_ref().clone();
        if exports.remove(&export_id).is_none() {
            return None;
        }

        let mut template = self.as_ref().clone();
        template.exports = Arc::new(exports);

        if let Some(controls) = self
            .controls
            .with_values_updated(|_, control| control.on_clear_template_export(&export_id))
        {
            template.controls = controls;
        }
        if let Some(templates) = self
            .templates
            .with_values_updated(|_, instance| instance.on_clear_template_export(&export_id))
        {
            template.templates = templates;
        }

        Some(Arc::new(template))
    }

    fn set_bulk_patterns(
        &self,
        patterns: HashMap<String, String>,
    ) -> Result<Option<Arc<project_manager::UiTemplateData>>, ProjectManagerActorError> {
        let mut exports = self.exports.as_ref().clone();
        let mut changed = false;

        for (key, value) in patterns.iter() {
            let Some(export) = exports.get(key) else {
                return Err(ProjectManagerActorError::DataProcessingError(format!(
                    "Export '{}' does not exist on template",
                    key
                )));
            };

            let mut export = export.as_ref().clone();
            export.bulk_pattern = Some(value.clone());
            exports.insert(key.clone(), Arc::new(export));
            changed = true;
        }

        if changed {
            Ok(Some(self.set_exports(exports)))
        } else {
            Ok(None)
        }
    }

    fn prepare_bindings(
        &self,
    ) -> Arc<HashMap<String, Arc<project_manager::UiTemplateInstanceBinding>>> {
        let mut bindings = HashMap::new();
        let binding = Arc::new(project_manager::UiTemplateInstanceBinding {
            component_id: None,
            member_name: None,
        });

        for export_id in self.exports.keys() {
            bindings.insert(export_id.clone(), binding.clone());
        }

        Arc::new(bindings)
    }

    fn get_export_value_type(
        &self,
        member_name: &str,
        member_type: project_manager::MemberType,
    ) -> Result<&String, ProjectManagerActorError> {
        let Some(export) = self.exports.get(member_name) else {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Export '{}' does not exist on template",
                member_name
            )));
        };

        if export.member_type != member_type {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Export '{}' has mismatched member type",
                member_name
            )));
        }

        Ok(&export.value_type)
    }

    fn set_exports(
        &self,
        exports: HashMap<String, Arc<project_manager::UiTemplateExport>>,
    ) -> Arc<project_manager::UiTemplateData> {
        let mut template = self.as_ref().clone();
        template.exports = Arc::new(exports);
        Arc::new(template)
    }
}
