//! The rules each kind of id plays by, and what happens when one is broken.

use super::fixture::*;

#[test]
fn an_authored_id_rejects_the_path_separator() {
    // ':' joins the chain of instance ids a deploy generates, so an authored id
    // containing one would make a generated id ambiguous.
    for id in ["a:b", ":", "win:"] {
        assert!(
            matches!(
                WindowId::new(id),
                Err(UiModelError::ForbiddenCharacter {
                    kind: IdKind::Window,
                    character: ':',
                    ..
                })
            ),
            "'{id}' should be refused"
        );
    }
}

#[test]
fn an_authored_id_rejects_being_empty() {
    for id in ["", "  ", "\t"] {
        assert!(matches!(
            ControlId::new(id),
            Err(UiModelError::EmptyId {
                kind: IdKind::Control
            })
        ));
    }
}

#[test]
fn the_error_names_the_kind_of_id_it_was() {
    // The kind is carried rather than formatted at the call site, so every
    // message agrees about what a thing is called.
    assert_eq!(
        TemplateId::new("a:b").unwrap_err().to_string(),
        "template id 'a:b' contains forbidden character ':'"
    );
    assert_eq!(
        ExportId::new("").unwrap_err().to_string(),
        "export id is empty"
    );
}

#[test]
fn a_component_id_keeps_whatever_the_core_side_calls_it() {
    // These are not ours to validate: they arrive with the component data.
    assert!(ComponentId::new("has:colon").is_ok());
    assert!(ComponentId::new("").is_err());
}

#[test]
fn a_plugin_id_is_structured_and_keeps_its_colon() {
    let id = plugin_id();

    assert_eq!(id.as_str(), "instance:module.name");
    assert_eq!(id.parts(), Some(("instance", "module", "name")));
}

#[test]
fn a_plugin_id_that_is_not_structured_has_no_parts() {
    assert_eq!(PluginId::new("nonsense").unwrap().parts(), None);
}

#[test]
fn a_missing_entry_is_reported_against_its_own_kind() {
    let project = project();

    assert_eq!(
        project
            .windows()
            .get_required(&window_id("nope"))
            .unwrap_err()
            .to_string(),
        "window 'nope' not found"
    );
    assert_eq!(
        project
            .templates()
            .get_required(&template_id("nope"))
            .unwrap_err()
            .to_string(),
        "template 'nope' not found"
    );
}
