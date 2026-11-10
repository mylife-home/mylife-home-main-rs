use studio_web_api::project_manager;

pub trait StyleOps {
    fn normalize(&mut self);
}

impl StyleOps for project_manager::Style {
    fn normalize(&mut self) {
        self.0.sort();
    }
}
