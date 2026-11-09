use std::str::FromStr;

use common::components::metadata::Type;
use studio_web_api::project_manager;

use crate::services::project_manager::ui_project::{
    component::component_find_member_value_type,
    path::{path_node, with_node},
};

pub fn validate(data: &project_manager::UiProject) -> Vec<project_manager::UiValidationError> {
    let mut validator = Validator {
        data,
        errors: Vec::new(),
    };

    for (context, window_id) in &data.default_window.0 {
        validator.check_window_id(
            window_id.as_deref(),
            vec![path_node("defaultWindow", context)],
        );
    }

    for (id, window) in data.windows.iter() {
        validator.validate_controls(&window.controls, "window", id, None);

        for (instance_id, instance) in window.templates.iter() {
            validator.validate_template_instance(instance, "window", id, instance_id, None);
        }

        if let Some(resource) = &window.background_resource {
            validator.check_resource_id(Some(&resource.0), vec![path_node("window", id)], true);
        }
    }

    for (id, template) in data.templates.iter() {
        validator.validate_controls(&template.controls, "template", id, Some(template));

        for (instance_id, instance) in template.templates.iter() {
            validator.validate_template_instance(
                instance,
                "template",
                id,
                instance_id,
                Some(template),
            );
        }
    }

    validator.errors
}

struct Validator<'a> {
    data: &'a project_manager::UiProject,
    errors: Vec<project_manager::UiValidationError>,
}

impl<'a> Validator<'a> {
    fn add_error(
        &mut self,
        message: impl Into<String>,
        path: Vec<project_manager::UiElementPathNode>,
    ) {
        self.errors.push(project_manager::UiValidationError {
            path,
            message: message.into(),
        });
    }

    fn check_id(&mut self, id: &str, path: Vec<project_manager::UiElementPathNode>) {
        if id.is_empty() {
            self.add_error("L'identifiant n'est pas défini.", path);
        }
    }

    fn check_window_id(&mut self, id: Option<&str>, path: Vec<project_manager::UiElementPathNode>) {
        match id {
            None => self.add_error("La fenêtre n'est pas définie.", path),
            Some(id) if !self.data.windows.contains_key(id) => {
                self.add_error(format!("La fenêtre '{id}' n'existe pas."), path)
            }
            Some(_) => {}
        }
    }

    fn check_resource_id(
        &mut self,
        id: Option<&str>,
        path: Vec<project_manager::UiElementPathNode>,
        optional: bool,
    ) {
        match id {
            None if !optional => self.add_error("La ressource n'est pas définie.", path),
            Some(id) if !self.data.resources.contains_key(id) => {
                self.add_error(format!("La ressource '{id}' n'existe pas."), path)
            }
            _ => {}
        }
    }

    fn validate_controls(
        &mut self,
        controls: &std::collections::HashMap<
            String,
            std::sync::Arc<project_manager::UiControlData>,
        >,
        view_type: &str,
        view_id: &str,
        owner_template: Option<&project_manager::UiTemplateData>,
    ) {
        for (control_id, control) in controls {
            let control_path = vec![
                path_node(view_type, view_id),
                path_node("control", control_id),
            ];
            self.check_id(
                control_id,
                vec![
                    path_node(view_type, view_id),
                    path_node("control", control_id),
                ],
            );

            match (
                control.display.as_ref().as_ref(),
                control.text.as_ref().as_ref(),
            ) {
                (Some(_), Some(_)) | (None, None) => {
                    self.add_error("Le contrôle doit être image ou texte", control_path.clone())
                }
                (Some(display), None) => {
                    if let Some(resource) = &display.default_resource {
                        self.check_resource_id(Some(&resource.0), control_path.clone(), true);
                    }

                    for (index, item) in display.map.iter().enumerate() {
                        self.check_resource_id(
                            item.resource.as_ref().map(|resource| resource.0.as_str()),
                            with_node(
                                control_path.clone(),
                                path_node("map-item", &index.to_string()),
                            ),
                            false,
                        );
                    }

                    let value_type = self.check_component(
                        display.component_id.as_deref(),
                        display.component_state.as_deref(),
                        owner_template,
                        project_manager::MemberType::State,
                        None,
                        control_path.clone(),
                        display.map.is_empty(),
                    );

                    if let Some(value_type) = value_type {
                        self.validate_display_map(&display.map, &value_type, control_path);
                    }
                }
                (None, Some(text)) => {
                    for (index, item) in text.context.iter().enumerate() {
                        self.check_id(
                            &item.id,
                            with_node(
                                control_path.clone(),
                                path_node("context-item", &index.to_string()),
                            ),
                        );
                        self.check_component(
                            item.component_id.as_deref(),
                            item.component_state.as_deref(),
                            owner_template,
                            project_manager::MemberType::State,
                            None,
                            with_node(control_path.clone(), path_node("context-item", &item.id)),
                            false,
                        );
                    }
                }
            }

            self.validate_action(
                control.primary_action.as_ref().as_ref(),
                "primaryAction",
                view_type,
                view_id,
                control_id,
                owner_template,
            );
            self.validate_action(
                control.secondary_action.as_ref().as_ref(),
                "secondaryAction",
                view_type,
                view_id,
                control_id,
                owner_template,
            );
        }
    }

    fn validate_display_map(
        &mut self,
        map: &[project_manager::UiControlDisplayMapItemData],
        value_type: &str,
        control_path: Vec<project_manager::UiElementPathNode>,
    ) {
        let value_type = match Type::from_str(value_type) {
            Ok(value_type) => value_type,
            Err(_) => {
                self.add_error(
                    format!("Le type '{value_type}' est incorrect."),
                    control_path,
                );
                return;
            }
        };

        if value_type == Type::Complex {
            self.add_error(
                "Un composant de type complexe ne peut pas être utilisé",
                control_path,
            );
            return;
        }

        for (index, item) in map.iter().enumerate() {
            let path = with_node(
                control_path.clone(),
                path_node("map-item", &index.to_string()),
            );

            match &value_type {
                Type::Text => {
                    if item.min.is_some() || item.max.is_some() {
                        self.add_error("Le type 'text' ne doit pas utiliser min/max", path.clone());
                    }
                    if !item.value.is_string() {
                        self.add_error("Valeur incorrecte", path);
                    }
                }
                Type::Enum(values) => {
                    if item.min.is_some() || item.max.is_some() {
                        self.add_error("Le type 'enum' ne doit pas utiliser min/max", path.clone());
                    }
                    if !item
                        .value
                        .as_str()
                        .is_some_and(|value| values.iter().any(|entry| entry == value))
                    {
                        self.add_error("Valeur incorrecte", path);
                    }
                }
                Type::Bool => {
                    if item.min.is_some() || item.max.is_some() {
                        self.add_error("Le type 'enum' ne doit pas utiliser min/max", path.clone());
                    }
                    if !item.value.is_boolean() {
                        self.add_error("Valeur incorrecte", path);
                    }
                }
                Type::Float => {
                    if !item.value.is_null() {
                        self.add_error(
                            "Le type 'float' ne doit pas utiliser valeur mais min/max",
                            path.clone(),
                        );
                    }
                    if item
                        .min
                        .is_some_and(|min| item.max.is_some_and(|max| min > max))
                    {
                        self.add_error("'min' > 'max'", path);
                    }
                }
                Type::Range(range) => {
                    if !item.value.is_null() {
                        self.add_error(
                            "Le type 'range' ne doit pas utiliser valeur mais min/max",
                            path.clone(),
                        );
                    }
                    if item.min.is_none_or(|min| min < *range.start()) {
                        self.add_error("'min' incorrect", path.clone());
                    }
                    if item.max.is_none_or(|max| max > *range.end()) {
                        self.add_error("'max' incorrect", path.clone());
                    }
                    if item.min.zip(item.max).is_some_and(|(min, max)| min > max) {
                        self.add_error("'min' > 'max'", path);
                    }
                }
                Type::Complex => unreachable!("complex values are rejected above"),
            }
        }
    }

    fn validate_action(
        &mut self,
        action: Option<&project_manager::UiActionData>,
        action_type: &str,
        view_type: &str,
        view_id: &str,
        control_id: &str,
        owner_template: Option<&project_manager::UiTemplateData>,
    ) {
        let Some(action) = action else {
            return;
        };

        let path = vec![
            path_node(view_type, view_id),
            path_node("control", control_id),
            path_node("action", action_type),
        ];

        if action.component.is_some() == action.window.is_some() {
            self.add_error("L'action doit être composant ou fenêtre", path.clone());
            return;
        }

        if let Some(component) = &action.component {
            self.check_component(
                component.id.as_deref(),
                component.action.as_deref(),
                owner_template,
                project_manager::MemberType::Action,
                Some("bool"),
                path.clone(),
                false,
            );
        }

        if let Some(window) = &action.window {
            self.check_window_id(window.id.as_deref(), path);
        }
    }

    fn validate_template_instance(
        &mut self,
        instance: &project_manager::UiTemplateInstanceData,
        view_type: &str,
        view_id: &str,
        instance_id: &str,
        owner_template: Option<&project_manager::UiTemplateData>,
    ) {
        let path = vec![
            path_node(view_type, view_id),
            path_node("template-instance", instance_id),
        ];
        self.check_id(instance_id, path.clone());

        let Some(template) = self.data.templates.get(&instance.template_id).cloned() else {
            self.add_error(
                format!("Le template '{}' n'existe pas.", instance.template_id),
                path,
            );
            return;
        };

        for binding_id in instance.bindings.keys() {
            if !template.exports.contains_key(binding_id) {
                self.add_error(
                    format!("Binding sur export '{binding_id}' inexistant"),
                    path.clone(),
                );
            }
        }

        for (binding_id, export) in template.exports.iter() {
            let Some(binding) = instance.bindings.get(binding_id) else {
                self.add_error(
                    format!("Binding inexistant sur export '{binding_id}'"),
                    path.clone(),
                );
                continue;
            };

            self.check_component(
                binding.component_id.as_deref(),
                binding.member_name.as_deref(),
                owner_template,
                export.member_type,
                Some(&export.value_type),
                with_node(path.clone(), path_node("template-binding", binding_id)),
                false,
            );
        }
    }

    fn check_component(
        &mut self,
        component_id: Option<&str>,
        member_name: Option<&str>,
        owner_template: Option<&project_manager::UiTemplateData>,
        member_type: project_manager::MemberType,
        expected_value_type: Option<&str>,
        path: Vec<project_manager::UiElementPathNode>,
        optional: bool,
    ) -> Option<String> {
        if (component_id.is_none() && owner_template.is_none())
            || member_name.is_none_or(str::is_empty)
        {
            if !optional {
                self.add_error("Le composant n'est pas défini.", path);
            }
            return None;
        }

        let member_name = member_name?;
        let value_type = if let Some(component_id) = component_id {
            if !self.data.components.contains_key(component_id) {
                self.add_error(
                    format!("Le composant '{component_id}' n'existe pas."),
                    path.clone(),
                );
                return None;
            }

            component_find_member_value_type(&self.data, component_id, member_name, member_type)
        } else {
            owner_template.and_then(|template| {
                template.exports.get(member_name).and_then(|export| {
                    (export.member_type == member_type).then(|| export.value_type.clone())
                })
            })
        };

        let Some(value_type) = value_type else {
            let member = match member_type {
                project_manager::MemberType::State => "L'état",
                project_manager::MemberType::Action => "L'action",
            };
            let suffix = component_id
                .map(|id| format!(" du composant '{id}'"))
                .unwrap_or_else(|| " de l'export template".to_owned());
            self.add_error(
                format!("{member} '{member_name}'{suffix} n'existe pas."),
                path,
            );
            return None;
        };

        if let Some(expected) = expected_value_type
            && expected != value_type
        {
            let member = match member_type {
                project_manager::MemberType::State => "L'état",
                project_manager::MemberType::Action => "L'action",
            };
            let suffix = component_id
                .map(|id| format!(" du composant '{id}'"))
                .unwrap_or_else(|| " de l'export template".to_owned());
            self.add_error(
                format!("{member} '{member_name}'{suffix} a un type incorrect : '{expected}'."),
                path,
            );
        }

        Some(value_type)
    }
}
