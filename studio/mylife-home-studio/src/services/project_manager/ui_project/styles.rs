use std::sync::Arc;

use studio_web_api::project_manager;

use crate::services::project_manager::{
    ProjectManagerActorError,
    ui_project::utils::{UpdateContext, check_id},
};

pub fn initial_notifications(ctx: &mut UpdateContext) {
    for (id, style) in ctx.data().styles.clone().iter() {
        ctx.push_notification(project_manager::UpdateProjectNotification::SetUiStyle(
            project_manager::SetUiStyleNotification {
                id: id.clone(),
                style: style.clone(),
            },
        ));
    }
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
