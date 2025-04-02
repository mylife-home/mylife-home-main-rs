use plugin_runtime::metadata::{ConfigType, MemberType, PluginMetadata, PluginUsage, Type};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestMetadata {
    name: String,
    description: Option<String>,
    usage: PluginUsage,
    config: Vec<TestConfigItem>,
    members: Vec<TestMember>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TestConfigItem {
    name: String,
    description: Option<String>,
    value_type: ConfigType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TestMember {
    name: String,
    description: Option<String>,
    member_type: MemberType,
    value_type: Type,
}

#[allow(dead_code)]
impl TestMetadata {
    pub fn from_metadata(source: &PluginMetadata) -> Self {
        let mut meta = TestMetadata::new(source.name(), source.description(), source.usage());

        for (name, config_item) in source.config() {
            meta.add_config(name, config_item.description(), config_item.value_type());
        }

        for (name, member) in source.members() {
            match member.member_type() {
                MemberType::Action => {
                    meta.add_action(name, member.description(), member.value_type().clone());
                }
                MemberType::State => {
                    meta.add_state(name, member.description(), member.value_type().clone());
                }
            }
        }

        meta
    }

    pub fn new(name: &str, description: Option<&str>, usage: PluginUsage) -> Self {
        TestMetadata {
            name: String::from(name),
            description: description.map(str::to_string),
            usage,
            config: Vec::new(),
            members: Vec::new(),
        }
    }

    pub fn add_config(
        &mut self,
        name: &str,
        description: Option<&str>,
        value_type: ConfigType,
    ) -> &mut Self {
        self.config.push(TestConfigItem {
            name: String::from(name),
            description: description.map(str::to_string),
            value_type: value_type,
        });

        self.config.sort_by_key(|item| item.name.clone());

        self
    }

    pub fn add_state(
        &mut self,
        name: &str,
        description: Option<&str>,
        value_type: Type,
    ) -> &mut Self {
        self.members.push(TestMember {
            name: String::from(name),
            description: description.map(str::to_string),
            member_type: MemberType::State,
            value_type,
        });

        self.members.sort_by_key(|member| member.name.clone());

        self
    }

    pub fn add_action(
        &mut self,
        name: &str,
        description: Option<&str>,
        value_type: Type,
    ) -> &mut Self {
        self.members.push(TestMember {
            name: String::from(name),
            description: description.map(str::to_string),
            member_type: MemberType::Action,
            value_type,
        });

        self.members.sort_by_key(|member| member.name.clone());

        self
    }
}
