use studio_web_api::project_manager;

use crate::services::project_manager::ui_project::operation::ProjectOperation;

pub trait ComponentOperation {
    /// Note: ctx.data() will not be modified by this function
    fn component_initial_notifications(&mut self);

    fn component_find_member_value_type(
        &self,
        component_id: &str,
        member_name: &str,
        member_type: project_manager::MemberType,
    ) -> Option<String>;
}

impl ComponentOperation for ProjectOperation {
    fn component_initial_notifications(&mut self) {
        self.push_notification(
            project_manager::UpdateProjectNotification::SetUiComponentData(
                project_manager::SetUiComponentDataNotification {
                    components: self.data().components.clone(),
                    plugins: self.data().plugins.clone(),
                },
            ),
        );
    }

    fn component_find_member_value_type(
        &self,
        component_id: &str,
        member_name: &str,
        member_type: project_manager::MemberType,
    ) -> Option<String> {
        let Some(component) = self.data().components.get(component_id) else {
            return None;
        };

        let Some(plugin) = self.data().plugins.get(&component.plugin) else {
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
}
