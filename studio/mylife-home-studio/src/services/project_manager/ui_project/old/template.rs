use std::{collections::HashMap, sync::Arc};

use studio_web_api::{project_manager, utils::PartialField};

use crate::services::project_manager::{
    ProjectManagerActorError,
    ui_project::old::{
        control::ControlOperation, operation::ProjectOperation, view::ViewOperation,
    },
};

pub trait TemplateOperation {
    /// Note: self.data() will not be modified by this function
    fn template_initial_notifications(&mut self);

    fn template_new(&mut self, id: String) -> Result<(), ProjectManagerActorError>;
    fn template_clear(&mut self, id: String) -> Result<(), ProjectManagerActorError>;
    fn template_rename(
        &mut self,
        id: String,
        new_id: String,
    ) -> Result<(), ProjectManagerActorError>;
    fn template_clone(
        &mut self,
        id: String,
        new_id: String,
    ) -> Result<(), ProjectManagerActorError>;
    fn template_set_properties(
        &mut self,
        id: String,
        properties: project_manager::UiTemplateProperties,
    ) -> Result<(), ProjectManagerActorError>;
    fn template_set_export(
        &mut self,
        id: String,
        export_id: String,
        member_type: project_manager::MemberType,
        value_type: String,
    ) -> Result<(), ProjectManagerActorError>;
    fn template_clear_export(
        &mut self,
        id: String,
        export_id: String,
    ) -> Result<(), ProjectManagerActorError>;
    fn template_set_bulk_patterns(
        &mut self,
        id: String,
        patterns: HashMap<String, String>,
    ) -> Result<(), ProjectManagerActorError>;
    fn template_update(
        &mut self,
        id: String,
        mutator: impl FnOnce(
            &ProjectOperation,
            &mut project_manager::UiTemplateData,
        ) -> Result<bool, ProjectManagerActorError>,
    ) -> Result<(), ProjectManagerActorError>;
    fn template_get(
        &self,
        id: &str,
    ) -> Result<&Arc<project_manager::UiTemplateData>, ProjectManagerActorError>;
    fn template_prepare_bindings(
        &self,
        id: &str,
    ) -> Result<
        Arc<HashMap<String, Arc<project_manager::UiTemplateInstanceBinding>>>,
        ProjectManagerActorError,
    >;
    fn template_get_export_value_type(
        &self,
        id: &str,
        member_name: &str,
        member_type: project_manager::MemberType,
    ) -> Result<Option<String>, ProjectManagerActorError>;
}

impl TemplateOperation for ProjectOperation {
    fn template_initial_notifications(&mut self) {
        for (id, template) in self.data().templates.clone().iter() {
            self.push_notification(project_manager::UpdateProjectNotification::SetUiTemplate(
                project_manager::SetUiTemplateNotification {
                    id: id.clone(),
                    template: template.clone(),
                },
            ));
        }
    }

    fn template_new(&mut self, id: String) -> Result<(), ProjectManagerActorError> {
        self.check_id(&id)?;

        if self.data().templates.contains_key(&id) {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Template with id '{}' already exists",
                id
            )));
        }

        let template = Arc::new(project_manager::UiTemplateData {
            height: 500,
            width: 500,
            exports: Arc::new(HashMap::new()),
            controls: Arc::new(HashMap::new()),
            templates: Arc::new(HashMap::new()),
        });

        let mut new_templates = self.data().templates.as_ref().clone();
        new_templates.insert(id.clone(), template.clone());

        self.set_data(Arc::new(project_manager::UiProject {
            templates: Arc::new(new_templates),
            ..self.data().as_ref().clone()
        }));

        self.push_notification(project_manager::UpdateProjectNotification::SetUiTemplate(
            project_manager::SetUiTemplateNotification { id, template },
        ));

        Ok(())
    }

    fn template_clear(&mut self, id: String) -> Result<(), ProjectManagerActorError> {
        let mut new_templates = self.data().templates.as_ref().clone();
        if new_templates.remove(&id).is_none() {
            return Ok(());
        }

        self.set_data(Arc::new(project_manager::UiProject {
            templates: Arc::new(new_templates),
            ..self.data().as_ref().clone()
        }));

        self.push_notification(project_manager::UpdateProjectNotification::ClearUiTemplate(
            project_manager::ClearUiTemplateNotification { id: id.clone() },
        ));

        self.view_on_change_template(&id, None)?;

        Ok(())
    }

    fn template_rename(
        &mut self,
        id: String,
        new_id: String,
    ) -> Result<(), ProjectManagerActorError> {
        self.check_id(&new_id)?;

        if id == new_id {
            return Ok(());
        }

        if self.data().templates.contains_key(&new_id) {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Template with id '{}' already exists",
                new_id
            )));
        }

        let mut new_templates = self.data().templates.as_ref().clone();
        let Some(template) = new_templates.remove(&id) else {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Template with id '{}' does not exist",
                id
            )));
        };

        new_templates.insert(new_id.clone(), template.clone());

        self.set_data(Arc::new(project_manager::UiProject {
            templates: Arc::new(new_templates),
            ..self.data().as_ref().clone()
        }));

        self.push_notification(
            project_manager::UpdateProjectNotification::RenameUiTemplate(
                project_manager::RenameUiTemplateNotification {
                    id: id.clone(),
                    new_id: new_id.clone(),
                },
            ),
        );

        self.view_on_change_template(&id, Some(&new_id))?;

        Ok(())
    }

    fn template_clone(
        &mut self,
        id: String,
        new_id: String,
    ) -> Result<(), ProjectManagerActorError> {
        self.check_id(&new_id)?;

        let mut new_templates = self.data().templates.as_ref().clone();
        let Some(template) = new_templates.get(&id).cloned() else {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Template with id '{}' does not exist",
                id
            )));
        };

        if new_templates.contains_key(&new_id) {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Template with id '{}' already exists",
                new_id
            )));
        }

        new_templates.insert(new_id.clone(), template.clone());

        self.set_data(Arc::new(project_manager::UiProject {
            templates: Arc::new(new_templates),
            ..self.data().as_ref().clone()
        }));

        self.push_notification(project_manager::UpdateProjectNotification::SetUiTemplate(
            project_manager::SetUiTemplateNotification {
                id: new_id,
                template,
            },
        ));

        Ok(())
    }

    fn template_set_properties(
        &mut self,
        id: String,
        properties: project_manager::UiTemplateProperties,
    ) -> Result<(), ProjectManagerActorError> {
        self.template_update(id, |_, template| {
            let mut changed = false;

            if let PartialField::Value(height) = properties.height {
                template.height = height;
                changed = true;
            }
            if let PartialField::Value(width) = properties.width {
                template.width = width;
                changed = true;
            }

            Ok(changed)
        })
    }

    fn template_set_export(
        &mut self,
        id: String,
        export_id: String,
        member_type: project_manager::MemberType,
        value_type: String,
    ) -> Result<(), ProjectManagerActorError> {
        self.template_update(id.clone(), |_, template| {
            let mut new_exports = template.exports.as_ref().clone();
            let export = Arc::new(project_manager::UiTemplateExport {
                bulk_pattern: None,
                description: None,
                member_type,
                value_type,
            });

            new_exports.insert(export_id.clone(), export);
            template.exports = Arc::new(new_exports);

            Ok(true)
        })?;

        self.view_on_change_template_export(&id, &export_id, true)?;

        Ok(())
    }

    fn template_clear_export(
        &mut self,
        id: String,
        export_id: String,
    ) -> Result<(), ProjectManagerActorError> {
        self.template_update(id.clone(), |self_, template| {
            let mut new_exports = template.exports.as_ref().clone();
            new_exports.remove(&export_id);
            template.exports = Arc::new(new_exports);

            let mut controls = template.controls.as_ref().clone();
            let mut controls_changed = false;
            for control in controls.values_mut() {
                let mut updated = control.as_ref().clone();
                if self_.control_on_clear_template_export(&mut updated, &export_id)? {
                    *control = Arc::new(updated);
                    controls_changed = true;
                }
            }
            if controls_changed {
                template.controls = Arc::new(controls);
            }

            let mut instances = template.templates.as_ref().clone();
            let mut instances_changed = false;
            for instance in instances.values_mut() {
                let mut updated = instance.as_ref().clone();
                let mut bindings = updated.bindings.as_ref().clone();
                if bindings.remove(&export_id).is_some() {
                    updated.bindings = Arc::new(bindings);
                    *instance = Arc::new(updated);
                    instances_changed = true;
                }
            }
            if instances_changed {
                template.templates = Arc::new(instances);
            }

            Ok(true)
        })?;

        self.view_on_change_template_export(&id, &export_id, false)?;

        Ok(())
    }

    fn template_set_bulk_patterns(
        &mut self,
        id: String,
        patterns: HashMap<String, String>,
    ) -> Result<(), ProjectManagerActorError> {
        self.template_update(id.clone(), |_, template| {
            let mut changed = false;
            let mut new_exports = template.exports.as_ref().clone();

            for (key, value) in patterns.iter() {
                let Some(export) = new_exports.get(key) else {
                    return Err(ProjectManagerActorError::DataProcessingError(format!(
                        "Export '{}' does not exist for template '{}'",
                        key, id
                    )));
                };

                let mut export = export.as_ref().clone();
                export.bulk_pattern = Some(value.clone());
                new_exports.insert(key.clone(), Arc::new(export));
                changed = true;
            }

            template.exports = Arc::new(new_exports);
            Ok(changed)
        })
    }

    fn template_update(
        &mut self,
        id: String,
        mutator: impl FnOnce(
            &ProjectOperation,
            &mut project_manager::UiTemplateData,
        ) -> Result<bool, ProjectManagerActorError>,
    ) -> Result<(), ProjectManagerActorError> {
        let mut new_templates = self.data().templates.as_ref().clone();
        let Some(template) = new_templates.get(&id).cloned() else {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Template with id '{}' does not exist",
                id
            )));
        };

        let mut template = template.as_ref().clone();
        let changed = mutator(self, &mut template)?;

        if !changed {
            return Ok(());
        }

        let template = Arc::new(template);
        new_templates.insert(id.clone(), template.clone());

        self.set_data(Arc::new(project_manager::UiProject {
            templates: Arc::new(new_templates),
            ..self.data().as_ref().clone()
        }));

        self.push_notification(project_manager::UpdateProjectNotification::SetUiTemplate(
            project_manager::SetUiTemplateNotification { id, template },
        ));

        Ok(())
    }

    fn template_get(
        &self,
        id: &str,
    ) -> Result<&Arc<project_manager::UiTemplateData>, ProjectManagerActorError> {
        let Some(template) = self.data().templates.get(id) else {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Template '{}' does not exist",
                id
            )));
        };

        Ok(template)
    }

    fn template_prepare_bindings(
        &self,
        id: &str,
    ) -> Result<
        Arc<HashMap<String, Arc<project_manager::UiTemplateInstanceBinding>>>,
        ProjectManagerActorError,
    > {
        let template = self.template_get(id)?;

        let mut bindings = HashMap::new();
        let binding = Arc::new(project_manager::UiTemplateInstanceBinding {
            component_id: None,
            member_name: None,
        });

        for export_id in template.exports.keys() {
            bindings.insert(export_id.clone(), binding.clone());
        }

        Ok(Arc::new(bindings))
    }

    fn template_get_export_value_type(
        &self,
        id: &str,
        member_name: &str,
        member_type: project_manager::MemberType,
    ) -> Result<Option<String>, ProjectManagerActorError> {
        let template = self.template_get(id)?;

        let Some(export) = template.exports.get(member_name) else {
            return Ok(None);
        };

        if export.member_type != member_type {
            return Ok(None);
        }

        Ok(Some(export.value_type.clone()))
    }
}
