use studio_web_api::project_manager;

pub trait StyleOps {
    fn normalize(&mut self);

    fn on_change(&self, style_id: &str, new_id: Option<&str>) -> Option<project_manager::Style>;
}

impl StyleOps for project_manager::Style {
    fn normalize(&mut self) {
        self.0.sort();
    }

    fn on_change(&self, style_id: &str, new_id: Option<&str>) -> Option<project_manager::Style> {
        let Some(index) = self.0.iter().position(|item| item.as_str() == style_id) else {
            return None;
        };

        let mut list = self.0.clone();

        if let Some(new_id) = new_id {
            list[index] = String::from(new_id);
        } else {
            list.remove(index);
        }

        Some(project_manager::Style(list))
    }
}
