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
    mutator: impl FnOnce(&mut project_manager::UiTemplateData) -> Result<bool, ProjectManagerActorError>,
) -> Result<(), ProjectManagerActorError> {
    let mut new_templates = ctx.data().templates.as_ref().clone();
    let Some(template) = new_templates.get(&id).cloned() else {
        return Err(ProjectManagerActorError::DataProcessingError(format!(
            "Template with id '{}' does not exist",
            id
        )));
    };

    let mut template = template.as_ref().clone();
    let changed = mutator(&mut template)?;

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
    update_template(ctx, id, |template| {
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
    update_template(ctx, id, |template| {
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
    update_template(ctx, id, |template| {
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
    update_template(ctx, id.clone(), |template| {
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

fn update_view_controls(
    ctx: &mut UpdateContext,
    view_type: project_manager::ViewType,
    view_id: String,
    mutator: impl FnOnce(
        &mut HashMap<String, Arc<project_manager::UiControlData>>,
    ) -> Result<bool, ProjectManagerActorError>,
) -> Result<(), ProjectManagerActorError> {
    match view_type {
        project_manager::ViewType::Window => update_window(ctx, view_id, |window| {
            let mut new_controls = window.controls.as_ref().clone();
            let changed = mutator(&mut new_controls)?;
            if changed {
                window.controls = Arc::new(new_controls);
            }
            Ok(changed)
        }),
        project_manager::ViewType::Template { .. } => update_template(ctx, view_id, |template| {
            let mut new_controls = template.controls.as_ref().clone();
            let changed = mutator(&mut new_controls)?;
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

    update_view_controls(ctx, view_type, view_id, |controls| {
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
    update_view_controls(ctx, view_type, view_id, |controls| {
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

    update_view_controls(ctx, view_type, view_id, |controls| {
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

    update_view_controls(ctx, target_view_type, target_view_id, |controls| {
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
    update_view_controls(ctx, view_type, view_id, |controls| {
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

fn get_window<'a>(
    ctx: &'a mut UpdateContext,
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
    ctx: &'a mut UpdateContext,
    view_id: &str,
) -> Result<&'a Arc<project_manager::UiTemplateData>, ProjectManagerActorError> {
    let Some(template) = ctx.data().templates.get(view_id) else {
        return Err(ProjectManagerActorError::DataProcessingError(format!(
            "Template '{}' does not exist",
            view_id
        )));
    };

    Ok(template)
}

fn get_view_control<'a>(
    ctx: &'a mut UpdateContext,
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
        },
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
