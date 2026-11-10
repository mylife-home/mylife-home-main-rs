use std::{collections::HashMap, sync::Arc};

use studio_web_api::{project_manager, utils::PartialField};

use crate::services::project_manager::{ProjectManagerActorError, ui_project::control::ControlOps};

pub trait TemplateOps {
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
    ) -> Option<String>;

    fn set_exports(
        &self,
        exports: HashMap<String, Arc<project_manager::UiTemplateExport>>,
    ) -> Arc<project_manager::UiTemplateData>;
}

impl TemplateOps for Arc<project_manager::UiTemplateData> {
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

        let mut controls = self.controls.as_ref().clone();
        for (_, control) in controls.iter_mut() {
            if let Some(ctrl) = control.on_clear_template_export(&export_id) {
                *control = ctrl;
            }
        }

        // TODO: template instances
        /*
                let mut updated = instance.as_ref().clone();
                let mut bindings = updated.bindings.as_ref().clone();
                if bindings.remove(&export_id).is_some() {
                    updated.bindings = Arc::new(bindings);
                    *instance = Arc::new(updated);
                    instances_changed = true;
                }
        */

        let mut template = self.as_ref().clone();
        template.exports = Arc::new(exports);
        template.controls = Arc::new(controls);
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
    ) -> Option<String> {
        let Some(export) = self.exports.get(member_name) else {
            return None;
        };

        if export.member_type != member_type {
            return None;
        }

        Some(export.value_type.clone())
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
