#[derive(Debug)]
pub struct TemplateModel {
    id: String,
}

impl TemplateModel {
    pub fn id(&self) -> &String {
        &self.id
    }
}
