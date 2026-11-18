use std::{collections::HashMap, sync::Arc};

use studio_web_api::{project_manager, ui_model};

use crate::services::project_manager::{
    ProjectManagerActorError,
    ui_project::{
        default_window::DefaultWindowOps,
        template::TemplateOps,
        utils::{ArcMapOps, check_id},
        validation::Validation,
        view::{DeployPosition, View},
        window::WindowOps,
    },
};

pub trait ProjectOps {
    fn deploy(&self) -> Result<ui_model::Definition, Vec<project_manager::UiValidationError>>;

    fn initial_notifications(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
    );

    fn set_default_window(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        default_window: project_manager::UiDefaultWindowData,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError>;

    fn set_resource(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        resource: project_manager::UiResourceData,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError>;

    fn clear_resource(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
    ) -> Result<Option<Arc<project_manager::UiProject>>, ProjectManagerActorError>;

    fn rename_resource(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        new_id: String,
    ) -> Result<Option<Arc<project_manager::UiProject>>, ProjectManagerActorError>;

    fn set_resources(
        &self,
        resources: Arc<HashMap<String, Arc<project_manager::UiResourceData>>>,
    ) -> Arc<project_manager::UiProject>;

    fn set_style(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        style: project_manager::UiStyleData,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError>;

    fn clear_style(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
    ) -> Result<Option<Arc<project_manager::UiProject>>, ProjectManagerActorError>;

    fn rename_style(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        new_id: String,
    ) -> Result<Option<Arc<project_manager::UiProject>>, ProjectManagerActorError>;

    fn set_styles(
        &self,
        styles: Arc<HashMap<String, Arc<project_manager::UiStyleData>>>,
    ) -> Arc<project_manager::UiProject>;

    fn new_window(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError>;

    fn clear_window(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
    ) -> Result<Option<Arc<project_manager::UiProject>>, ProjectManagerActorError>;

    fn rename_window(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        new_id: String,
    ) -> Result<Option<Arc<project_manager::UiProject>>, ProjectManagerActorError>;

    fn clone_window(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        new_id: String,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError>;

    fn update_window(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        window: Arc<project_manager::UiWindowData>,
    ) -> Result<Option<Arc<project_manager::UiProject>>, ProjectManagerActorError>;

    fn set_windows(
        &self,
        windows: Arc<HashMap<String, Arc<project_manager::UiWindowData>>>,
    ) -> Arc<project_manager::UiProject>;

    fn new_template(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError>;

    fn clear_template(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
    ) -> Result<Option<Arc<project_manager::UiProject>>, ProjectManagerActorError>;

    fn rename_template(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        new_id: String,
    ) -> Result<Option<Arc<project_manager::UiProject>>, ProjectManagerActorError>;

    fn clone_template(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        new_id: String,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError>;

    fn update_template(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        template: Arc<project_manager::UiTemplateData>,
    ) -> Result<Option<Arc<project_manager::UiProject>>, ProjectManagerActorError>;

    fn set_templates(
        &self,
        templates: Arc<HashMap<String, Arc<project_manager::UiTemplateData>>>,
    ) -> Arc<project_manager::UiProject>;

    fn get_view(
        &self,
        view_type: project_manager::ViewType,
        view_id: &str,
    ) -> Result<View, ProjectManagerActorError>;

    fn update_view(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        view_id: String,
        view: View,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError>;

    fn find_component_member_value_type(
        &self,
        component_id: &str,
        member_name: &str,
        member_type: project_manager::MemberType,
    ) -> Result<&String, ProjectManagerActorError>;

    fn on_change_resource(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: &str,
        new_id: Option<&str>,
    ) -> Option<Arc<project_manager::UiProject>>;

    fn on_change_style(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: &str,
        new_id: Option<&str>,
    ) -> Option<Arc<project_manager::UiProject>>;

    fn on_change_window(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: &str,
        new_id: Option<&str>,
    ) -> Option<Arc<project_manager::UiProject>>;

    fn on_change_template(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: &str,
        new_id: Option<&str>,
    ) -> Option<Arc<project_manager::UiProject>>;

    fn on_change_views(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        change: impl FnMut(View) -> Option<View>,
    ) -> Option<Arc<project_manager::UiProject>>;
}

impl ProjectOps for Arc<project_manager::UiProject> {
    fn deploy(&self) -> Result<ui_model::Definition, Vec<project_manager::UiValidationError>> {
        let validation = Validation::new(self.clone());

        let default_window = self.default_window.deploy(&validation);

        // Validate templates
        let templates =
            self.templates
                .deploy(&validation, "template", |validation, id, template| {
                    let bindings = template
                        .exports
                        .keys()
                        .map(|name| {
                            (
                                name.clone(),
                                project_manager::UiTemplateInstanceBinding {
                                    component_id: Some(name.clone()),
                                    member_name: Some(name.clone()),
                                },
                            )
                        })
                        .collect();
                    let validation = validation.enter_template(id)?.with_bindings(bindings);
                    template.deploy(&validation, &DeployPosition::default())
                });

        let windows = self
            .windows
            .deploy(&validation, "window", |validation, id, window| {
                window.deploy(id, validation)
            });

        let (Ok(default_window), Ok(_), Ok(windows)) = (default_window, templates, windows) else {
            return Err(validation.errors());
        };

        Ok(ui_model::Definition {
            resources: self
                .resources
                .iter()
                .map(|(id, resource)| ui_model::DefinitionResource {
                    id: id.clone(),
                    mime: resource.mime.clone(),
                    data: resource.data.clone(),
                })
                .collect(),
            styles: self
                .styles
                .iter()
                .map(|(id, style)| ui_model::DefinitionStyle {
                    id: id.clone(),
                    properties: style.properties.clone(),
                })
                .collect(),
            windows,
            default_window,
        })
    }

    fn initial_notifications(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
    ) {
        notifications.push(
            project_manager::UpdateProjectNotification::SetUiDefaultWindow(
                project_manager::SetUiDefaultWindowNotification {
                    default_window: self.default_window.clone(),
                },
            ),
        );

        notifications.push(
            project_manager::UpdateProjectNotification::SetUiComponentData(
                project_manager::SetUiComponentDataNotification {
                    components: self.components.clone(),
                    plugins: self.plugins.clone(),
                },
            ),
        );

        for (id, resource) in self.resources.iter() {
            notifications.push(project_manager::UpdateProjectNotification::SetUiResource(
                project_manager::SetUiResourceNotification {
                    id: id.clone(),
                    resource: resource.clone(),
                },
            ));
        }

        for (id, style) in self.styles.iter() {
            notifications.push(project_manager::UpdateProjectNotification::SetUiStyle(
                project_manager::SetUiStyleNotification {
                    id: id.clone(),
                    style: style.clone(),
                },
            ));
        }

        for (id, window) in self.windows.iter() {
            notifications.push(project_manager::UpdateProjectNotification::SetUiWindow(
                project_manager::SetUiWindowNotification {
                    id: id.clone(),
                    window: window.clone(),
                },
            ));
        }

        for (id, template) in self.templates.iter() {
            notifications.push(project_manager::UpdateProjectNotification::SetUiTemplate(
                project_manager::SetUiTemplateNotification {
                    id: id.clone(),
                    template: template.clone(),
                },
            ));
        }
    }

    fn set_default_window(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        default_window: project_manager::UiDefaultWindowData,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError> {
        let mut project = self.as_ref().clone();
        project.default_window = self.default_window.set(default_window);
        let project = Arc::new(project);

        notifications.push(
            project_manager::UpdateProjectNotification::SetUiDefaultWindow(
                project_manager::SetUiDefaultWindowNotification {
                    default_window: project.default_window.clone(),
                },
            ),
        );

        Ok(project)
    }

    fn set_resource(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        resource: project_manager::UiResourceData,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError> {
        check_id(&id)?;
        let resource = Arc::new(resource);
        let resources = self.resources.with_set(id.clone(), resource.clone())?;
        let project = self.set_resources(resources);

        notifications.push(project_manager::UpdateProjectNotification::SetUiResource(
            project_manager::SetUiResourceNotification { id, resource },
        ));

        Ok(project)
    }

    fn clear_resource(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
    ) -> Result<Option<Arc<project_manager::UiProject>>, ProjectManagerActorError> {
        let Some(resources) = self.resources.with_cleared(&id)? else {
            return Ok(None);
        };
        let project = self.set_resources(resources);

        notifications.push(project_manager::UpdateProjectNotification::ClearUiResource(
            project_manager::ClearUiResourceNotification { id: id.clone() },
        ));

        let project = project
            .on_change_resource(notifications, &id, None)
            .unwrap_or(project);

        Ok(Some(project))
    }

    fn rename_resource(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        new_id: String,
    ) -> Result<Option<Arc<project_manager::UiProject>>, ProjectManagerActorError> {
        let Some(resources) = self.resources.with_renamed(&id, &new_id)? else {
            return Ok(None);
        };
        let project = self.set_resources(resources);

        notifications.push(
            project_manager::UpdateProjectNotification::RenameUiResource(
                project_manager::RenameUiResourceNotification {
                    id: id.clone(),
                    new_id: new_id.clone(),
                },
            ),
        );

        let project = project
            .on_change_resource(notifications, &id, Some(&new_id))
            .unwrap_or(project);

        Ok(Some(project))
    }

    fn set_resources(
        &self,
        resources: Arc<HashMap<String, Arc<project_manager::UiResourceData>>>,
    ) -> Arc<project_manager::UiProject> {
        let mut project = self.as_ref().clone();
        project.resources = resources;
        Arc::new(project)
    }

    fn set_style(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        style: project_manager::UiStyleData,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError> {
        check_id(&id)?;
        let style = Arc::new(style);
        let styles = self.styles.with_set(id.clone(), style.clone())?;
        let project = self.set_styles(styles);

        notifications.push(project_manager::UpdateProjectNotification::SetUiStyle(
            project_manager::SetUiStyleNotification { id, style },
        ));

        Ok(project)
    }

    fn clear_style(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
    ) -> Result<Option<Arc<project_manager::UiProject>>, ProjectManagerActorError> {
        let Some(styles) = self.styles.with_cleared(&id)? else {
            return Ok(None);
        };
        let project = self.set_styles(styles);

        notifications.push(project_manager::UpdateProjectNotification::ClearUiStyle(
            project_manager::ClearUiStyleNotification { id: id.clone() },
        ));

        let project = project
            .on_change_style(notifications, &id, None)
            .unwrap_or(project);

        Ok(Some(project))
    }

    fn rename_style(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        new_id: String,
    ) -> Result<Option<Arc<project_manager::UiProject>>, ProjectManagerActorError> {
        let Some(styles) = self.styles.with_renamed(&id, &new_id)? else {
            return Ok(None);
        };
        let project = self.set_styles(styles);

        notifications.push(project_manager::UpdateProjectNotification::RenameUiStyle(
            project_manager::RenameUiStyleNotification {
                id: id.clone(),
                new_id: new_id.clone(),
            },
        ));

        let project = project
            .on_change_style(notifications, &id, Some(&new_id))
            .unwrap_or(project);

        Ok(Some(project))
    }

    fn set_styles(
        &self,
        styles: Arc<HashMap<String, Arc<project_manager::UiStyleData>>>,
    ) -> Arc<project_manager::UiProject> {
        let mut project = self.as_ref().clone();
        project.styles = styles;
        Arc::new(project)
    }

    fn new_window(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError> {
        let window = <Arc<project_manager::UiWindowData> as WindowOps>::new();
        let windows = self.windows.with_inserted(id.clone(), window.clone())?;
        let project = self.set_windows(windows);

        notifications.push(project_manager::UpdateProjectNotification::SetUiWindow(
            project_manager::SetUiWindowNotification { id, window },
        ));

        Ok(project)
    }

    fn clear_window(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
    ) -> Result<Option<Arc<project_manager::UiProject>>, ProjectManagerActorError> {
        let Some(windows) = self.windows.with_cleared(&id)? else {
            return Ok(None);
        };
        let project = self.set_windows(windows);

        notifications.push(project_manager::UpdateProjectNotification::ClearUiWindow(
            project_manager::ClearUiWindowNotification { id: id.clone() },
        ));

        let project = project
            .on_change_window(notifications, &id, None)
            .unwrap_or(project);

        Ok(Some(project))
    }

    fn rename_window(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        new_id: String,
    ) -> Result<Option<Arc<project_manager::UiProject>>, ProjectManagerActorError> {
        let Some(windows) = self.windows.with_renamed(&id, &new_id)? else {
            return Ok(None);
        };
        let project = self.set_windows(windows);

        notifications.push(project_manager::UpdateProjectNotification::RenameUiWindow(
            project_manager::RenameUiWindowNotification {
                id: id.clone(),
                new_id: new_id.clone(),
            },
        ));

        let project = project
            .on_change_window(notifications, &id, Some(&new_id))
            .unwrap_or(project);

        Ok(Some(project))
    }

    fn clone_window(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        new_id: String,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError> {
        let window = self.windows.get_required(&id)?.clone();
        let windows = self.windows.with_inserted(new_id.clone(), window.clone())?;
        let project = self.set_windows(windows);

        notifications.push(project_manager::UpdateProjectNotification::SetUiWindow(
            project_manager::SetUiWindowNotification { id: new_id, window },
        ));

        Ok(project)
    }

    fn update_window(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        window: Arc<project_manager::UiWindowData>,
    ) -> Result<Option<Arc<project_manager::UiProject>>, ProjectManagerActorError> {
        let windows = self.windows.with_updated(id.clone(), window.clone())?;
        let project = self.set_windows(windows);

        notifications.push(project_manager::UpdateProjectNotification::SetUiWindow(
            project_manager::SetUiWindowNotification { id, window },
        ));

        Ok(Some(project))
    }

    fn set_windows(
        &self,
        windows: Arc<HashMap<String, Arc<project_manager::UiWindowData>>>,
    ) -> Arc<project_manager::UiProject> {
        let mut project = self.as_ref().clone();
        project.windows = windows;
        Arc::new(project)
    }

    fn new_template(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError> {
        let template = <Arc<project_manager::UiTemplateData> as TemplateOps>::new();
        let templates = self.templates.with_inserted(id.clone(), template.clone())?;
        let project = self.set_templates(templates);

        notifications.push(project_manager::UpdateProjectNotification::SetUiTemplate(
            project_manager::SetUiTemplateNotification { id, template },
        ));

        Ok(project)
    }

    fn clear_template(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
    ) -> Result<Option<Arc<project_manager::UiProject>>, ProjectManagerActorError> {
        let Some(templates) = self.templates.with_cleared(&id)? else {
            return Ok(None);
        };
        let project = self.set_templates(templates);

        notifications.push(project_manager::UpdateProjectNotification::ClearUiTemplate(
            project_manager::ClearUiTemplateNotification { id: id.clone() },
        ));

        let project = project
            .on_change_template(notifications, &id, None)
            .unwrap_or(project);

        Ok(Some(project))
    }

    fn rename_template(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        new_id: String,
    ) -> Result<Option<Arc<project_manager::UiProject>>, ProjectManagerActorError> {
        let Some(templates) = self.templates.with_renamed(&id, &new_id)? else {
            return Ok(None);
        };
        let project = self.set_templates(templates);

        notifications.push(
            project_manager::UpdateProjectNotification::RenameUiTemplate(
                project_manager::RenameUiTemplateNotification {
                    id: id.clone(),
                    new_id: new_id.clone(),
                },
            ),
        );

        let project = project
            .on_change_template(notifications, &id, Some(&new_id))
            .unwrap_or(project);

        Ok(Some(project))
    }

    fn clone_template(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        new_id: String,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError> {
        let template = self.templates.get_required(&id)?.clone();
        let templates = self
            .templates
            .with_inserted(new_id.clone(), template.clone())?;
        let project = self.set_templates(templates);

        notifications.push(project_manager::UpdateProjectNotification::SetUiTemplate(
            project_manager::SetUiTemplateNotification {
                id: new_id,
                template,
            },
        ));

        Ok(project)
    }

    fn update_template(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: String,
        template: Arc<project_manager::UiTemplateData>,
    ) -> Result<Option<Arc<project_manager::UiProject>>, ProjectManagerActorError> {
        let templates = self.templates.with_updated(id.clone(), template.clone())?;
        let project = self.set_templates(templates);

        notifications.push(project_manager::UpdateProjectNotification::SetUiTemplate(
            project_manager::SetUiTemplateNotification { id, template },
        ));

        Ok(Some(project))
    }

    fn set_templates(
        &self,
        templates: Arc<HashMap<String, Arc<project_manager::UiTemplateData>>>,
    ) -> Arc<project_manager::UiProject> {
        let mut project = self.as_ref().clone();
        project.templates = templates;
        Arc::new(project)
    }

    fn get_view(
        &self,
        view_type: project_manager::ViewType,
        view_id: &str,
    ) -> Result<View, ProjectManagerActorError> {
        match view_type {
            project_manager::ViewType::Window => {
                let window = self.windows.get_required(view_id)?;
                Ok(View::Window(window.clone()))
            }
            project_manager::ViewType::Template => {
                let template = self.templates.get_required(view_id)?;
                Ok(View::Template(template.clone()))
            }
        }
    }

    fn update_view(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        view_id: String,
        view: View,
    ) -> Result<Arc<project_manager::UiProject>, ProjectManagerActorError> {
        match view {
            View::Window(window) => {
                let windows = self.windows.with_updated(view_id.clone(), window.clone())?;
                let mut project = self.as_ref().clone();
                project.windows = windows;
                let project = Arc::new(project);

                notifications.push(project_manager::UpdateProjectNotification::SetUiWindow(
                    project_manager::SetUiWindowNotification {
                        id: view_id,
                        window,
                    },
                ));

                Ok(project)
            }
            View::Template(template) => {
                let templates = self
                    .templates
                    .with_updated(view_id.clone(), template.clone())?;
                let mut project = self.as_ref().clone();
                project.templates = templates;
                let project = Arc::new(project);

                notifications.push(project_manager::UpdateProjectNotification::SetUiTemplate(
                    project_manager::SetUiTemplateNotification {
                        id: view_id,
                        template,
                    },
                ));

                Ok(project)
            }
        }
    }

    fn find_component_member_value_type(
        &self,
        component_id: &str,
        member_name: &str,
        member_type: project_manager::MemberType,
    ) -> Result<&String, ProjectManagerActorError> {
        let Some(component) = self.components.get(component_id) else {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Component '{}' not found",
                component_id
            )));
        };

        let Some(plugin) = self.plugins.get(&component.plugin) else {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Plugin '{}' not found for component '{}'",
                component.plugin, component_id
            )));
        };

        let Some(member) = plugin.members.get(member_name) else {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Member '{}' not found in plugin '{}' for component '{}'",
                member_name, component.plugin, component_id
            )));
        };

        if member.member_type != member_type {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Member '{}' has incorrect type in plugin '{}' for component '{}'",
                member_name, component.plugin, component_id
            )));
        }

        Ok(&member.value_type)
    }

    fn on_change_resource(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: &str,
        new_id: Option<&str>,
    ) -> Option<Arc<project_manager::UiProject>> {
        self.on_change_views(notifications, |view| view.on_change_resource(id, new_id))
    }

    fn on_change_style(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: &str,
        new_id: Option<&str>,
    ) -> Option<Arc<project_manager::UiProject>> {
        self.on_change_views(notifications, |view| view.on_change_style(id, new_id))
    }

    fn on_change_window(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: &str,
        new_id: Option<&str>,
    ) -> Option<Arc<project_manager::UiProject>> {
        let updated_views =
            self.on_change_views(notifications, |view| view.on_change_window(id, new_id));
        let updated_default = self.default_window.on_change_window(id, new_id);

        if updated_views.is_none() && updated_default.is_none() {
            return None;
        }

        let mut project = match updated_views {
            Some(project) => project.as_ref().clone(),
            None => self.as_ref().clone(),
        };
        if let Some(default_window) = updated_default {
            project.default_window = default_window;
            notifications.push(
                project_manager::UpdateProjectNotification::SetUiDefaultWindow(
                    project_manager::SetUiDefaultWindowNotification {
                        default_window: project.default_window.clone(),
                    },
                ),
            );
        }
        Some(Arc::new(project))
    }

    fn on_change_template(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: &str,
        new_id: Option<&str>,
    ) -> Option<Arc<project_manager::UiProject>> {
        self.on_change_views(notifications, |view| view.on_change_template(id, new_id))
    }

    fn on_change_views(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        mut change: impl FnMut(View) -> Option<View>,
    ) -> Option<Arc<project_manager::UiProject>> {
        let windows = self.windows.with_values_updated(|id, window| {
            let Some(view) = change(View::window(window.clone())) else {
                return None;
            };
            let View::Window(window) = view else {
                unreachable!("window transformation returned a template")
            };

            notifications.push(project_manager::UpdateProjectNotification::SetUiWindow(
                project_manager::SetUiWindowNotification {
                    id: id.clone(),
                    window: window.clone(),
                },
            ));
            Some(window)
        });

        let templates = self.templates.with_values_updated(|id, template| {
            let Some(view) = change(View::template(template.clone())) else {
                return None;
            };
            let View::Template(template) = view else {
                unreachable!("template transformation returned a window")
            };

            notifications.push(project_manager::UpdateProjectNotification::SetUiTemplate(
                project_manager::SetUiTemplateNotification {
                    id: id.clone(),
                    template: template.clone(),
                },
            ));
            Some(template)
        });

        if windows.is_none() && templates.is_none() {
            return None;
        }

        let mut project = self.as_ref().clone();
        if let Some(windows) = windows {
            project.windows = windows;
        }
        if let Some(templates) = templates {
            project.templates = templates;
        }
        Some(Arc::new(project))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::project_manager::ui_project::{
        control::ControlOps, new_ui_project_data, template_instance::TemplateInstanceOps,
    };
    use studio_web_api::component_model::Member;

    fn project_with_window() -> Arc<project_manager::UiProject> {
        let mut project = new_ui_project_data().as_ref().clone();
        project.windows = Arc::new(HashMap::from([(
            "home".into(),
            <Arc<project_manager::UiWindowData> as WindowOps>::new(),
        )]));
        project.default_window = Arc::new(project_manager::UiDefaultWindowData(HashMap::from([(
            "desktop".into(),
            Some("home".into()),
        )])));
        Arc::new(project)
    }

    #[test]
    fn prepares_nested_templates_with_bindings_and_offsets() {
        let mut project = project_with_window().as_ref().clone();
        project.components = Arc::new(HashMap::from([(
            "device".into(),
            Arc::new(project_manager::UiComponentData {
                plugin: "plugin".into(),
            }),
        )]));
        project.plugins = Arc::new(HashMap::from([(
            "plugin".into(),
            Arc::new(project_manager::UiPluginData {
                name: "plugin".into(),
                module: String::new(),
                version: String::new(),
                description: None,
                instance_name: String::new(),
                members: HashMap::from([(
                    "state".into(),
                    Member {
                        description: None,
                        member_type: project_manager::MemberType::State,
                        value_type: "bool".into(),
                    },
                )]),
            }),
        )]));
        let mut inner = <Arc<project_manager::UiTemplateData> as TemplateOps>::new()
            .as_ref()
            .clone();
        inner.exports = Arc::new(HashMap::from([(
            "value".into(),
            Arc::new(project_manager::UiTemplateExport {
                bulk_pattern: None,
                description: None,
                member_type: project_manager::MemberType::State,
                value_type: "bool".into(),
            }),
        )]));
        let mut control = <Arc<project_manager::UiControlData> as ControlOps>::new(
            3,
            4,
            project_manager::UiControlType::Text,
        )
        .as_ref()
        .clone();
        control.text = Some(Arc::new(project_manager::UiControlTextData {
            format: "value".into(),
            context: vec![project_manager::UiControlTextContextItemData {
                id: "value".into(),
                component_id: None,
                component_state: Some("value".into()),
                test_value: serde_json::Value::Null,
            }],
        }));
        inner.controls = Arc::new(HashMap::from([("label".into(), Arc::new(control))]));
        let inner = Arc::new(inner);
        let mut outer = <Arc<project_manager::UiTemplateData> as TemplateOps>::new()
            .as_ref()
            .clone();
        outer.exports = inner.exports.clone();
        let mut inner_instance =
            <Arc<project_manager::UiTemplateInstanceData> as TemplateInstanceOps>::new(
                "inner".into(),
                &inner,
                10,
                20,
            )
            .unwrap()
            .as_ref()
            .clone();
        inner_instance.bindings = Arc::new(HashMap::from([(
            "value".into(),
            Arc::new(project_manager::UiTemplateInstanceBinding {
                component_id: None,
                member_name: Some("value".into()),
            }),
        )]));
        outer.templates = Arc::new(HashMap::from([("nested".into(), Arc::new(inner_instance))]));
        let outer = Arc::new(outer);
        let mut outer_instance =
            <Arc<project_manager::UiTemplateInstanceData> as TemplateInstanceOps>::new(
                "outer".into(),
                &outer,
                100,
                200,
            )
            .unwrap()
            .as_ref()
            .clone();
        outer_instance.bindings = Arc::new(HashMap::from([(
            "value".into(),
            Arc::new(project_manager::UiTemplateInstanceBinding {
                component_id: Some("device".into()),
                member_name: Some("state".into()),
            }),
        )]));
        let mut window = project.windows["home"].as_ref().clone();
        window.templates = Arc::new(HashMap::from([(
            "instance".into(),
            Arc::new(outer_instance),
        )]));
        project.windows = Arc::new(HashMap::from([("home".into(), Arc::new(window))]));
        project.templates = Arc::new(HashMap::from([
            ("inner".into(), inner),
            ("outer".into(), outer),
        ]));

        let definition = Arc::new(project).deploy().unwrap();
        let control = &definition.windows[0].controls[0];
        assert_eq!(
            (&control.id, control.x, control.y),
            (&"instance:nested:label".to_string(), 113, 224)
        );
        let item = &control.text.as_ref().unwrap().context[0];
        assert_eq!(
            (&item.component_id, &item.component_state),
            (&"device".to_string(), &"state".to_string())
        );
    }

    #[test]
    fn accumulates_errors_across_siblings_and_unused_templates() {
        let mut project = project_with_window().as_ref().clone();
        let mut window = project.windows["home"].as_ref().clone();
        let mut control = <Arc<project_manager::UiControlData> as ControlOps>::new(
            0,
            0,
            project_manager::UiControlType::Display,
        )
        .as_ref()
        .clone();
        control.text = Some(Arc::new(project_manager::UiControlTextData {
            context: vec![],
            format: String::new(),
        }));
        control.primary_action = Some(Arc::new(project_manager::UiActionData {
            component: Some(project_manager::UiActionComponentData {
                id: Some("missing".into()),
                action: Some("action".into()),
            }),
            window: Some(project_manager::UiActionWindow {
                id: Some("missing".into()),
                popup: false,
            }),
        }));
        window.controls = Arc::new(HashMap::from([("bad".into(), Arc::new(control))]));
        window.background_resource = Some(project_manager::Resource("missing".into()));
        project.windows = Arc::new(HashMap::from([("home".into(), Arc::new(window))]));
        let mut template = <Arc<project_manager::UiTemplateData> as TemplateOps>::new()
            .as_ref()
            .clone();
        let mut bad = <Arc<project_manager::UiControlData> as ControlOps>::new(
            0,
            0,
            project_manager::UiControlType::Display,
        )
        .as_ref()
        .clone();
        bad.display = None;
        template.controls = Arc::new(HashMap::from([("bad".into(), Arc::new(bad))]));
        project.templates = Arc::new(HashMap::from([("unused".into(), Arc::new(template))]));
        let errors = Arc::new(project).deploy().unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.message.contains("ressource"))
        );
        assert!(
            errors
                .iter()
                .any(|error| error.message.contains("composant ou fenêtre"))
        );
        assert!(
            errors
                .iter()
                .any(|error| error.message.contains("composant 'missing'"))
        );
        assert!(
            errors
                .iter()
                .any(|error| error.message.contains("fenêtre 'missing'"))
        );
        assert_eq!(
            errors
                .iter()
                .filter(|error| error.message.contains("contrôle doit"))
                .count(),
            2
        );
    }

    #[test]
    fn rejects_recursive_templates() {
        let mut project = project_with_window().as_ref().clone();
        let template = <Arc<project_manager::UiTemplateData> as TemplateOps>::new();
        let instance = <Arc<project_manager::UiTemplateInstanceData> as TemplateInstanceOps>::new(
            "loop".into(),
            &template,
            0,
            0,
        )
        .unwrap();
        let mut template = template.as_ref().clone();
        template.templates = Arc::new(HashMap::from([("again".into(), instance)]));
        project.templates = Arc::new(HashMap::from([("loop".into(), Arc::new(template))]));
        let errors = Arc::new(project).deploy().unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.message.contains("récursif"))
        );
    }

    #[test]
    fn validates_display_members_and_accepts_window_actions() {
        let mut project = project_with_window().as_ref().clone();
        let mut window = project.windows["home"].as_ref().clone();
        let mut control = <Arc<project_manager::UiControlData> as ControlOps>::new(
            0,
            0,
            project_manager::UiControlType::Display,
        )
        .as_ref()
        .clone();
        control.primary_action = Some(Arc::new(project_manager::UiActionData {
            component: None,
            window: Some(project_manager::UiActionWindow {
                id: Some("home".into()),
                popup: true,
            }),
        }));
        window.controls = Arc::new(HashMap::from([(
            "control".into(),
            Arc::new(control.clone()),
        )]));
        project.windows = Arc::new(HashMap::from([("home".into(), Arc::new(window.clone()))]));
        let definition = Arc::new(project.clone()).deploy().unwrap();
        assert_eq!(
            definition.windows[0].controls[0]
                .primary_action
                .as_ref()
                .unwrap()
                .window
                .as_ref()
                .unwrap()
                .id,
            "home"
        );

        let mut display = control.display.as_ref().unwrap().as_ref().clone();
        display.component_id = Some("missing".into());
        display.component_state = Some("state".into());
        control.display = Some(Arc::new(display));
        window.controls = Arc::new(HashMap::from([("control".into(), Arc::new(control))]));
        project.windows = Arc::new(HashMap::from([("home".into(), Arc::new(window))]));
        let errors = Arc::new(project).deploy().unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.message.contains("composant 'missing'"))
        );
    }
}
