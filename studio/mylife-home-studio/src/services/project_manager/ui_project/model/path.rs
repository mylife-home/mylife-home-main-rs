use super::error::UiModelError;
use super::ids::{ControlId, ExportId, TemplateId, TemplateInstanceId, WindowId};
use super::view::ViewKey;

/// One step of an element path.
///
/// On the wire this is `{ type: String, id: String }`. As an enum, a match on
/// a node is exhaustive, so the string is parsed once, here, and no call site
/// needs a runtime error arm for an unexpected type.
///
/// The variants below the first four carry no id of their own: they are the
/// fixed strings `display`, `text`, `primary-action` and friends. They exist so
/// a path points at the exact reference it came from, which is what the refresh
/// dialog shows the user.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PathNode {
    Window(WindowId),
    Template(TemplateId),
    Control(ControlId),
    TemplateInstance(TemplateInstanceId),
    Display,
    Text,
    ContextItem(usize),
    PrimaryAction,
    SecondaryAction,
    Binding(ExportId),
}

impl PathNode {
    /// The `type` half on the wire.
    pub const fn kind(&self) -> &'static str {
        match self {
            PathNode::Window(_) => "window",
            PathNode::Template(_) => "template",
            PathNode::Control(_) => "control",
            PathNode::TemplateInstance(_) => "template-instance",
            PathNode::Display | PathNode::Text => "control-type",
            PathNode::ContextItem(_) => "context-item",
            PathNode::PrimaryAction | PathNode::SecondaryAction => "action",
            PathNode::Binding(_) => "template-binding",
        }
    }

    /// The `id` half on the wire.
    pub fn id(&self) -> String {
        match self {
            PathNode::Window(id) => id.to_string(),
            PathNode::Template(id) => id.to_string(),
            PathNode::Control(id) => id.to_string(),
            PathNode::TemplateInstance(id) => id.to_string(),
            PathNode::Display => String::from("display"),
            PathNode::Text => String::from("text"),
            PathNode::ContextItem(index) => index.to_string(),
            PathNode::PrimaryAction => String::from("primary-action"),
            PathNode::SecondaryAction => String::from("secondary-action"),
            PathNode::Binding(id) => id.to_string(),
        }
    }

    pub fn parse(kind: &str, id: &str) -> Result<Self, UiModelError> {
        let unknown = || UiModelError::UnknownPathNode {
            kind: kind.to_owned(),
            id: id.to_owned(),
        };

        match kind {
            "window" => Ok(PathNode::Window(WindowId::new(id)?)),
            "template" => Ok(PathNode::Template(TemplateId::new(id)?)),
            "control" => Ok(PathNode::Control(ControlId::new(id)?)),
            "template-instance" => Ok(PathNode::TemplateInstance(TemplateInstanceId::new(id)?)),
            "control-type" => match id {
                "display" => Ok(PathNode::Display),
                "text" => Ok(PathNode::Text),
                _ => Err(unknown()),
            },
            "context-item" => id.parse().map(PathNode::ContextItem).map_err(|_| unknown()),
            "action" => match id {
                "primary-action" => Ok(PathNode::PrimaryAction),
                "secondary-action" => Ok(PathNode::SecondaryAction),
                _ => Err(unknown()),
            },
            "template-binding" => Ok(PathNode::Binding(ExportId::new(id)?)),
            _ => Err(unknown()),
        }
    }
}

/// Where an element lives: view, then element inside it, then deeper for the
/// exact reference.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ElementPath(Vec<PathNode>);

impl ElementPath {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn child(&self, node: PathNode) -> Self {
        let mut nodes = self.0.clone();
        nodes.push(node);
        Self(nodes)
    }

    pub fn nodes(&self) -> &[PathNode] {
        &self.0
    }

    /// The view this path starts in.
    ///
    /// The only remaining failure is a malformed path, which cannot be produced
    /// by the collector. The "unknown view type" case the string version has to
    /// handle is gone: a `PathNode` is one of ten things, and two of them are
    /// views.
    pub fn view_key(&self) -> Result<ViewKey, UiModelError> {
        match self.0.first() {
            Some(PathNode::Window(id)) => Ok(ViewKey::Window(id.clone())),
            Some(PathNode::Template(id)) => Ok(ViewKey::Template(id.clone())),
            Some(node) => Err(UiModelError::PathRootIsNotAView { node: node.kind() }),
            None => Err(UiModelError::EmptyPath),
        }
    }

    /// The element inside the view: a control or a template instance.
    pub fn element(&self) -> Result<&PathNode, UiModelError> {
        self.0.get(1).ok_or(UiModelError::EmptyPath)
    }
}

impl FromIterator<PathNode> for ElementPath {
    fn from_iter<T: IntoIterator<Item = PathNode>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}
