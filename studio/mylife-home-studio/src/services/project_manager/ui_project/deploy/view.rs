use std::collections::HashMap;
use std::sync::Arc;

use studio_web_api::ui_model;

use super::super::model::{
    Collection, Control, ControlId, Template, TemplateInstance, TemplateInstanceId, Window,
    WindowId,
};
use super::control::deploy_control;
use super::validation::{InvalidForDeployment, Validation, ValidationResult};
use super::{deploy_items, style};

/// Where a control lands once its enclosing instances have been accounted for.
///
/// A template's controls are deployed once per instance of it, so their ids
/// have to be made unique and their positions made absolute. `prefix` is the
/// chain of instance ids, joined with ':', which is why ':' is reserved in an
/// authored id.
#[derive(Clone, Default)]
pub struct DeployPosition {
    pub prefix: String,
    pub x: i32,
    pub y: i32,
}

impl DeployPosition {
    pub fn child(&self, id: impl ToString, x: i32, y: i32) -> Self {
        let id = id.to_string();

        Self {
            prefix: if self.prefix.is_empty() {
                id
            } else {
                format!("{}:{}", self.prefix, id)
            },
            x: self.x + x,
            y: self.y + y,
        }
    }
}

pub fn deploy_window(
    window: &Window,
    id: &WindowId,
    validation: &Validation,
) -> ValidationResult<ui_model::Window> {
    let background = validation.validate_optional_resource(&window.background_resource);

    let controls = deploy_controls(
        &window.controls,
        &window.templates,
        validation,
        &DeployPosition::default(),
    );

    let (Ok(background_resource), Ok(controls)) = (background, controls) else {
        return Err(InvalidForDeployment);
    };

    Ok(ui_model::Window {
        id: id.to_string(),
        style: style(&window.style),
        height: window.height,
        width: window.width,
        background_resource,
        controls,
    })
}

/// A template's own contents. The template becomes the owner, so a reference to
/// one of its exports resolves from here on.
pub fn deploy_template(
    template: &Arc<Template>,
    validation: &Validation,
    position: &DeployPosition,
) -> ValidationResult<Vec<ui_model::Control>> {
    deploy_controls(
        &template.controls,
        &template.templates,
        &validation.with_owner_template(Some(template.clone())),
        position,
    )
}

/// One use of a template: check the bindings it supplies, then deploy the
/// template's contents with them in scope.
fn deploy_instance(
    instance: &TemplateInstance,
    validation: &Validation,
    position: &DeployPosition,
) -> ValidationResult<Vec<ui_model::Control>> {
    let validation = validation.enter_template(&instance.template_id)?;

    let Some(template) = validation.project().templates().get(&instance.template_id) else {
        validation.add_error(format!(
            "Le template '{}' n'existe pas.",
            instance.template_id
        ));
        return Err(InvalidForDeployment);
    };

    // A slot keyed by an export the template no longer has. Clearing an export
    // only reaches the instances inside the template that owns it, so this is
    // where an instance living in another view finds out.
    let extra = deploy_items(
        &instance.bindings,
        &validation,
        "template-binding",
        |validation, export_id, _| {
            if template.exports.contains(export_id) {
                return Ok(());
            }

            validation.add_error(format!("Binding sur export '{export_id}' inexistant"));
            Err(InvalidForDeployment)
        },
    );

    let bindings = deploy_items(
        template.exports.iter(),
        &validation,
        "template-binding",
        |validation, export_id, export| {
            let Some(binding) = instance.bindings.get(export_id) else {
                validation.add_error(format!("Binding inexistant sur export '{export_id}'"));
                return Err(InvalidForDeployment);
            };

            validation.validate_member(
                binding,
                export.member_type,
                Some(&export.value_type),
                false,
            )?;

            let Some(resolved) = validation.resolve(binding) else {
                validation.add_error("Le binding ne peut pas être résolu.");
                return Err(InvalidForDeployment);
            };

            Ok((export_id.clone(), resolved))
        },
    );

    let (Ok(_), Ok(bindings)) = (extra, bindings) else {
        return Err(InvalidForDeployment);
    };

    deploy_template(
        template,
        &validation.with_bindings(bindings.into_iter().collect::<HashMap<_, _>>()),
        position,
    )
}

/// The contents of a view, flattened: the controls of every instance in it,
/// then its own controls.
fn deploy_controls(
    controls: &Collection<ControlId, Control>,
    instances: &Collection<TemplateInstanceId, TemplateInstance>,
    validation: &Validation,
    position: &DeployPosition,
) -> ValidationResult<Vec<ui_model::Control>> {
    let from_instances = deploy_items(
        instances.iter(),
        validation,
        "template-instance",
        |validation, id, instance| {
            deploy_instance(
                instance,
                validation,
                &position.child(id, instance.x, instance.y),
            )
        },
    );

    let own = deploy_items(
        controls.iter(),
        validation,
        "control",
        |validation, id, control| {
            let position = position.child(id, control.x, control.y);
            let mut deployed = deploy_control(control, position.prefix, validation)?;

            deployed.x = position.x;
            deployed.y = position.y;
            Ok(deployed)
        },
    );

    let (Ok(from_instances), Ok(own)) = (from_instances, own) else {
        return Err(InvalidForDeployment);
    };

    Ok(from_instances.into_iter().flatten().chain(own).collect())
}
