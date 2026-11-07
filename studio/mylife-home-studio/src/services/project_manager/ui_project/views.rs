use std::{collections::HashMap, sync::Arc};

use studio_web_api::{project_manager, utils::PartialField};

use crate::services::project_manager::{
    ProjectManagerActorError,
    ui_project::{
        components, templates,
        utils::{UpdateContext, check_id},
        windows,
    },
};

fn update_view_controls(
    ctx: &mut UpdateContext,
    view_type: project_manager::ViewType,
    view_id: String,
    mutator: impl FnOnce(
        &UpdateContext,
        &mut HashMap<String, Arc<project_manager::UiControlData>>,
    ) -> Result<bool, ProjectManagerActorError>,
) -> Result<(), ProjectManagerActorError> {
    match view_type {
        project_manager::ViewType::Window => windows::update_window(ctx, view_id, |ctx, window| {
            let mut new_controls = window.controls.as_ref().clone();
            let changed = mutator(ctx, &mut new_controls)?;
            if changed {
                window.controls = Arc::new(new_controls);
            }
            Ok(changed)
        }),
        project_manager::ViewType::Template => {
            templates::update_template(ctx, view_id, |ctx, template| {
                let mut new_controls = template.controls.as_ref().clone();
                let changed = mutator(ctx, &mut new_controls)?;
                if changed {
                    template.controls = Arc::new(new_controls);
                }
                Ok(changed)
            })
        }
    }
}

pub fn new_control(
    ctx: &mut UpdateContext,
    view_type: project_manager::ViewType,
    view_id: String,
    id: String,
    x: i32,
    y: i32,
    r#type: project_manager::UiControlType,
) -> Result<(), ProjectManagerActorError> {
    check_id(&id)?;

    update_view_controls(ctx, view_type, view_id, |_, controls| {
        if controls.contains_key(&id) {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Control with id '{}' already exists",
                id
            )));
        }

        let (display, text) = match r#type {
            project_manager::UiControlType::Display => (
                Arc::new(Some(project_manager::ControlDisplay {
                    component_id: None,
                    component_state: None,
                    default_resource: None,
                    map: Vec::new(),
                })),
                Arc::new(None),
            ),
            project_manager::UiControlType::Text => (
                Arc::new(None),
                Arc::new(Some(project_manager::UiControlTextData {
                    context: Vec::new(),
                    format: String::from("return '';"),
                })),
            ),
        };

        let control = project_manager::UiControlData {
            x,
            y,
            style: project_manager::Style(Vec::new()),
            height: 50,
            width: 50,
            display,
            text,
            primary_action: Arc::new(None),
            secondary_action: Arc::new(None),
        };

        controls.insert(id, Arc::new(control));
        Ok(true)
    })
}

pub fn clear_control(
    ctx: &mut UpdateContext,
    view_type: project_manager::ViewType,
    view_id: String,
    id: String,
) -> Result<(), ProjectManagerActorError> {
    update_view_controls(ctx, view_type, view_id, |_, controls| {
        Ok(controls.remove(&id).is_some())
    })
}

pub fn rename_control(
    ctx: &mut UpdateContext,
    view_type: project_manager::ViewType,
    view_id: String,
    id: String,
    new_id: String,
) -> Result<(), ProjectManagerActorError> {
    check_id(&new_id)?;

    update_view_controls(ctx, view_type, view_id, |_, controls| {
        if controls.contains_key(&new_id) {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Control with id '{}' already exists",
                new_id
            )));
        }

        let Some(control) = controls.remove(&id) else {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Control with id '{}' does not exist",
                id
            )));
        };

        controls.insert(new_id, control);

        Ok(true)
    })
}

pub fn clone_control(
    ctx: &mut UpdateContext,
    view_type: project_manager::ViewType,
    view_id: String,
    id: String,
    new_id: String,
    target_view_type: project_manager::ViewType,
    target_view_id: String,
) -> Result<(), ProjectManagerActorError> {
    check_id(&new_id)?;

    let same_view = view_type == target_view_type && view_id == target_view_id;
    let source_control = get_view_control(ctx, view_type, &view_id, &id)?;
    let mut new_control = source_control.as_ref().clone();

    if same_view {
        new_control.x += 10;
        new_control.y += 10;
    } else {
        new_control.x = 10;
        new_control.y = 10;
    }

    let new_control = Arc::new(new_control);

    update_view_controls(ctx, target_view_type, target_view_id, |_, controls| {
        if controls.contains_key(&new_id) {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Control with id '{}' already exists",
                new_id
            )));
        }

        controls.insert(new_id, new_control);

        Ok(true)
    })
}

pub fn set_control_properties(
    ctx: &mut UpdateContext,
    view_type: project_manager::ViewType,
    view_id: String,
    id: String,
    properties: project_manager::UiControlProperties,
) -> Result<(), ProjectManagerActorError> {
    update_view_controls(ctx, view_type, view_id, |_, controls| {
        let Some(control) = controls.get_mut(&id) else {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Control with id '{}' does not exist",
                id
            )));
        };

        let mut changed = false;
        let mut control = control.as_ref().clone();

        if let PartialField::Value(mut style) = properties.style {
            style.0.sort();
            control.style = style;
            changed = true;
        }
        if let PartialField::Value(height) = properties.height {
            control.height = height;
            changed = true;
        }
        if let PartialField::Value(width) = properties.width {
            control.width = width;
            changed = true;
        }
        if let PartialField::Value(x) = properties.x {
            control.x = x;
            changed = true;
        }
        if let PartialField::Value(y) = properties.y {
            control.y = y;
            changed = true;
        }
        if let PartialField::Value(display) = properties.display {
            control.display = Arc::new(display);
            changed = true;
        }
        if let PartialField::Value(text) = properties.text {
            control.text = Arc::new(text);
            changed = true;
        }
        if let PartialField::Value(primary_action) = properties.primary_action {
            control.primary_action = Arc::new(primary_action);
            changed = true;
        }
        if let PartialField::Value(secondary_action) = properties.secondary_action {
            control.secondary_action = Arc::new(secondary_action);
            changed = true;
        }

        if !changed {
            return Ok(false);
        }

        controls.insert(id, Arc::new(control));
        Ok(true)
    })
}

fn update_view_template_instances(
    ctx: &mut UpdateContext,
    view_type: project_manager::ViewType,
    view_id: String,
    mutator: impl FnOnce(
        &UpdateContext,
        &mut HashMap<String, Arc<project_manager::UiTemplateInstanceData>>,
    ) -> Result<bool, ProjectManagerActorError>,
) -> Result<(), ProjectManagerActorError> {
    match view_type {
        project_manager::ViewType::Window => windows::update_window(ctx, view_id, |ctx, window| {
            let mut new_templates = window.templates.as_ref().clone();
            let changed = mutator(ctx, &mut new_templates)?;
            if changed {
                window.templates = Arc::new(new_templates);
            }
            Ok(changed)
        }),
        project_manager::ViewType::Template => {
            templates::update_template(ctx, view_id, |ctx, template| {
                let mut new_templates = template.templates.as_ref().clone();
                let changed = mutator(ctx, &mut new_templates)?;
                if changed {
                    template.templates = Arc::new(new_templates);
                }
                Ok(changed)
            })
        }
    }
}

pub fn new_template_instance(
    ctx: &mut UpdateContext,
    view_type: project_manager::ViewType,
    view_id: String,
    id: String,
    template_id: String,
    x: i32,
    y: i32,
) -> Result<(), ProjectManagerActorError> {
    check_id(&id)?;

    if !ctx.data().templates.contains_key(&template_id) {
        return Err(ProjectManagerActorError::DataProcessingError(format!(
            "Template '{}' does not exist",
            template_id
        )));
    }

    let bindings = templates::prepare_template_bindings(ctx, &id)?;

    update_view_template_instances(ctx, view_type, view_id, |_, templates| {
        if templates.contains_key(&id) {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Template instance with id '{}' already exists",
                id
            )));
        }

        let template = project_manager::UiTemplateInstanceData {
            template_id,
            x,
            y,
            bindings,
        };

        templates.insert(id, Arc::new(template));

        Ok(true)
    })?;

    Ok(())
}

pub fn clear_template_instance(
    ctx: &mut UpdateContext,
    view_type: project_manager::ViewType,
    view_id: String,
    id: String,
) -> Result<(), ProjectManagerActorError> {
    update_view_template_instances(ctx, view_type, view_id, |_, templates| {
        Ok(templates.remove(&id).is_some())
    })?;

    Ok(())
}

pub fn rename_template_instance(
    ctx: &mut UpdateContext,
    view_type: project_manager::ViewType,
    view_id: String,
    id: String,
    new_id: String,
) -> Result<(), ProjectManagerActorError> {
    check_id(&new_id)?;

    update_view_template_instances(ctx, view_type, view_id, |_, templates| {
        if templates.contains_key(&new_id) {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Template instance with id '{}' already exists",
                new_id
            )));
        }

        let Some(template) = templates.remove(&id) else {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Template instance with id '{}' does not exist",
                id
            )));
        };

        templates.insert(new_id, template);

        Ok(true)
    })?;

    Ok(())
}

pub fn clone_template_instance(
    ctx: &mut UpdateContext,
    view_type: project_manager::ViewType,
    view_id: String,
    id: String,
    new_id: String,
) -> Result<(), ProjectManagerActorError> {
    check_id(&new_id)?;

    update_view_template_instances(ctx, view_type, view_id, |_, templates| {
        if templates.contains_key(&new_id) {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Template instance with id '{}' already exists",
                new_id
            )));
        }

        let Some(source) = templates.get(&id) else {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Template instance with id '{}' does not exist",
                id
            )));
        };

        let mut new_template_instance = source.as_ref().clone();
        new_template_instance.x += 10;
        new_template_instance.y += 10;

        templates.insert(new_id, Arc::new(new_template_instance));

        Ok(true)
    })
}

pub fn move_template_instance(
    ctx: &mut UpdateContext,
    view_type: project_manager::ViewType,
    view_id: String,
    id: String,
    x: i32,
    y: i32,
) -> Result<(), ProjectManagerActorError> {
    update_view_template_instances(ctx, view_type, view_id, |_, templates| {
        let Some(template) = templates.get(&id) else {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Template instance with id '{}' does not exist",
                id
            )));
        };

        let mut template = template.as_ref().clone();
        template.x = x;
        template.y = y;

        templates.insert(id, Arc::new(template));

        Ok(true)
    })?;

    Ok(())
}

pub fn set_template_instance_template(
    ctx: &mut UpdateContext,
    view_type: project_manager::ViewType,
    view_id: String,
    id: String,
    template_id: String,
) -> Result<(), ProjectManagerActorError> {
    let bindings = templates::prepare_template_bindings(ctx, &template_id)?;

    update_view_template_instances(ctx, view_type, view_id, |_, templates| {
        let Some(template_instance) = templates.get(&id) else {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Template instance with id '{}' does not exist",
                id
            )));
        };

        let mut template_instance = template_instance.as_ref().clone();
        template_instance.template_id = template_id;
        // reset bindings
        template_instance.bindings = bindings;

        templates.insert(id, Arc::new(template_instance));

        Ok(true)
    })?;

    Ok(())
}

pub fn set_template_instance_bindings(
    ctx: &mut UpdateContext,
    view_type: project_manager::ViewType,
    view_id: String,
    id: String,
    bindings: HashMap<String, project_manager::UiTemplateInstanceBinding>,
) -> Result<(), ProjectManagerActorError> {
    update_view_template_instances(ctx, view_type, view_id.clone(), |ctx, templates| {
        let Some(template_instance) = templates.get(&id) else {
            return Err(ProjectManagerActorError::DataProcessingError(format!(
                "Template instance with id '{}' does not exist",
                id
            )));
        };

        let mut template_instance = template_instance.as_ref().clone();
        let mut new_bindings = template_instance.bindings.as_ref().clone();

        let template = templates::get_template(ctx, &template_instance.template_id)?.clone();

        for (
            export_id,
            project_manager::UiTemplateInstanceBinding {
                component_id,
                member_name,
            },
        ) in bindings
        {
            let Some(binding) = new_bindings.get_mut(&export_id) else {
                return Err(ProjectManagerActorError::DataProcessingError(format!(
                    "Binding from export '{}' not found on template '{}'",
                    export_id, id
                )));
            };

            let mut binding = binding.as_ref().clone();

            let Some(member_name) = member_name else {
                return Err(ProjectManagerActorError::DataProcessingError(format!(
                    "Member name not provided for export '{}'",
                    export_id
                )));
            };

            let Some(export_data) = template.exports.get(&export_id) else {
                return Err(ProjectManagerActorError::DataProcessingError(format!(
                    "Export '{}' does not exist on template '{}'",
                    export_id, template_instance.template_id
                )));
            };

            let value_type = if let Some(component_id) = &component_id {
                components::find_component_member_value_type(
                    ctx,
                    component_id,
                    &member_name,
                    export_data.member_type,
                )
            } else if view_type == project_manager::ViewType::Template {
                templates::get_template_export_value_type(
                    ctx,
                    &view_id,
                    &member_name,
                    export_data.member_type,
                )?
            } else {
                None
            };

            if value_type != Some(export_data.value_type.clone()) {
                return Err(ProjectManagerActorError::DataProcessingError(format!(
                    "Binding mismatch: cannot bind '{}.{}' on export '{}'",
                    component_id.as_deref().unwrap_or("<template>"),
                    member_name,
                    export_id
                )));
            }

            binding.component_id = component_id;
            binding.member_name = Some(member_name);
            new_bindings.insert(export_id, Arc::new(binding));
        }

        template_instance.bindings = Arc::new(new_bindings);
        templates.insert(id, Arc::new(template_instance));

        Ok(true)
    })?;

    Ok(())
}

fn get_view_control<'a>(
    ctx: &'a UpdateContext,
    view_type: project_manager::ViewType,
    view_id: &str,
    id: &str,
) -> Result<&'a Arc<project_manager::UiControlData>, ProjectManagerActorError> {
    match view_type {
        project_manager::ViewType::Window => {
            let window = windows::get_window(ctx, view_id)?;
            let Some(control) = window.controls.get(id) else {
                return Err(ProjectManagerActorError::DataProcessingError(format!(
                    "Control '{}' does not exist in window '{}'",
                    id, view_id
                )));
            };

            Ok(control)
        }
        project_manager::ViewType::Template => {
            let template = templates::get_template(ctx, view_id)?;
            let Some(control) = template.controls.get(id) else {
                return Err(ProjectManagerActorError::DataProcessingError(format!(
                    "Control '{}' does not exist in template '{}'",
                    id, view_id
                )));
            };

            Ok(control)
        }
    }
}

fn get_view_template_instance<'a>(
    ctx: &'a UpdateContext,
    view_type: project_manager::ViewType,
    view_id: &str,
    id: &str,
) -> Result<&'a Arc<project_manager::UiTemplateInstanceData>, ProjectManagerActorError> {
    match view_type {
        project_manager::ViewType::Window => {
            let window = windows::get_window(ctx, view_id)?;
            let Some(template_instance) = window.templates.get(id) else {
                return Err(ProjectManagerActorError::DataProcessingError(format!(
                    "Template instance '{}' does not exist in window '{}'",
                    id, view_id
                )));
            };

            Ok(template_instance)
        }
        project_manager::ViewType::Template => {
            let template = templates::get_template(ctx, view_id)?;
            let Some(template_instance) = template.templates.get(id) else {
                return Err(ProjectManagerActorError::DataProcessingError(format!(
                    "Template instance '{}' does not exist in template '{}'",
                    id, view_id
                )));
            };

            Ok(template_instance)
        }
    }
}
