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

pub trait OptionResourceOps {
    fn on_change_resource(
        &self,
        id: &str,
        new_id: Option<&str>,
    ) -> Option<Option<project_manager::Resource>>;
}

impl OptionResourceOps for Option<project_manager::Resource> {
    fn on_change_resource(
        &self,
        id: &str,
        new_id: Option<&str>,
    ) -> Option<Option<project_manager::Resource>> {
        if let Some(resource) = self
            && resource.as_str() == id
        {
            Some(new_id.map(project_manager::Resource::from_str))
        } else {
            None
        }
    }
}
