use std::marker::PhantomData;

use crate::{
    metadata::PluginMetadata,
    runtime::{MyLifePluginRuntime, MylifeComponent},
};

pub struct MyLifePluginRuntimeImpl<Component: MylifeComponent + Default + 'static> {
    metadata: PluginMetadata,
    _marker: PhantomData<Component>, // only to keep Component type info
}

impl<Component: MylifeComponent + Default> MyLifePluginRuntimeImpl<Component> {
    pub fn new(metadata: PluginMetadata) -> Box<Self> {
        Box::new(MyLifePluginRuntimeImpl {
            metadata,
            _marker: PhantomData,
        })
    }
}

impl<Component: MylifeComponent + Default> MyLifePluginRuntime
    for MyLifePluginRuntimeImpl<Component>
{
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn create(&self) -> Box<dyn MylifeComponent> {
        Box::new(Component::default())
    }
}
