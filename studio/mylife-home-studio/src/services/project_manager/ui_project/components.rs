use studio_web_api::project_manager;

use crate::services::project_manager::ui_project::utils::UpdateContext;

/// Note: ctx.data() will not be modified by this function
pub fn initial_notifications(ctx: &mut UpdateContext) {
    ctx.push_notification(
        project_manager::UpdateProjectNotification::SetUiComponentData(
            project_manager::SetUiComponentDataNotification {
                components: ctx.data().components.clone(),
                plugins: ctx.data().plugins.clone(),
            },
        ),
    );
}

pub fn find_component_member_value_type(
    ctx: &UpdateContext,
    component_id: &str,
    member_name: &str,
    member_type: project_manager::MemberType,
) -> Option<String> {
    let Some(component) = ctx.data().components.get(component_id) else {
        return None;
    };

    let Some(plugin) = ctx.data().plugins.get(&component.plugin) else {
        return None;
    };

    let Some(member) = plugin.members.get(member_name) else {
        return None;
    };

    if member.member_type != member_type {
        return None;
    }

    Some(member.value_type.clone())
}
