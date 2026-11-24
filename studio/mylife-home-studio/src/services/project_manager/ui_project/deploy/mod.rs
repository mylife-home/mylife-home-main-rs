//! Turning a project into the definition the runtime UI consumes.
//!
//! Deploy is read-only and shaped unlike anything in `model`: an operation
//! fails with one `UiModelError`, while a deploy has to report every problem it
//! finds, each with the path of the element it was found at, so the designer
//! can list them. That accumulator is `Validation`, and it is the reason this
//! lives beside the model rather than in it.
//!
//! The other reason is `ui_model`, the runtime's own format. The model knows
//! about exactly one foreign shape, the one in `model::wire`; the definition
//! format stops here.
//!
//! Flattening is the work itself: a template's controls are deployed once per
//! instance of it, with ids prefixed by the chain of instance ids and positions
//! made absolute, so the result is a flat list of windows and controls with no
//! templates left in it.

mod control;
mod validation;
mod view;

use std::collections::HashMap;
use std::fmt;

use studio_web_api::{project_manager, ui_model};

use super::model::{Project, StyleList};
use validation::{InvalidForDeployment, Validation, ValidationResult};
use view::DeployPosition;

/// The deployable definition, or every reason it is not deployable.
pub fn deploy(
    project: &Project,
) -> Result<ui_model::Definition, Vec<project_manager::UiValidationError>> {
    let validation = Validation::new(project);

    let default_window = deploy_default_window(project, &validation);

    // Templates are checked on their own as well as through the windows that
    // use them, so a template nothing uses yet still reports its errors. The
    // result is discarded: a template reaches the runtime only inlined into the
    // windows that use it.
    let templates = deploy_items(
        project.templates().iter(),
        &validation,
        "template",
        |validation, id, template| {
            let validation = validation.enter_template(id)?;
            view::deploy_template(template, &validation, &DeployPosition::default())
        },
    );

    let windows = deploy_items(
        project.windows().iter(),
        &validation,
        "window",
        |validation, id, window| view::deploy_window(window, id, validation),
    );

    let (Ok(default_window), Ok(_), Ok(windows)) = (default_window, templates, windows) else {
        return Err(validation.errors());
    };

    Ok(ui_model::Definition {
        resources: project
            .resources()
            .iter()
            .map(|(id, resource)| ui_model::DefinitionResource {
                id: id.to_string(),
                mime: resource.mime.clone(),
                data: resource.data.clone(),
            })
            .collect(),
        styles: project
            .styles()
            .iter()
            .map(|(id, style)| ui_model::DefinitionStyle {
                id: id.to_string(),
                properties: style.properties.clone(),
            })
            .collect(),
        windows,
        default_window,
    })
}

fn deploy_default_window(
    project: &Project,
    validation: &Validation,
) -> ValidationResult<ui_model::DefaultWindow> {
    let mut windows = HashMap::new();
    let mut has_errors = false;

    for (context, window) in project.default_window().iter() {
        let validation = validation.at("default-window", context);

        let Ok(window) = validation.validate_window(window) else {
            has_errors = true;
            continue;
        };

        windows.insert(context.clone(), window);
    }

    if has_errors {
        Err(InvalidForDeployment)
    } else {
        Ok(ui_model::DefaultWindow(windows))
    }
}

/// Deploy every entry of a keyed collection, descending one path node per
/// entry.
///
/// Every entry is visited even after one fails, so a deploy reports all of its
/// problems in one pass instead of one per attempt.
fn deploy_items<'a, I, K, V, T>(
    items: I,
    validation: &Validation,
    item_type: &str,
    mut deploy_item: impl FnMut(&Validation, &K, &V) -> ValidationResult<T>,
) -> ValidationResult<Vec<T>>
where
    I: IntoIterator<Item = (&'a K, &'a V)>,
    K: fmt::Display + 'a,
    V: 'a,
{
    let results: Vec<_> = items
        .into_iter()
        .map(|(id, value)| deploy_item(&validation.at(item_type, id), id, value))
        .collect();

    results.into_iter().collect()
}

/// The same, for the lists that are addressed by index.
fn deploy_vec<V, T>(
    items: &[V],
    validation: &Validation,
    item_type: &str,
    mut deploy_item: impl FnMut(&Validation, &V) -> ValidationResult<T>,
) -> ValidationResult<Vec<T>> {
    let results: Vec<_> = items
        .iter()
        .enumerate()
        .map(|(index, value)| deploy_item(&validation.at(item_type, index), value))
        .collect();

    results.into_iter().collect()
}

fn style(styles: &StyleList) -> ui_model::Style {
    ui_model::Style(styles.as_slice().iter().map(ToString::to_string).collect())
}
