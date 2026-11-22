use std::{collections::HashMap, sync::Arc};

use studio_web_api::{project_manager, ui_model};

use crate::services::project_manager::{
    ProjectManagerActorError,
    ui_project::{
        component::{ComponentUsage, ComponentUsageCollector},
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

    fn with_updated_views(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        change: impl FnMut(View) -> Option<View>,
    ) -> Option<Arc<project_manager::UiProject>>;

    fn get_views(&self) -> impl Iterator<Item = (&String, View)>;

    fn collect_component_usage(&self, collector: &ComponentUsageCollector);

    fn clear_component_usage(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        usage: &Vec<ComponentUsage>,
    ) -> Result<Option<Arc<project_manager::UiProject>>, ProjectManagerActorError>;
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
        self.with_updated_views(notifications, |view| view.on_change_resource(id, new_id))
    }

    fn on_change_style(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: &str,
        new_id: Option<&str>,
    ) -> Option<Arc<project_manager::UiProject>> {
        self.with_updated_views(notifications, |view| view.on_change_style(id, new_id))
    }

    fn on_change_window(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        id: &str,
        new_id: Option<&str>,
    ) -> Option<Arc<project_manager::UiProject>> {
        let updated_views =
            self.with_updated_views(notifications, |view| view.on_change_window(id, new_id));
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
        self.with_updated_views(notifications, |view| view.on_change_template(id, new_id))
    }

    fn with_updated_views(
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

    fn get_views(&self) -> impl Iterator<Item = (&String, View)> {
        self.windows
            .iter()
            .map(|(id, window)| (id, View::window(window.clone())))
            .chain(
                self.templates
                    .iter()
                    .map(|(id, template)| (id, View::template(template.clone()))),
            )
    }

    fn collect_component_usage(&self, collector: &ComponentUsageCollector) {
        for (_, view) in self.get_views() {
            view.collect_component_usage(collector);
        }
    }

    fn clear_component_usage(
        &self,
        notifications: &mut Vec<project_manager::UpdateProjectNotification>,
        usage: &Vec<ComponentUsage>,
    ) -> Result<Option<Arc<project_manager::UiProject>>, ProjectManagerActorError> {
        let mut project = self.clone();

        for usage in usage {
            let Some(node) = usage.path.get(0) else {
                return Err(ProjectManagerActorError::DataProcessingError(format!(
                    "Malformed usage path: {:?}",
                    usage
                )));
            };

            let view_type = match node.r#type.as_str() {
                "window" => project_manager::ViewType::Window,
                "template" => project_manager::ViewType::Template,
                _ => {
                    return Err(ProjectManagerActorError::DataProcessingError(format!(
                        "Unknown view type: {:?}",
                        node.r#type
                    )));
                }
            };

            let view_id = node.id.clone();

            let view = self.get_view(view_type, &view_id)?;
            if let Some(view) = view.clear_component_usage(usage)? {
                project = project.update_view(notifications, view_id, view)?;
            }
        }

        Ok(if Arc::ptr_eq(&self, &project) {
            None
        } else {
            Some(project)
        })
    }
}
