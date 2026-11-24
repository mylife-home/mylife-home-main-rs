use std::collections::HashMap;

use super::Id;
use super::error::CoreModelError;
use super::ids::{ComponentId, TemplateId};
use super::project::Project;
use super::types::ComponentDefinition;
use super::view::ViewKey;

/// Simulates the component ids a deploy would generate, so an operation can be
/// refused before it is applied rather than producing a project that cannot be
/// deployed.
///
/// A template's components are instantiated once per component that uses the
/// template, and each level substitutes `{{id}}`. Two different chains can land
/// on the same final id, and that is what this looks for.
pub struct NamingDryRun {
    views: HashMap<ViewKey, Vec<Entry>>,
}

#[derive(Debug, Clone)]
struct Entry {
    id: ComponentId,
    /// `Some` when the component instantiates a template, which is what makes
    /// the walk recursive.
    template: Option<TemplateId>,
}

impl NamingDryRun {
    /// Snapshot the project as it stands. Pending operations are then layered
    /// on with `set_component` / `rename_component` before `validate`.
    pub fn new(project: &Project) -> Self {
        let mut views = HashMap::new();

        for key in project.view_keys() {
            let view = match project.view(&key) {
                Ok(view) => view,
                // A view that cannot be read cannot contribute ids.
                Err(_) => continue,
            };

            let entries = view
                .components
                .iter()
                .map(|(id, component)| Entry {
                    id: id.clone(),
                    template: match &component.definition {
                        ComponentDefinition::Template(id) => Some(id.clone()),
                        ComponentDefinition::Plugin(_) => None,
                    },
                })
                .collect();

            views.insert(key, entries);
        }

        Self { views }
    }

    pub fn set_component(
        &mut self,
        view: &ViewKey,
        id: ComponentId,
        definition: &ComponentDefinition,
    ) {
        let template = match definition {
            ComponentDefinition::Template(id) => Some(id.clone()),
            ComponentDefinition::Plugin(_) => None,
        };

        self.views
            .entry(view.clone())
            .or_default()
            .push(Entry { id, template });
    }

    pub fn rename_component(&mut self, view: &ViewKey, id: &ComponentId, new_id: ComponentId) {
        let Some(entries) = self.views.get_mut(view) else {
            return;
        };

        for entry in entries {
            if &entry.id == id {
                entry.id = new_id;
                return;
            }
        }
    }

    pub fn validate(&self) -> Result<(), CoreModelError> {
        let mut generated: HashMap<String, Vec<String>> = HashMap::new();

        let roots = self
            .views
            .get(&ViewKey::Project)
            .cloned()
            .unwrap_or_default();

        for entry in &roots {
            self.walk(entry, &mut Vec::new(), &mut Vec::new(), &mut generated)?;
        }

        let mut duplicates: Vec<(&String, &Vec<String>)> = generated
            .iter()
            .filter(|(_, owners)| owners.len() > 1)
            .collect();

        if duplicates.is_empty() {
            return Ok(());
        }

        // Sorted so the message is the same every time it is produced.
        duplicates.sort_by_key(|(id, _)| *id);

        let mut report = String::new();

        for (id, owners) in duplicates {
            report.push_str(&format!("- id duplicate: {id}\n"));

            let mut owners = owners.clone();
            owners.sort();

            for owner in owners {
                report.push_str(&format!("  - {owner}\n"));
            }
        }

        Err(CoreModelError::DuplicateGeneratedIds {
            report: report.trim_end().to_owned(),
        })
    }

    fn walk(
        &self,
        entry: &Entry,
        chain: &mut Vec<ComponentId>,
        templates: &mut Vec<TemplateId>,
        generated: &mut HashMap<String, Vec<String>>,
    ) -> Result<(), CoreModelError> {
        chain.push(entry.id.clone());

        let result = (|| {
            let Some(template_id) = &entry.template else {
                let id = fold_id(chain);
                generated.entry(id).or_default().push(describe(chain));
                return Ok(());
            };

            // A project with a cycle in it cannot be flattened; stop rather
            // than recurse forever. `set_component` refuses to create one, so
            // this is reachable only from stored data.
            if templates.contains(template_id) {
                return Err(CoreModelError::CircularTemplate {
                    id: template_id.to_string(),
                });
            }

            templates.push(template_id.clone());

            let children = self
                .views
                .get(&ViewKey::Template(template_id.clone()))
                .cloned()
                .unwrap_or_default();

            for child in &children {
                self.walk(child, chain, templates, generated)?;
            }

            templates.pop();
            Ok(())
        })();

        chain.pop();
        result
    }
}

/// The id a chain of components flattens to: each level substitutes the id
/// built so far into its own `{{id}}` placeholder.
fn fold_id(chain: &[ComponentId]) -> String {
    let mut id = String::new();

    for item in chain {
        id = item.as_str().replace(ComponentId::PLACEHOLDER, &id);
    }

    id
}

fn describe(chain: &[ComponentId]) -> String {
    chain
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(" -> ")
}
