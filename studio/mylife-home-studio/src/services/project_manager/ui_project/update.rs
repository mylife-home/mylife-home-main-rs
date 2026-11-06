use std::{collections::HashMap, sync::Arc};

use studio_web_api::{project_manager, utils::PartialField};

use crate::services::project_manager::ProjectManagerActorError;

pub struct UpdateContext {
    notifications: Vec<project_manager::UpdateProjectNotification>,
    data: Arc<project_manager::UiProject>,
}

impl UpdateContext {
    pub fn new(data: &Arc<project_manager::UiProject>) -> Self {
        Self {
            notifications: Vec::new(),
            data: data.clone(),
        }
    }

    pub fn output(
        self,
    ) -> (
        Vec<project_manager::UpdateProjectNotification>,
        Arc<project_manager::UiProject>,
    ) {
        (self.notifications, self.data)
    }

    pub fn push_notification(&mut self, notification: project_manager::UpdateProjectNotification) {
        self.notifications.push(notification);
    }

    pub fn data(&self) -> &Arc<project_manager::UiProject> {
        &self.data
    }

    pub fn set_data(&mut self, data: Arc<project_manager::UiProject>) {
        self.data = data;
    }
}

/// Note: ctx.data() will not be modified by this function
pub fn initial_notifications(ctx: &mut UpdateContext) {
    ctx.push_notification(
        project_manager::UpdateProjectNotification::SetUiDefaultWindow(
            project_manager::SetUiDefaultWindowNotification {
                default_window: ctx.data().default_window.clone(),
            },
        ),
    );

    ctx.push_notification(
        project_manager::UpdateProjectNotification::SetUiComponentData(
            project_manager::SetUiComponentDataNotification {
                components: ctx.data().components.clone(),
                plugins: ctx.data().plugins.clone(),
            },
        ),
    );

    /*
        notifier.notify({ operation: 'set-ui-component-data', components: this.project.components, plugins: this.project.plugins } as SetUiComponentDataNotification);
    */

    for (id, resource) in ctx.data().resources.clone().iter() {
        ctx.push_notification(project_manager::UpdateProjectNotification::SetUiResource(
            project_manager::SetUiResourceNotification {
                id: id.clone(),
                resource: resource.clone(),
            },
        ));
    }

    for (id, style) in ctx.data().styles.clone().iter() {
        ctx.push_notification(project_manager::UpdateProjectNotification::SetUiStyle(
            project_manager::SetUiStyleNotification {
                id: id.clone(),
                style: style.clone(),
            },
        ));
    }

    for (id, window) in ctx.data().windows.clone().iter() {
        ctx.push_notification(project_manager::UpdateProjectNotification::SetUiWindow(
            project_manager::SetUiWindowNotification {
                id: id.clone(),
                window: window.clone(),
            },
        ));
    }

    for (id, template) in ctx.data().templates.clone().iter() {
        ctx.push_notification(project_manager::UpdateProjectNotification::SetUiTemplate(
            project_manager::SetUiTemplateNotification {
                id: id.clone(),
                template: template.clone(),
            },
        ));
    }
}

pub fn set_default_window(
    ctx: &mut UpdateContext,
    default_window: project_manager::UiDefaultWindowData,
) -> Result<(), ProjectManagerActorError> {
    let mut new_map = ctx.data().default_window.0.clone();

    for (key, value) in default_window.0 {
        new_map.insert(key.clone(), value);
    }

    ctx.set_data(Arc::new(project_manager::UiProject {
        default_window: Arc::new(project_manager::UiDefaultWindowData(new_map)),
        ..ctx.data().as_ref().clone()
    }));

    ctx.push_notification(
        project_manager::UpdateProjectNotification::SetUiDefaultWindow(
            project_manager::SetUiDefaultWindowNotification {
                default_window: ctx.data().default_window.clone(),
            },
        ),
    );

    Ok(())
}

pub fn set_resource(
    ctx: &mut UpdateContext,
    id: String,
    resource: project_manager::UiResourceData,
) -> Result<(), ProjectManagerActorError> {
    check_id(&id)?;

    let resource = Arc::new(resource);
    let mut new_resources = ctx.data().resources.as_ref().clone();
    new_resources.insert(id.clone(), resource.clone());

    ctx.set_data(Arc::new(project_manager::UiProject {
        resources: Arc::new(new_resources),
        ..ctx.data().as_ref().clone()
    }));

    ctx.push_notification(project_manager::UpdateProjectNotification::SetUiResource(
        project_manager::SetUiResourceNotification { id, resource },
    ));

    Ok(())
}

pub fn clear_resource(ctx: &mut UpdateContext, id: String) -> Result<(), ProjectManagerActorError> {
    let mut new_resources = ctx.data().resources.as_ref().clone();
    if new_resources.remove(&id).is_none() {
        return Ok(());
    }

    ctx.set_data(Arc::new(project_manager::UiProject {
        resources: Arc::new(new_resources),
        ..ctx.data().as_ref().clone()
    }));

    ctx.push_notification(project_manager::UpdateProjectNotification::ClearUiResource(
        project_manager::ClearUiResourceNotification { id },
    ));

    Ok(())
}

pub fn rename_resource(
    ctx: &mut UpdateContext,
    id: String,
    new_id: String,
) -> Result<(), ProjectManagerActorError> {
    check_id(&new_id)?;

    if id == new_id {
        return Ok(());
    }

    if ctx.data().resources.contains_key(&new_id) {
        return Err(ProjectManagerActorError::DataProcessingError(format!(
            "Resource with id '{}' already exists",
            new_id
        )));
    }

    let mut new_resources = ctx.data().resources.as_ref().clone();

    let Some(resource) = new_resources.remove(&id) else {
        return Err(ProjectManagerActorError::DataProcessingError(format!(
            "Resource with id '{}' not found",
            id
        )));
    };

    new_resources.insert(new_id.clone(), resource);

    ctx.set_data(Arc::new(project_manager::UiProject {
        resources: Arc::new(new_resources),
        ..ctx.data().as_ref().clone()
    }));

    ctx.push_notification(
        project_manager::UpdateProjectNotification::RenameUiResource(
            project_manager::RenameUiResourceNotification { id, new_id },
        ),
    );

    Ok(())
}

pub fn set_style(
    ctx: &mut UpdateContext,
    id: String,
    style: project_manager::UiStyleData,
) -> Result<(), ProjectManagerActorError> {
    check_id(&id)?;

    let style = Arc::new(style);
    let mut new_styles = ctx.data().styles.as_ref().clone();
    new_styles.insert(id.clone(), style.clone());

    ctx.set_data(Arc::new(project_manager::UiProject {
        styles: Arc::new(new_styles),
        ..ctx.data().as_ref().clone()
    }));

    ctx.push_notification(project_manager::UpdateProjectNotification::SetUiStyle(
        project_manager::SetUiStyleNotification { id, style },
    ));

    Ok(())
}

pub fn clear_style(ctx: &mut UpdateContext, id: String) -> Result<(), ProjectManagerActorError> {
    let mut new_styles = ctx.data().styles.as_ref().clone();
    if new_styles.remove(&id).is_none() {
        return Ok(());
    }

    ctx.set_data(Arc::new(project_manager::UiProject {
        styles: Arc::new(new_styles),
        ..ctx.data().as_ref().clone()
    }));

    ctx.push_notification(project_manager::UpdateProjectNotification::ClearUiStyle(
        project_manager::ClearUiStyleNotification { id },
    ));

    Ok(())
}

pub fn rename_style(
    ctx: &mut UpdateContext,
    id: String,
    new_id: String,
) -> Result<(), ProjectManagerActorError> {
    check_id(&new_id)?;

    if id == new_id {
        return Ok(());
    }

    if ctx.data().styles.contains_key(&new_id) {
        return Err(ProjectManagerActorError::DataProcessingError(format!(
            "Style with id '{}' already exists",
            new_id
        )));
    }

    let mut new_styles = ctx.data().styles.as_ref().clone();

    let Some(style) = new_styles.remove(&id) else {
        return Err(ProjectManagerActorError::DataProcessingError(format!(
            "Style with id '{}' not found",
            id
        )));
    };

    new_styles.insert(new_id.clone(), style);

    ctx.set_data(Arc::new(project_manager::UiProject {
        styles: Arc::new(new_styles),
        ..ctx.data().as_ref().clone()
    }));

    ctx.push_notification(project_manager::UpdateProjectNotification::RenameUiStyle(
        project_manager::RenameUiStyleNotification { id, new_id },
    ));

    Ok(())
}

pub fn new_window(ctx: &mut UpdateContext, id: String) -> Result<(), ProjectManagerActorError> {
    check_id(&id)?;

    if ctx.data().windows.contains_key(&id) {
        return Err(ProjectManagerActorError::DataProcessingError(format!(
            "Window with id '{}' already exists",
            id
        )));
    }

    let window = Arc::new(project_manager::UiWindowData {
        style: project_manager::Style(vec![]),
        height: 500,
        width: 500,
        background_resource: None,
        controls: Arc::new(HashMap::new()),
        templates: Arc::new(HashMap::new()),
    });

    let mut new_windows = ctx.data().windows.as_ref().clone();
    new_windows.insert(id.clone(), window.clone());

    ctx.set_data(Arc::new(project_manager::UiProject {
        windows: Arc::new(new_windows),
        ..ctx.data().as_ref().clone()
    }));

    ctx.push_notification(project_manager::UpdateProjectNotification::SetUiWindow(
        project_manager::SetUiWindowNotification { id, window },
    ));

    Ok(())
}

pub fn clear_window(ctx: &mut UpdateContext, id: String) -> Result<(), ProjectManagerActorError> {
    let mut new_windows = ctx.data().windows.as_ref().clone();
    if new_windows.remove(&id).is_none() {
        return Ok(());
    }

    ctx.set_data(Arc::new(project_manager::UiProject {
        windows: Arc::new(new_windows),
        ..ctx.data().as_ref().clone()
    }));

    ctx.push_notification(project_manager::UpdateProjectNotification::ClearUiWindow(
        project_manager::ClearUiWindowNotification { id },
    ));

    Ok(())
}

pub fn rename_window(
    ctx: &mut UpdateContext,
    id: String,
    new_id: String,
) -> Result<(), ProjectManagerActorError> {
    check_id(&new_id)?;

    let mut new_windows = ctx.data().windows.as_ref().clone();
    if !new_windows.contains_key(&id) {
        return Err(ProjectManagerActorError::DataProcessingError(format!(
            "Window with id '{}' does not exist",
            id
        )));
    }

    if new_windows.contains_key(&new_id) {
        return Err(ProjectManagerActorError::DataProcessingError(format!(
            "Window with id '{}' already exists",
            new_id
        )));
    }

    let window = new_windows.remove(&id).unwrap();
    new_windows.insert(new_id.clone(), window.clone());

    ctx.set_data(Arc::new(project_manager::UiProject {
        windows: Arc::new(new_windows),
        ..ctx.data().as_ref().clone()
    }));

    ctx.push_notification(project_manager::UpdateProjectNotification::RenameUiWindow(
        project_manager::RenameUiWindowNotification { id, new_id },
    ));

    Ok(())
}

pub fn clone_window(
    ctx: &mut UpdateContext,
    id: String,
    new_id: String,
) -> Result<(), ProjectManagerActorError> {
    check_id(&new_id)?;

    let mut new_windows = ctx.data().windows.as_ref().clone();
    let Some(window) = new_windows.get(&id).cloned() else {
        return Err(ProjectManagerActorError::DataProcessingError(format!(
            "Window with id '{}' does not exist",
            id
        )));
    };

    if new_windows.contains_key(&new_id) {
        return Err(ProjectManagerActorError::DataProcessingError(format!(
            "Window with id '{}' already exists",
            new_id
        )));
    }

    new_windows.insert(new_id.clone(), window.clone());

    ctx.set_data(Arc::new(project_manager::UiProject {
        windows: Arc::new(new_windows),
        ..ctx.data().as_ref().clone()
    }));

    ctx.push_notification(project_manager::UpdateProjectNotification::SetUiWindow(
        project_manager::SetUiWindowNotification { id: new_id, window },
    ));

    Ok(())
}

fn update_window(
    ctx: &mut UpdateContext,
    id: String,
    mutator: impl FnOnce(&mut project_manager::UiWindowData) -> Result<bool, ProjectManagerActorError>,
) -> Result<(), ProjectManagerActorError> {
    let mut new_windows = ctx.data().windows.as_ref().clone();
    let Some(window) = new_windows.get(&id).cloned() else {
        return Err(ProjectManagerActorError::DataProcessingError(format!(
            "Window with id '{}' does not exist",
            id
        )));
    };

    let mut window = window.as_ref().clone();
    let changed = mutator(&mut window)?;

    if !changed {
        return Ok(());
    }

    let window = Arc::new(window);
    new_windows.insert(id.clone(), window.clone());

    ctx.set_data(Arc::new(project_manager::UiProject {
        windows: Arc::new(new_windows),
        ..ctx.data().as_ref().clone()
    }));

    ctx.push_notification(project_manager::UpdateProjectNotification::SetUiWindow(
        project_manager::SetUiWindowNotification { id, window },
    ));

    Ok(())
}

pub fn set_window_properties(
    ctx: &mut UpdateContext,
    id: String,
    properties: project_manager::UiWindowProperties,
) -> Result<(), ProjectManagerActorError> {
    update_window(ctx, id, |window| {
        let mut changed = false;

        if let PartialField::Value(height) = properties.height {
            window.height = height;
            changed = true;
        }
        if let PartialField::Value(width) = properties.width {
            window.width = width;
            changed = true;
        }
        if let PartialField::Value(style) = properties.style {
            window.style = style;
            changed = true;
        }
        if let PartialField::Value(background_resource) = properties.background_resource {
            window.background_resource = background_resource;
            changed = true;
        }

        Ok(changed)
    })
}

fn check_id(id: &str) -> Result<(), ProjectManagerActorError> {
    if id.trim().is_empty() {
        return Err(ProjectManagerActorError::DataProcessingError(String::from(
            "Id is empty",
        )));
    }

    if id.contains(':') {
        return Err(ProjectManagerActorError::DataProcessingError(format!(
            "Id '{}' contains forbidden character ':'",
            id
        )));
    }
    Ok(())
}
