use super::ids::ComponentId;
use super::path::ElementPath;
use super::types::MemberRef;

/// One place a component member is referenced from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentUsage {
    pub component_id: ComponentId,
    pub member_name: String,
    pub path: ElementPath,
}

impl ComponentUsage {
    /// Whether a reference is this usage. An unbound reference, or one pointing
    /// at an enclosing template's export, never is.
    pub fn matches(&self, member: &Option<MemberRef>) -> bool {
        matches!(
            member,
            Some(MemberRef::Component { id, member })
                if *id == self.component_id && *member == self.member_name
        )
    }
}

/// Accumulates usages while walking the project.
///
/// The `Rc<RefCell<..>>` of the current collector is not needed here: the walk
/// is a plain `&mut` recursion.
#[derive(Debug, Default)]
pub struct UsageCollector {
    usage: Vec<ComponentUsage>,
}

impl UsageCollector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, component_id: &ComponentId, member_name: &str, path: &ElementPath) {
        self.usage.push(ComponentUsage {
            component_id: component_id.clone(),
            member_name: member_name.to_owned(),
            path: path.clone(),
        });
    }

    pub fn into_usage(self) -> Vec<ComponentUsage> {
        self.usage
    }

    pub fn usage(&self) -> &[ComponentUsage] {
        &self.usage
    }
}
