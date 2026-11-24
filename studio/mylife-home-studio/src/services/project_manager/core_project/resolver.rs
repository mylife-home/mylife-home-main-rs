//! Flattening a project into the one the runtime would see.
//!
//! A template is a design-time grouping: at deploy time its components are
//! instantiated once per component that uses it, with `{{id}}` filled in and
//! the exported config supplied from outside. Nothing downstream of here knows
//! templates exist.
//!
//! Both `validate` and the deploy path start by resolving, which is why this
//! sits beside the model rather than in it: its result is a different shape
//! from a project, and no operation produces one.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use super::model::{
    Binding, BindingId, Collection, Component, ComponentDefinition, ComponentId, Config,
    CoreModelError, ExportId, Id, IdKind, InstanceId, Plugin, PluginId, Project, Template,
    TemplateId,
};

/// How deep template nesting may go before the data is called malformed. The
/// same guard the model's definition lookups use.
const MAX_DEPTH: usize = 32;

/// A project with its templates flattened away: components of plugins, and
/// bindings between them.
#[derive(Debug, Clone)]
pub struct ResolvedProject {
    plugins: Collection<PluginId, Plugin>,
    components: BTreeMap<ComponentId, ResolvedComponent>,
    bindings: BTreeMap<BindingId, ResolvedBinding>,
}

#[derive(Debug, Clone)]
pub struct ResolvedComponent {
    pub plugin: PluginId,
    pub config: Config,
    pub external: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedBinding {
    pub source_component: ComponentId,
    pub source_state: String,
    pub target_component: ComponentId,
    pub target_action: String,
}

impl ResolvedProject {
    pub fn plugins(&self) -> &Collection<PluginId, Plugin> {
        &self.plugins
    }

    pub fn components(&self) -> &BTreeMap<ComponentId, ResolvedComponent> {
        &self.components
    }

    pub fn bindings(&self) -> &BTreeMap<BindingId, ResolvedBinding> {
        &self.bindings
    }

    pub fn component(&self, id: &ComponentId) -> Result<&ResolvedComponent, CoreModelError> {
        self.components
            .get(id)
            .ok_or_else(|| CoreModelError::not_found(IdKind::Component, id))
    }

    /// The plugin a resolved component instantiates.
    pub fn plugin_of(&self, component: &ResolvedComponent) -> Result<&Arc<Plugin>, CoreModelError> {
        self.plugins.get_required(&component.plugin)
    }

    /// External components are left out: they belong to another project.
    pub fn instances(&self) -> Result<BTreeMap<InstanceId, Vec<ComponentId>>, CoreModelError> {
        let mut instances: BTreeMap<InstanceId, Vec<ComponentId>> = BTreeMap::new();

        for (id, component) in &self.components {
            if component.external {
                continue;
            }

            let plugin = self.plugin_of(component)?;
            instances
                .entry(plugin.instance_name.clone())
                .or_default()
                .push(id.clone());
        }

        Ok(instances)
    }
}

/// Flatten `project`.
pub fn resolve(project: &Project) -> Result<ResolvedProject, CoreModelError> {
    let mut resolver = Resolver {
        project,
        components: BTreeMap::new(),
        bindings: BTreeMap::new(),
    };

    let nodes = resolver.build_components(project.components(), "", &HashMap::new(), 0)?;

    resolver.build_bindings(project.bindings(), &nodes)?;

    Ok(ResolvedProject {
        plugins: project.plugins().clone(),
        components: resolver.components,
        bindings: resolver.bindings,
    })
}

/// What one component of a view became. A template became the whole subtree
/// it expanded to, which a binding onto one of its exports has to walk.
enum Node {
    Direct(ComponentId),
    Template {
        id: TemplateId,
        children: HashMap<ComponentId, Node>,
    },
}

struct Resolver<'a> {
    project: &'a Project,
    components: BTreeMap<ComponentId, ResolvedComponent>,
    bindings: BTreeMap<BindingId, ResolvedBinding>,
}

impl Resolver<'_> {
    /// `substitute` fills the `{{id}}` placeholder: empty at the top level,
    /// and the resolved id of the instantiating component below it.
    fn build_components(
        &mut self,
        components: &Collection<ComponentId, Component>,
        substitute: &str,
        config_overrides: &HashMap<ComponentId, HashMap<String, serde_json::Value>>,
        depth: usize,
    ) -> Result<HashMap<ComponentId, Node>, CoreModelError> {
        if depth >= MAX_DEPTH {
            return Err(CoreModelError::CircularTemplate {
                id: String::from("<nested too deep>"),
            });
        }

        let mut nodes = HashMap::new();

        for (id, component) in components {
            let resolved_id = id.substituted(substitute)?;
            let config = merged_config(component, config_overrides.get(id));

            let node = match &component.definition {
                ComponentDefinition::Plugin(plugin) => {
                    self.project.plugins().get_required(plugin)?;

                    self.components.insert(
                        resolved_id.clone(),
                        ResolvedComponent {
                            plugin: plugin.clone(),
                            config,
                            external: component.external,
                        },
                    );

                    Node::Direct(resolved_id)
                }

                ComponentDefinition::Template(template_id) => {
                    let template = self.project.templates().get_required(template_id)?.clone();

                    let children = self.build_components(
                        &template.components,
                        resolved_id.as_str(),
                        &exported_config(&template, &config),
                        depth + 1,
                    )?;

                    Node::Template {
                        id: template_id.clone(),
                        children,
                    }
                }
            };

            nodes.insert(id.clone(), node);
        }

        Ok(nodes)
    }

    /// Resolve one view's bindings against what its components became, then do
    /// the same inside every template those components expanded to.
    fn build_bindings(
        &mut self,
        bindings: &Collection<BindingId, Binding>,
        nodes: &HashMap<ComponentId, Node>,
    ) -> Result<(), CoreModelError> {
        for (_, binding) in bindings {
            let (source_component, source_state) =
                self.member_of(nodes, &binding.source_component, &binding.source_state)?;
            let (target_component, target_action) =
                self.member_of(nodes, &binding.target_component, &binding.target_action)?;

            let resolved = ResolvedBinding {
                source_component,
                source_state,
                target_component,
                target_action,
            };

            self.bindings
                .insert(resolved_binding_id(&resolved)?, resolved);
        }

        for node in nodes.values() {
            if let Node::Template { id, children } = node {
                let template = self.project.templates().get_required(id)?.clone();
                self.build_bindings(&template.bindings, children)?;
            }
        }

        Ok(())
    }

    /// On a template the member is an export, pointing at one of the
    /// template's own components, which may itself be a template.
    fn member_of(
        &self,
        nodes: &HashMap<ComponentId, Node>,
        component: &ComponentId,
        member: &str,
    ) -> Result<(ComponentId, String), CoreModelError> {
        let node = nodes
            .get(component)
            .ok_or_else(|| CoreModelError::not_found(IdKind::Component, component))?;

        match node {
            Node::Direct(id) => Ok((id.clone(), member.to_owned())),

            Node::Template { id, children } => {
                let template = self.project.templates().get_required(id)?.clone();
                let export = template.exports.member_export(&ExportId::new(member)?)?;

                self.member_of(children, &export.component, &export.member)
            }
        }
    }
}

/// A component's own configuration, with the items its template export
/// supplies written over it.
fn merged_config(
    component: &Component,
    overrides: Option<&HashMap<String, serde_json::Value>>,
) -> Config {
    let mut config = component.config.clone().unwrap_or_default();

    if let Some(overrides) = overrides {
        for (name, value) in overrides {
            config = config.with_set(name, value.clone());
        }
    }

    config
}

/// What the component instantiating a template supplies to the components
/// inside it, keyed by the inner component the export points at.
fn exported_config(
    template: &Template,
    config: &Config,
) -> HashMap<ComponentId, HashMap<String, serde_json::Value>> {
    let mut overrides: HashMap<ComponentId, HashMap<String, serde_json::Value>> = HashMap::new();

    for (export_id, export) in &template.exports.config {
        let Some(value) = config.get(export_id.as_str()) else {
            // Nothing supplied it, so the inner component keeps whatever it
            // had. A missing value is reported by validation, not here.
            continue;
        };

        overrides
            .entry(export.component.clone())
            .or_default()
            .insert(export.config_name.clone(), value.clone());
    }

    overrides
}

fn resolved_binding_id(binding: &ResolvedBinding) -> Result<BindingId, CoreModelError> {
    BindingId::build(
        &binding.source_component,
        &binding.source_state,
        &binding.target_component,
        &binding.target_action,
    )
}
