use std::sync::Arc;

use studio_web_api::project_manager;

use crate::services::project_manager::{
    ProjectManagerActorError,
    ui_project::old::{
        control::ControlOperation, operation::ProjectOperation, path::path_node,
        view::ViewOperation,
    },
};

#[derive(Debug, Clone)]
pub struct ComponentUsage {
    pub component_id: String,
    pub member_name: String,
    pub path: project_manager::UiElementPath,
}

pub trait ComponentOperation {
    /// Note: ctx.data() will not be modified by this function
    fn component_initial_notifications(&mut self);

    fn component_find_member_value_type(
        &self,
        component_id: &str,
        member_name: &str,
        member_type: project_manager::MemberType,
    ) -> Option<String>;

    fn component_collect_usage(&self) -> Vec<ComponentUsage>;
    fn component_clear_usage(
        &mut self,
        usage: &ComponentUsage,
    ) -> Result<(), ProjectManagerActorError>;
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
        component_find_member_value_type(self.data(), component_id, member_name, member_type)
    }

    fn component_collect_usage(&self) -> Vec<ComponentUsage> {
        let mut usages = Vec::new();

        for (view_type, view_id, controls, instances) in self
            .data()
            .windows
            .iter()
            .map(|(id, data)| ("window", id, &data.controls, &data.templates))
            .chain(
                self.data()
                    .templates
                    .iter()
                    .map(|(id, data)| ("template", id, &data.controls, &data.templates)),
            )
        {
            for (control_id, control) in controls.iter() {
                let control_path = vec![
                    path_node(view_type, view_id),
                    path_node("control", control_id),
                ];

                if let Some(display) = control.display.as_ref().as_ref()
                    && let (Some(component_id), Some(member_name)) =
                        (&display.component_id, &display.component_state)
                {
                    usages.push(ComponentUsage {
                        component_id: component_id.clone(),
                        member_name: member_name.clone(),
                        path: control_path.clone(),
                    });
                }

                if let Some(text) = control.text.as_ref().as_ref() {
                    for (index, item) in text.context.iter().enumerate() {
                        if let (Some(component_id), Some(member_name)) =
                            (&item.component_id, &item.component_state)
                        {
                            let mut path = control_path.clone();
                            path.push(path_node("context-item", &index.to_string()));
                            usages.push(ComponentUsage {
                                component_id: component_id.clone(),
                                member_name: member_name.clone(),
                                path,
                            });
                        }
                    }
                }

                for (action_type, action) in [
                    ("primaryAction", control.primary_action.as_ref()),
                    ("secondaryAction", control.secondary_action.as_ref()),
                ] {
                    let Some(action) = action.as_ref() else {
                        continue;
                    };
                    let Some(component) = &action.component else {
                        continue;
                    };
                    if let (Some(component_id), Some(member_name)) =
                        (&component.id, &component.action)
                    {
                        let mut path = control_path.clone();
                        path.push(path_node("action", action_type));
                        usages.push(ComponentUsage {
                            component_id: component_id.clone(),
                            member_name: member_name.clone(),
                            path,
                        });
                    }
                }
            }

            for (instance_id, instance) in instances.iter() {
                for (binding_id, binding) in instance.bindings.iter() {
                    if let (Some(component_id), Some(member_name)) =
                        (&binding.component_id, &binding.member_name)
                    {
                        usages.push(ComponentUsage {
                            component_id: component_id.clone(),
                            member_name: member_name.clone(),
                            path: vec![
                                path_node(view_type, view_id),
                                path_node("template-instance", instance_id),
                                path_node("template-binding", binding_id),
                            ],
                        });
                    }
                }
            }
        }

        usages
    }

    fn component_clear_usage(
        &mut self,
        usage: &ComponentUsage,
    ) -> Result<(), ProjectManagerActorError> {
        let Some(view) = usage.path.first() else {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Invalid component usage path: '{:?}' (missing view type)",
                usage.path
            )));
        };
        let Some(element) = usage.path.get(1) else {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Invalid component usage path: '{:?}' (missing element)",
                usage.path
            )));
        };

        let view_type = match view.r#type.as_str() {
            "window" => project_manager::ViewType::Window,
            "template" => project_manager::ViewType::Template,
            _ => {
                return Err(ProjectManagerActorError::DataProcessingError(format!(
                    "Invalid view type '{}' in component usage path: '{:?}'",
                    view.r#type, usage.path
                )));
            }
        };

        match element.r#type.as_str() {
            "control" => {
                let view_id = view.id.clone();
                let control_id = element.id.clone();
                let component_id = usage.component_id.clone();
                let member_name = usage.member_name.clone();

                self.view_update_controls(view_type, view_id, |operation, controls| {
                    let Some(control) = controls.get_mut(&control_id) else {
                        return Ok(false);
                    };

                    let mut updated = control.as_ref().clone();
                    let changed = operation.control_clear_component_usage(
                        &mut updated,
                        &component_id,
                        &member_name,
                    );
                    if changed {
                        *control = Arc::new(updated);
                    }
                    Ok(changed)
                })?;
            }
            "template-instance" => {
                let view_id = view.id.clone();
                let instance_id = element.id.clone();
                let component_id = usage.component_id.clone();
                let member_name = usage.member_name.clone();
                let binding_id = usage
                    .path
                    .get(2)
                    .filter(|node| node.r#type == "template-binding")
                    .map(|node| node.id.clone());

                self.view_update_template_instances(view_type, view_id, |_, instances| {
                    let Some(instance) = instances.get_mut(&instance_id) else {
                        return Ok(false);
                    };

                    let mut updated = instance.as_ref().clone();
                    let mut bindings = updated.bindings.as_ref().clone();
                    let mut changed = false;

                    for (id, binding) in bindings.iter_mut() {
                        if binding_id
                            .as_ref()
                            .is_some_and(|binding_id| binding_id != id)
                            || binding.component_id.as_deref() != Some(component_id.as_str())
                            || binding.member_name.as_deref() != Some(member_name.as_str())
                        {
                            continue;
                        }

                        let mut updated_binding = binding.as_ref().clone();
                        updated_binding.component_id = None;
                        updated_binding.member_name = None;
                        *binding = Arc::new(updated_binding);
                        changed = true;
                    }

                    if changed {
                        updated.bindings = Arc::new(bindings);
                        *instance = Arc::new(updated);
                    }

                    Ok(changed)
                })?;
            }
            _ => {
                return Err(ProjectManagerActorError::DataProcessingError(format!(
                    "Invalid element type '{}' in component usage path: '{:?}'",
                    element.r#type, usage.path,
                )));
            }
        }

        Ok(())
    }
}

/// Also used by validation
pub fn component_find_member_value_type(
    data: &project_manager::UiProject,
    component_id: &str,
    member_name: &str,
    member_type: project_manager::MemberType,
) -> Option<String> {
    let Some(component) = data.components.get(component_id) else {
        return None;
    };

    let Some(plugin) = data.plugins.get(&component.plugin) else {
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
