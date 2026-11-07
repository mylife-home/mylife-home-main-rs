use std::{collections::HashMap, sync::Arc};

use studio_web_api::{project_manager, utils::PartialField};

use crate::services::project_manager::{
    ProjectManagerActorError,
    ui_project::utils::{UpdateContext, check_id},
};

/// Note: ctx.data() will not be modified by this function
pub fn initial_notifications(ctx: &mut UpdateContext) {
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

    // TODO: impacts

    Ok(())
}

pub fn rename_window(
    ctx: &mut UpdateContext,
    id: String,
    new_id: String,
) -> Result<(), ProjectManagerActorError> {
    check_id(&new_id)?;

    if id == new_id {
        return Ok(());
    }

    if ctx.data().windows.contains_key(&new_id) {
        return Err(ProjectManagerActorError::DataProcessingError(format!(
            "Window with id '{}' already exists",
            new_id
        )));
    }

    let mut new_windows = ctx.data().windows.as_ref().clone();
    let Some(window) = new_windows.remove(&id) else {
        return Err(ProjectManagerActorError::DataProcessingError(format!(
            "Window with id '{}' does not exist",
            id
        )));
    };

    new_windows.insert(new_id.clone(), window);

    ctx.set_data(Arc::new(project_manager::UiProject {
        windows: Arc::new(new_windows),
        ..ctx.data().as_ref().clone()
    }));

    ctx.push_notification(project_manager::UpdateProjectNotification::RenameUiWindow(
        project_manager::RenameUiWindowNotification { id, new_id },
    ));

    // TODO: impacts

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

pub fn set_window_properties(
    ctx: &mut UpdateContext,
    id: String,
    properties: project_manager::UiWindowProperties,
) -> Result<(), ProjectManagerActorError> {
    update_window(ctx, id, |_, window| {
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

pub fn update_window(
    ctx: &mut UpdateContext,
    id: String,
    mutator: impl FnOnce(
        &UpdateContext,
        &mut project_manager::UiWindowData,
    ) -> Result<bool, ProjectManagerActorError>,
) -> Result<(), ProjectManagerActorError> {
    let mut new_windows = ctx.data().windows.as_ref().clone();
    let Some(window) = new_windows.get(&id).cloned() else {
        return Err(ProjectManagerActorError::DataProcessingError(format!(
            "Window with id '{}' does not exist",
            id
        )));
    };

    let mut window = window.as_ref().clone();
    let changed = mutator(ctx, &mut window)?;

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

pub fn get_window<'a>(
    ctx: &'a UpdateContext,
    id: &str,
) -> Result<&'a Arc<project_manager::UiWindowData>, ProjectManagerActorError> {
    let Some(window) = ctx.data().windows.get(id) else {
        return Err(ProjectManagerActorError::DataProcessingError(format!(
            "Window '{}' does not exist",
            id
        )));
    };

    Ok(window)
}
