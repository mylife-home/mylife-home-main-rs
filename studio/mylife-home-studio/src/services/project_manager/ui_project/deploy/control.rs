use common::components::metadata::Type;
use studio_web_api::component_model::MemberType;
use studio_web_api::ui_model;

use super::super::model::{Action, Control, ControlDisplay, ControlDisplayMapItem, ControlText};
use super::validation::{InvalidForDeployment, Validation, ValidationResult};
use super::{deploy_vec, style};

pub fn deploy_control(
    control: &Control,
    id: String,
    validation: &Validation,
) -> ValidationResult<ui_model::Control> {
    let kind = if control.display.is_some() == control.text.is_some() {
        validation.add_error("Le contrôle doit être image ou texte");
        Err(InvalidForDeployment)
    } else {
        Ok(())
    };

    let display = control
        .display
        .as_ref()
        .map(|display| deploy_display(display, &validation.at("control-type", "display")))
        .transpose();
    let text = control
        .text
        .as_ref()
        .map(|text| deploy_text(text, &validation.at("control-type", "text")))
        .transpose();
    let primary_action = control
        .primary_action
        .as_ref()
        .map(|action| deploy_action(action, &validation.at("action", "primary-action")))
        .transpose();
    let secondary_action = control
        .secondary_action
        .as_ref()
        .map(|action| deploy_action(action, &validation.at("action", "secondary-action")))
        .transpose();

    let (Ok(_), Ok(display), Ok(text), Ok(primary_action), Ok(secondary_action)) =
        (kind, display, text, primary_action, secondary_action)
    else {
        return Err(InvalidForDeployment);
    };

    Ok(ui_model::Control {
        id,
        style: style(&control.style),
        height: control.height,
        width: control.width,
        x: control.x,
        y: control.y,
        display,
        text,
        primary_action,
        secondary_action,
    })
}

fn deploy_display(
    display: &ControlDisplay,
    validation: &Validation,
) -> ValidationResult<ui_model::ControlDisplay> {
    let default_resource = validation.validate_optional_resource(&display.default_resource);

    // A display with a map has to know what it is mapping, so the reference is
    // only optional when there is no map.
    let member_value_type = validation.validate_member(
        &display.member,
        MemberType::State,
        None,
        display.map.is_empty(),
    );

    let map = deploy_vec(&display.map, validation, "map-item", |validation, item| {
        let resource = validation.validate_resource(&item.resource);

        let item_props = if let Ok(Some(value_type)) = &member_value_type {
            validate_map_item(validation, item, value_type)
        } else {
            Ok(())
        };

        let (Ok(_), Ok(resource)) = (item_props, resource) else {
            return Err(InvalidForDeployment);
        };

        Ok(ui_model::ControlDisplayMapItem {
            min: item.min,
            max: item.max,
            value: item.value.clone(),
            resource: Some(resource),
        })
    });

    let (component_id, component_state) = match validation.resolve(&display.member) {
        Some((component_id, component_state)) => (Some(component_id), Some(component_state)),
        None => (None, None),
    };

    let (Ok(default_resource), Ok(_), Ok(map)) = (default_resource, member_value_type, map) else {
        return Err(InvalidForDeployment);
    };

    Ok(ui_model::ControlDisplay {
        component_id,
        component_state,
        default_resource,
        map,
    })
}

/// What a map item is allowed to say depends on the type of the value it maps.
fn validate_map_item(
    validation: &Validation,
    item: &ControlDisplayMapItem,
    value_type: &Type,
) -> ValidationResult<()> {
    let mut has_errors = false;

    match value_type {
        Type::Text => {
            if item.min.is_some() || item.max.is_some() {
                validation.add_error("Le type 'text' ne doit pas utiliser min/max");
                has_errors = true;
            }
            if !item.value.is_string() {
                validation.add_error("Valeur incorrecte");
                has_errors = true;
            }
        }

        Type::Enum(values) => {
            if item.min.is_some() || item.max.is_some() {
                validation.add_error("Le type 'enum' ne doit pas utiliser min/max");
                has_errors = true;
            }
            if !item
                .value
                .as_str()
                .is_some_and(|value| values.iter().any(|entry| entry == value))
            {
                validation.add_error("Valeur incorrecte");
                has_errors = true;
            }
        }

        Type::Bool => {
            if item.min.is_some() || item.max.is_some() {
                validation.add_error("Le type 'enum' ne doit pas utiliser min/max");
                has_errors = true;
            }
            if !item.value.is_boolean() {
                validation.add_error("Valeur incorrecte");
                has_errors = true;
            }
        }

        Type::Float => {
            if !item.value.is_null() {
                validation.add_error("Le type 'float' ne doit pas utiliser valeur mais min/max");
                has_errors = true;
            }
            if item.min.zip(item.max).is_some_and(|(min, max)| min > max) {
                validation.add_error("'min' > 'max'");
                has_errors = true;
            }
        }

        Type::Range(range) => {
            if !item.value.is_null() {
                validation.add_error("Le type 'range' ne doit pas utiliser valeur mais min/max");
                has_errors = true;
            }
            if item.min.is_none_or(|min| min < *range.start()) {
                validation.add_error("'min' incorrect");
                has_errors = true;
            }
            if item.max.is_none_or(|max| max > *range.end()) {
                validation.add_error("'max' incorrect");
                has_errors = true;
            }
            if item.min.zip(item.max).is_some_and(|(min, max)| min > max) {
                validation.add_error("'min' > 'max'");
                has_errors = true;
            }
        }

        Type::Complex => {
            validation.add_error("Un composant de type complexe ne peut pas être utilisé");
            has_errors = true;
        }
    }

    if has_errors {
        Err(InvalidForDeployment)
    } else {
        Ok(())
    }
}

fn deploy_text(
    text: &ControlText,
    validation: &Validation,
) -> ValidationResult<ui_model::ControlText> {
    let context = deploy_vec(
        &text.context,
        validation,
        "context-item",
        |validation, item| {
            let id = validation.validate_free_id(&item.id);

            let validation = validation.at("context-item", &item.id);
            let member = validation.validate_member(&item.member, MemberType::State, None, false);

            let (Ok(id), Ok(_), Some((component_id, component_state))) =
                (id, member, validation.resolve(&item.member))
            else {
                return Err(InvalidForDeployment);
            };

            Ok(ui_model::ControlTextContextItem {
                id,
                component_id,
                component_state,
            })
        },
    );

    let Ok(context) = context else {
        return Err(InvalidForDeployment);
    };

    Ok(ui_model::ControlText {
        context,
        format: text.format.clone(),
    })
}

fn deploy_action(action: &Action, validation: &Validation) -> ValidationResult<ui_model::Action> {
    let kind = if action.component.is_some() == action.window.is_some() {
        validation.add_error("L'action doit être composant ou fenêtre");
        Err(InvalidForDeployment)
    } else {
        Ok(())
    };

    let component = action
        .component
        .as_ref()
        .map(|component| {
            let validation = validation.at("action-type", "component");

            // An action is triggered, so the member it points at has to take a
            // bool.
            validation.validate_member(
                &component.member,
                MemberType::Action,
                Some("bool"),
                false,
            )?;

            let Some((id, action)) = validation.resolve(&component.member) else {
                return Err(InvalidForDeployment);
            };

            Ok(ui_model::ActionComponent { id, action })
        })
        .transpose();

    let window: ValidationResult<_> = action
        .window
        .as_ref()
        .map(|window| {
            let validation = validation.at("action-type", "window");

            Ok(ui_model::ActionWindow {
                id: validation.validate_window(&window.id)?,
                popup: window.popup,
            })
        })
        .transpose();

    let (Ok(_), Ok(component), Ok(window)) = (kind, component, window) else {
        return Err(InvalidForDeployment);
    };

    Ok(ui_model::Action { component, window })
}
