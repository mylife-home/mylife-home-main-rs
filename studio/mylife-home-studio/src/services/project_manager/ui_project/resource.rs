use studio_web_api::project_manager;

pub trait ResourceOps {
    fn as_str(&self) -> &str;

    fn from_str(value: impl Into<String>) -> project_manager::Resource;
}

impl ResourceOps for project_manager::Resource {
    fn as_str(&self) -> &str {
        &self.0
    }

    fn from_str(value: impl Into<String>) -> Self {
        project_manager::Resource(value.into())
    }
}
