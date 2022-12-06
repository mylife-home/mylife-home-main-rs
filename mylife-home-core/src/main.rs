fn main() {
    println!("Hello, world!");
}


pub struct PluginRuntime {
    plugin: Box<dyn Plugin>,
}

impl PluginRuntime {
    pub fn configure(&mut self, config: ConfigMap) {
        // TODO
    }

    // TODO: state
    // TODO: action

    pub fn init(&mut self) {
        self.plugin.init();
    }

    pub fn terminate(&mut self) {
        self.plugin.terminate();
    }
}

pub struct ConfigMap {}
