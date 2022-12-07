use std::collections::HashMap;

use crate::metadata::{
    ConfigItem, ConfigType, Member, MemberType, PluginMetadata, PluginUsage, Type,
};

pub struct PluginMetadataBuilder {
    name: Option<String>,
    usage: Option<PluginUsage>,
    description: Option<String>,
    members: HashMap<String, Member>,
    config: HashMap<String, ConfigItem>,
}

impl PluginMetadataBuilder {
    pub fn new() -> Self {
        PluginMetadataBuilder {
            name: None,
            usage: None,
            description: None,
            members: HashMap::new(),
            config: HashMap::new(),
        }
    }

    pub fn build(self) -> PluginMetadata {
        // Note: we must ensure that name/usage are set
        PluginMetadata::new(
            self.name.expect("Name not set in metadata"),
            self.usage.expect("Usage not set in metadata"),
            self.description,
            self.members,
            self.config,
        )
    }

    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    pub fn set_usage(&mut self, usage: PluginUsage) {
        self.usage = Some(usage);
    }

    pub fn set_description(&mut self, description: String) {
        self.description = Some(description);
    }

    pub fn add_member(
        &mut self,
        name: String,
        description: Option<String>,
        member_type: MemberType,
        value_type: Type,
    ) {
        let member = Member::new(description, member_type, value_type);
        self.members.insert(name, member);
    }

    pub fn add_config(
        &mut self,
        name: String,
        description: Option<String>,
        value_type: ConfigType,
    ) {
        let config_item = ConfigItem::new(description, value_type);
        self.config.insert(name, config_item);
    }
}
