use std::{collections::HashMap, sync::Arc};

use studio_web_api::{project_manager, utils::PartialField};

use crate::services::project_manager::{
    ProjectManagerActorError,
    ui_project::{
        components,
        utils::{UpdateContext, check_id},
    },
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

fn update_window(
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

pub fn new_template(ctx: &mut UpdateContext, id: String) -> Result<(), ProjectManagerActorError> {
    check_id(&id)?;

    if ctx.data().templates.contains_key(&id) {
        return Err(ProjectManagerActorError::DataProcessingError(format!(
            "Template with id '{}' already exists",
            id
        )));
    }

    let template = Arc::new(project_manager::UiTemplateData {
        height: 500,
        width: 500,
        exports: Arc::new(HashMap::new()),
        controls: Arc::new(HashMap::new()),
        templates: Arc::new(HashMap::new()),
    });

    let mut new_templates = ctx.data().templates.as_ref().clone();
    new_templates.insert(id.clone(), template.clone());

    ctx.set_data(Arc::new(project_manager::UiProject {
        templates: Arc::new(new_templates),
        ..ctx.data().as_ref().clone()
    }));

    ctx.push_notification(project_manager::UpdateProjectNotification::SetUiTemplate(
        project_manager::SetUiTemplateNotification { id, template },
    ));

    Ok(())
}

pub fn clear_template(ctx: &mut UpdateContext, id: String) -> Result<(), ProjectManagerActorError> {
    let mut new_templates = ctx.data().templates.as_ref().clone();
    if new_templates.remove(&id).is_none() {
        return Ok(());
    }

    ctx.set_data(Arc::new(project_manager::UiProject {
        templates: Arc::new(new_templates),
        ..ctx.data().as_ref().clone()
    }));

    ctx.push_notification(project_manager::UpdateProjectNotification::ClearUiTemplate(
        project_manager::ClearUiTemplateNotification { id },
    ));

    // TODO: impacts

    Ok(())
}

pub fn rename_template(
    ctx: &mut UpdateContext,
    id: String,
    new_id: String,
) -> Result<(), ProjectManagerActorError> {
    check_id(&new_id)?;

    if id == new_id {
        return Ok(());
    }

    if ctx.data().templates.contains_key(&new_id) {
        return Err(ProjectManagerActorError::DataProcessingError(format!(
            "Template with id '{}' already exists",
            new_id
        )));
    }

    let mut new_templates = ctx.data().templates.as_ref().clone();
    let Some(template) = new_templates.remove(&id) else {
        return Err(ProjectManagerActorError::DataProcessingError(format!(
            "Template with id '{}' does not exist",
            id
        )));
    };

    new_templates.insert(new_id.clone(), template.clone());

    ctx.set_data(Arc::new(project_manager::UiProject {
        templates: Arc::new(new_templates),
        ..ctx.data().as_ref().clone()
    }));

    ctx.push_notification(
        project_manager::UpdateProjectNotification::RenameUiTemplate(
            project_manager::RenameUiTemplateNotification { id, new_id },
        ),
    );

    // TODO: impacts

    Ok(())
}

pub fn clone_template(
    ctx: &mut UpdateContext,
    id: String,
    new_id: String,
) -> Result<(), ProjectManagerActorError> {
    check_id(&new_id)?;

    let mut new_templates = ctx.data().templates.as_ref().clone();
    let Some(template) = new_templates.get(&id).cloned() else {
        return Err(ProjectManagerActorError::DataProcessingError(format!(
            "Template with id '{}' does not exist",
            id
        )));
    };

    if new_templates.contains_key(&new_id) {
        return Err(ProjectManagerActorError::DataProcessingError(format!(
            "Template with id '{}' already exists",
            new_id
        )));
    }

    new_templates.insert(new_id.clone(), template.clone());

    ctx.set_data(Arc::new(project_manager::UiProject {
        templates: Arc::new(new_templates),
        ..ctx.data().as_ref().clone()
    }));

    ctx.push_notification(project_manager::UpdateProjectNotification::SetUiTemplate(
        project_manager::SetUiTemplateNotification {
            id: new_id,
            template,
        },
    ));

    Ok(())
}

fn update_template(
    ctx: &mut UpdateContext,
    id: String,
    mutator: impl FnOnce(
        &UpdateContext,
        &mut project_manager::UiTemplateData,
    ) -> Result<bool, ProjectManagerActorError>,
) -> Result<(), ProjectManagerActorError> {
    let mut new_templates = ctx.data().templates.as_ref().clone();
    let Some(template) = new_templates.get(&id).cloned() else {
        return Err(ProjectManagerActorError::DataProcessingError(format!(
            "Template with id '{}' does not exist",
            id
        )));
    };

    let mut template = template.as_ref().clone();
    let changed = mutator(ctx, &mut template)?;

    if !changed {
        return Ok(());
    }

    let template = Arc::new(template);
    new_templates.insert(id.clone(), template.clone());

    ctx.set_data(Arc::new(project_manager::UiProject {
        templates: Arc::new(new_templates),
        ..ctx.data().as_ref().clone()
    }));

    ctx.push_notification(project_manager::UpdateProjectNotification::SetUiTemplate(
        project_manager::SetUiTemplateNotification { id, template },
    ));

    Ok(())
}

pub fn set_template_properties(
    ctx: &mut UpdateContext,
    id: String,
    properties: project_manager::UiTemplateProperties,
) -> Result<(), ProjectManagerActorError> {
    update_template(ctx, id, |_, template| {
        let mut changed = false;

        if let PartialField::Value(height) = properties.height {
            template.height = height;
            changed = true;
        }
        if let PartialField::Value(width) = properties.width {
            template.width = width;
            changed = true;
        }

        Ok(changed)
    })
}

pub fn set_template_export(
    ctx: &mut UpdateContext,
    id: String,
    export_id: String,
    member_type: project_manager::MemberType,
    value_type: String,
) -> Result<(), ProjectManagerActorError> {
    update_template(ctx, id, |_, template| {
        let mut new_exports = template.exports.as_ref().clone();
        let export = Arc::new(project_manager::UiTemplateExport {
            bulk_pattern: None,
            description: None,
            member_type,
            value_type,
        });

        new_exports.insert(export_id.clone(), export);
        template.exports = Arc::new(new_exports);

        Ok(true)
    })?;

    // TODO: impacts

    Ok(())
}

pub fn clear_template_export(
    ctx: &mut UpdateContext,
    id: String,
    export_id: String,
) -> Result<(), ProjectManagerActorError> {
    update_template(ctx, id, |_, template| {
        let mut new_exports = template.exports.as_ref().clone();
        new_exports.remove(&export_id);
        template.exports = Arc::new(new_exports);

        // TODO: inner controls + template_instances

        Ok(true)
    })?;

    // TODO: impacts

    Ok(())
}

pub fn set_template_bulk_patterns(
    ctx: &mut UpdateContext,
    id: String,
    patterns: HashMap<String, String>,
) -> Result<(), ProjectManagerActorError> {
    update_template(ctx, id.clone(), |_, template| {
        let mut changed = false;
        let mut new_exports = template.exports.as_ref().clone();

        for (key, value) in patterns.iter() {
            let Some(export) = new_exports.get(key) else {
                return Err(ProjectManagerActorError::DataProcessingError(format!(
                    "Export '{}' does not exist for template '{}'",
                    key, id
                )));
            };

            let mut export = export.as_ref().clone();
            export.bulk_pattern = Some(value.clone());
            new_exports.insert(key.clone(), Arc::new(export));
            changed = true;
        }

        template.exports = Arc::new(new_exports);
        Ok(changed)
    })
}

fn prepare_template_bindings(
    ctx: &UpdateContext,
    id: &str,
) -> Result<
    Arc<HashMap<String, Arc<project_manager::UiTemplateInstanceBinding>>>,
    ProjectManagerActorError,
> {
    let template = get_template(ctx, id)?;

    let mut bindings = HashMap::new();
    let binding = Arc::new(project_manager::UiTemplateInstanceBinding {
        component_id: None,
        member_name: None,
    });

    for export_id in template.exports.keys() {
        bindings.insert(export_id.clone(), binding.clone());
    }

    Ok(Arc::new(bindings))
}

fn get_template_export_value_type(
    ctx: &UpdateContext,
    id: &str,
    member_name: &str,
    member_type: project_manager::MemberType,
) -> Result<Option<String>, ProjectManagerActorError> {
    let template = get_template(ctx, id)?;

    let Some(export) = template.exports.get(member_name) else {
        return Ok(None);
    };

    if export.member_type != member_type {
        return Ok(None);
    }

    Ok(Some(export.value_type.clone()))
}

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
        project_manager::ViewType::Window => update_window(ctx, view_id, |ctx, window| {
            let mut new_controls = window.controls.as_ref().clone();
            let changed = mutator(ctx, &mut new_controls)?;
            if changed {
                window.controls = Arc::new(new_controls);
            }
            Ok(changed)
        }),
        project_manager::ViewType::Template => update_template(ctx, view_id, |ctx, template| {
            let mut new_controls = template.controls.as_ref().clone();
            let changed = mutator(ctx, &mut new_controls)?;
            if changed {
                template.controls = Arc::new(new_controls);
            }
            Ok(changed)
        }),
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
        project_manager::ViewType::Window => update_window(ctx, view_id, |ctx, window| {
            let mut new_templates = window.templates.as_ref().clone();
            let changed = mutator(ctx, &mut new_templates)?;
            if changed {
                window.templates = Arc::new(new_templates);
            }
            Ok(changed)
        }),
        project_manager::ViewType::Template { .. } => {
            update_template(ctx, view_id, |ctx, template| {
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

    let bindings = prepare_template_bindings(ctx, &id)?;

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
    let bindings = prepare_template_bindings(ctx, &template_id)?;

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

        let template = get_template(ctx, &template_instance.template_id)?.clone();

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
                get_template_export_value_type(
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

fn get_window<'a>(
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

fn get_template<'a>(
    ctx: &'a UpdateContext,
    id: &str,
) -> Result<&'a Arc<project_manager::UiTemplateData>, ProjectManagerActorError> {
    let Some(template) = ctx.data().templates.get(id) else {
        return Err(ProjectManagerActorError::DataProcessingError(format!(
            "Template '{}' does not exist",
            id
        )));
    };

    Ok(template)
}

fn get_view_control<'a>(
    ctx: &'a UpdateContext,
    view_type: project_manager::ViewType,
    view_id: &str,
    id: &str,
) -> Result<&'a Arc<project_manager::UiControlData>, ProjectManagerActorError> {
    match view_type {
        project_manager::ViewType::Window => {
            let window = get_window(ctx, view_id)?;
            let Some(control) = window.controls.get(id) else {
                return Err(ProjectManagerActorError::DataProcessingError(format!(
                    "Control '{}' does not exist in window '{}'",
                    id, view_id
                )));
            };

            Ok(control)
        }
        project_manager::ViewType::Template => {
            let template = get_template(ctx, view_id)?;
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
            let window = get_window(ctx, view_id)?;
            let Some(template_instance) = window.templates.get(id) else {
                return Err(ProjectManagerActorError::DataProcessingError(format!(
                    "Template instance '{}' does not exist in window '{}'",
                    id, view_id
                )));
            };

            Ok(template_instance)
        }
        project_manager::ViewType::Template => {
            let template = get_template(ctx, view_id)?;
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
