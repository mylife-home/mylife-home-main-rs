//! The rules each kind of id plays by, and what happens when one is broken.

use super::fixture::*;

#[test]
fn an_authored_id_rejects_the_path_separator() {
    // ':' separates the parts of a plugin id and of a binding id, and the
    // chain of component ids a deploy generates.
    for id in ["a:b", ":", "comp:"] {
        assert!(
            matches!(
                ComponentId::new(id),
                Err(CoreModelError::ForbiddenCharacter {
                    kind: IdKind::Component,
                    character: ':',
                    ..
                })
            ),
            "'{id}' should be refused"
        );
    }

    assert!(TemplateId::new("a:b").is_err());
    assert!(ExportId::new("a:b").is_err());
}

#[test]
fn an_authored_id_rejects_being_empty() {
    for id in ["", "  ", "\t"] {
        assert!(matches!(
            TemplateId::new(id),
            Err(CoreModelError::EmptyId {
                kind: IdKind::Template
            })
        ));
    }
}

#[test]
fn the_error_names_the_kind_of_id_it_was() {
    assert_eq!(
        ComponentId::new("a:b").unwrap_err().to_string(),
        "component id 'a:b' contains forbidden character ':'"
    );
    assert_eq!(
        ExportId::new("").unwrap_err().to_string(),
        "export id is empty"
    );
}

#[test]
fn a_derived_id_is_allowed_the_separator_it_is_built_from() {
    // Plugin, instance and binding ids are not authored here, so the only
    // thing left to assert about them is that they exist.
    assert!(PluginId::new("inst:module.name").is_ok());
    assert!(BindingId::new("a:state:b:action").is_ok());
    assert!(InstanceId::new("some-host").is_ok());

    assert!(PluginId::new("").is_err());
    assert!(BindingId::new("  ").is_err());
}

#[test]
fn a_plugin_id_is_structured_and_keeps_its_colon() {
    let id = plugin_id();

    assert_eq!(id.as_str(), "inst:module.name");
    assert_eq!(id.parts(), Some(("inst", "module", "name")));
    assert_eq!(id.instance().unwrap(), instance_id("inst"));
}

#[test]
fn a_plugin_id_that_is_not_structured_has_no_parts() {
    let id = PluginId::new("nonsense").unwrap();

    assert_eq!(id.parts(), None);
    assert!(matches!(
        id.instance(),
        Err(CoreModelError::MalformedPluginId { .. })
    ));
}

#[test]
fn a_binding_id_is_its_four_parts() {
    // A binding has no id of its own, so the id and the data cannot disagree:
    // one is derived from the other.
    let binding = binding("source", "state", "target", "action");

    assert_eq!(binding.id().unwrap().as_str(), "source:state:target:action");
}

#[test]
fn a_missing_entry_is_reported_against_its_own_kind() {
    let project = project();

    assert_eq!(
        project
            .templates()
            .get_required(&template_id("nope"))
            .unwrap_err()
            .to_string(),
        "template 'nope' not found"
    );
    assert_eq!(
        project
            .plugins()
            .get_required(&PluginId::new("nope").unwrap())
            .unwrap_err()
            .to_string(),
        "plugin 'nope' not found"
    );
}
