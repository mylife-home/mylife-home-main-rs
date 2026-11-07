use std::sync::Arc;

use studio_web_api::project_manager;

use crate::services::project_manager::{
    ProjectManagerActorError,
    ui_project::utils::{UpdateContext, check_id},
};

/// Note: ctx.data() will not be modified by this function
pub fn initial_notifications(ctx: &mut UpdateContext) {
    for (id, resource) in ctx.data().resources.clone().iter() {
        ctx.push_notification(project_manager::UpdateProjectNotification::SetUiResource(
            project_manager::SetUiResourceNotification {
                id: id.clone(),
                resource: resource.clone(),
            },
        ));
    }
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
