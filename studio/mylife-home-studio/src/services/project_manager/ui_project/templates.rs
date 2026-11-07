use std::{collections::HashMap, sync::Arc};

use studio_web_api::{project_manager, utils::PartialField};

use crate::services::project_manager::{
    ProjectManagerActorError,
    ui_project::utils::{UpdateContext, check_id},
};

/// Note: ctx.data() will not be modified by this function
pub fn initial_notifications(ctx: &mut UpdateContext) {
    for (id, template) in ctx.data().templates.clone().iter() {
        ctx.push_notification(project_manager::UpdateProjectNotification::SetUiTemplate(
            project_manager::SetUiTemplateNotification {
                id: id.clone(),
                template: template.clone(),
            },
        ));
    }
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

pub fn update_template(
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

pub fn get_template<'a>(
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

pub fn prepare_template_bindings(
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

pub fn get_template_export_value_type(
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
