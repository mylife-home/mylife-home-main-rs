use studio_web_api::project_manager;

pub fn path_node(node_type: &str, id: &str) -> project_manager::UiElementPathNode {
    project_manager::UiElementPathNode {
        r#type: node_type.to_owned(),
        id: id.to_owned(),
    }
}

pub fn with_node(
    mut path: Vec<project_manager::UiElementPathNode>,
    node: project_manager::UiElementPathNode,
) -> Vec<project_manager::UiElementPathNode> {
    path.push(node);
    path
}
