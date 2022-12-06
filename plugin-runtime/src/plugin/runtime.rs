use crate::metadata;

pub trait MyLifePluginRuntime {
    fn metadata(&self) -> &metadata::PluginMetadata;
    fn create(&self) -> Box<dyn MylifeComponent>;
}

pub trait MylifeComponent {
    fn set_on_fail(&mut self, handler: fn(error: Box<dyn std::error::Error>));
    fn set_on_state(&mut self, handler: fn(state: &State));
    fn configure(&mut self, config: &Config);
    fn execute_action(&mut self, action: &Action);
}

pub struct State {

}

pub struct Action {

}

pub struct Config {

}