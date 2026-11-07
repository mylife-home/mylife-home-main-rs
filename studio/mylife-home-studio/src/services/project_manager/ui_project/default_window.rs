use std::sync::Arc;

use studio_web_api::project_manager;

use crate::services::project_manager::{
    ProjectManagerActorError, ui_project::utils::UpdateContext,
};

/// Note: ctx.data() will not be modified by this function
pub fn initial_notifications(ctx: &mut UpdateContext) {
    ctx.push_notification(
        project_manager::UpdateProjectNotification::SetUiDefaultWindow(
            project_manager::SetUiDefaultWindowNotification {
                default_window: ctx.data().default_window.clone(),
            },
        ),
    );
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
