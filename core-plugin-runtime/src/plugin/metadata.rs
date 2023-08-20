use regex::Regex;
use std::{collections::HashMap, fmt, num::ParseIntError, str};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PluginUsage {
    Sensor,
    Actuator,
    Logic,
    Ui,
}

#[derive(Debug)]
pub struct PluginMetadata {
    // id
    name: String,
    // module
    usage: PluginUsage,
    // version
    description: Option<String>,

    members: HashMap<String, Member>,
    config: HashMap<String, ConfigItem>,
}

impl PluginMetadata {
    pub(crate) fn new(
        name: String,
        usage: PluginUsage,
        description: Option<String>,
        members: HashMap<String, Member>,
        config: HashMap<String, ConfigItem>,
    ) -> PluginMetadata {
        PluginMetadata {
            name,
            usage,
            description,
            members,
            config,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn usage(&self) -> PluginUsage {
        self.usage
    }

    pub fn members(&self) -> &HashMap<String, Member> {
        &self.members
    }

    pub fn config(&self) -> &HashMap<String, ConfigItem> {
        &self.config
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MemberType {
    Action,
    State,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Range(i64, i64),
    Text,
    Float,
    Bool,
    Enum(Vec<String>),
    Complex,
}

impl str::FromStr for Type {
    type Err = TypeParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        lazy_static::lazy_static! {
            static ref PARSER: Regex = Regex::new(r"([a-z]+)(.*)").unwrap();
            static ref RANGE_PARSER: Regex = Regex::new(r"\[(-?\d+);(-?\d+)\]").unwrap();
            static ref ENUM_PARSER: Regex = Regex::new(r"\{(.[\w_\-,]+)\}").unwrap();
        }

        let matchs = PARSER
            .captures(input)
            .ok_or_else(|| TypeParseError::new(input, TypeParseErrorReason::InvalidType))?;

        let base_type: &str;
        let args: &str;

        match matchs.len() {
            2 => {
                base_type = &matchs[1];
                args = "";
            }
            3 => {
                base_type = &matchs[1];
                args = &matchs[2];
            }

            _ => {
                return Err(TypeParseError::new(
                    input,
                    TypeParseErrorReason::BadMatchLen,
                ));
            }
        }

        match base_type {
            "range" => {
                let matchs = RANGE_PARSER
                    .captures(args)
                    .ok_or_else(|| TypeParseError::new(input, TypeParseErrorReason::BadArgs))?;
                if matchs.len() != 3 {
                    return Err(TypeParseError::new(input, TypeParseErrorReason::BadArgs));
                }

                let min = matchs[1].parse::<i64>().map_err(|err| {
                    TypeParseError::new(input, TypeParseErrorReason::BadValue(err))
                })?;
                let max = matchs[2].parse::<i64>().map_err(|err| {
                    TypeParseError::new(input, TypeParseErrorReason::BadValue(err))
                })?;

                if min >= max {
                    return Err(TypeParseError::new(input, TypeParseErrorReason::MinMax));
                }

                return Ok(Type::Range(min, max));
            }
            "text" => {
                if args != "" {
                    return Err(TypeParseError::new(
                        input,
                        TypeParseErrorReason::UnexpectedArgs,
                    ));
                }
                return Ok(Type::Text);
            }
            "float" => {
                if args != "" {
                    return Err(TypeParseError::new(
                        input,
                        TypeParseErrorReason::UnexpectedArgs,
                    ));
                }
                return Ok(Type::Float);
            }
            "bool" => {
                if args != "" {
                    return Err(TypeParseError::new(
                        input,
                        TypeParseErrorReason::UnexpectedArgs,
                    ));
                }
                return Ok(Type::Bool);
            }
            "enum" => {
                let matchs = ENUM_PARSER
                    .captures(args)
                    .ok_or_else(|| TypeParseError::new(input, TypeParseErrorReason::BadArgs))?;
                if matchs.len() != 2 {
                    return Err(TypeParseError::new(input, TypeParseErrorReason::BadArgs));
                }

                let values: Vec<String> = matchs[1].split(",").map(String::from).collect();
                if values.len() < 2 {
                    return Err(TypeParseError::new(input, TypeParseErrorReason::BadArgs));
                }

                return Ok(Type::Enum(values));
            }
            "complex" => {
                if args != "" {
                    return Err(TypeParseError::new(
                        input,
                        TypeParseErrorReason::UnexpectedArgs,
                    ));
                }
                return Ok(Type::Complex);
            }
            _ => {
                return Err(TypeParseError::new(
                    input,
                    TypeParseErrorReason::UnknownType,
                ));
            }
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Range(min, max) => write!(f, "range[{};{}]", min, max),
            Type::Text => write!(f, "text"),
            Type::Float => write!(f, "float"),
            Type::Bool => write!(f, "bool"),
            Type::Enum(list) => write!(f, "enum{{{}}}", list.join(",")),
            Type::Complex => write!(f, "complex"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypeParseError {
    pub input: String,
    pub reason: TypeParseErrorReason,
}

#[derive(Debug, Clone)]
pub enum TypeParseErrorReason {
    InvalidType,
    BadMatchLen,
    BadArgs,
    UnexpectedArgs,
    UnknownType,
    BadValue(ParseIntError),
    MinMax,
}

impl TypeParseError {
    pub fn new(input: &str, reason: TypeParseErrorReason) -> Self {
        TypeParseError {
            input: input.into(),
            reason,
        }
    }
}

impl fmt::Display for TypeParseError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match &self.reason {
            TypeParseErrorReason::InvalidType => {
                write!(fmt, "Invalid type '{}'", self.input)
            }
            TypeParseErrorReason::BadMatchLen => {
                write!(fmt, "Invalid type '{}' (bad match len)", self.input)
            }
            TypeParseErrorReason::BadArgs => {
                write!(fmt, "Invalid type '{}' (bad args)", self.input)
            }
            TypeParseErrorReason::UnexpectedArgs => {
                write!(fmt, "Invalid type '{}' (unexpected args)", self.input)
            }
            TypeParseErrorReason::BadValue(source) => {
                write!(fmt, "Invalid type '{}' ({})", self.input, source)
            }
            TypeParseErrorReason::MinMax => {
                write!(fmt, "Invalid type '{}' (min >= max)", self.input)
            }
            TypeParseErrorReason::UnknownType => {
                write!(fmt, "Invalid type '{}' (unknown type)", self.input)
            }
        }
    }
}

impl std::error::Error for TypeParseError {}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    fn test_parse_type(str: &str) {
        let typ = Type::from_str(str).unwrap();
        assert_eq!(typ.to_string(), str);
    }

    #[test]
    fn test_parse_range() {
        test_parse_type("range[-12;42]");
    }

    #[test]
    fn test_parse_text() {
        test_parse_type("text");
    }

    #[test]
    fn test_parse_float() {
        test_parse_type("float");
    }

    #[test]
    fn test_parse_bool() {
        test_parse_type("bool");
    }

    #[test]
    fn test_parse_enum() {
        test_parse_type("enum{one,two,three}");
    }

    #[test]
    fn test_parse_complex() {
        test_parse_type("complex");
    }
}

#[derive(Debug, Clone)]
pub struct Member {
    description: Option<String>,
    member_type: MemberType,
    value_type: Type,
}

impl Member {
    pub(crate) fn new(
        description: Option<String>,
        member_type: MemberType,
        value_type: Type,
    ) -> Member {
        Member {
            description,
            member_type,
            value_type,
        }
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn member_type(&self) -> MemberType {
        self.member_type
    }

    pub fn value_type(&self) -> &Type {
        &self.value_type
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ConfigType {
    String,
    Bool,
    Integer,
    Float,
}

#[derive(Debug, Clone)]
pub struct ConfigItem {
    description: Option<String>,
    value_type: ConfigType,
}

impl ConfigItem {
    pub(crate) fn new(description: Option<String>, value_type: ConfigType) -> ConfigItem {
        ConfigItem {
            description,
            value_type,
        }
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn value_type(&self) -> ConfigType {
        self.value_type
    }
}
