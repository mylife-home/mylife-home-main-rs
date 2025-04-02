use std::{fmt, str::FromStr};

use darling::{FromAttributes, FromDeriveInput, FromField, FromMeta, ToTokens};
use plugin_runtime::metadata;
use proc_macro2::TokenStream;
use quote::{TokenStreamExt, quote};

pub fn option_string_to_tokens(value: &Option<String>) -> TokenStream {
    if let Some(str) = value {
        quote! { Some(#str) }
    } else {
        quote! { None }
    }
}

// c/c from metadata to add FromMeta/ToToken
#[derive(FromMeta, PartialEq, Eq, Debug)]
pub enum PluginUsage {
    Sensor,
    Actuator,
    Logic,
    Ui,
}

impl ToTokens for PluginUsage {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let generated = match *self {
            PluginUsage::Sensor => quote! { plugin_runtime::metadata::PluginUsage::Sensor },
            PluginUsage::Actuator => {
                quote! { plugin_runtime::metadata::PluginUsage::Actuator }
            }
            PluginUsage::Logic => quote! { plugin_runtime::metadata::PluginUsage::Logic },
            PluginUsage::Ui => quote! { plugin_runtime::metadata::PluginUsage::Ui },
        };

        tokens.append_all(generated);
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Type(metadata::Type);

impl Type {
    pub fn value(&self) -> &metadata::Type {
        &self.0
    }

    pub fn new(r#type: metadata::Type) -> Self {
        Type(r#type)
    }
}

impl FromMeta for Type {
    fn from_string(value: &str) -> Result<Self, darling::Error> {
        match metadata::Type::from_str(value) {
            Ok(typ) => Ok(Type(typ)),
            Err(err) => Err(darling::Error::custom(err)),
        }
    }
}

impl ToTokens for Type {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let generated = match self.value() {
            metadata::Type::Range(min, max) => {
                quote! { plugin_runtime::metadata::Type::Range(#min, #max) }
            }
            metadata::Type::Text => quote! { plugin_runtime::metadata::Type::Text },
            metadata::Type::Float => quote! { plugin_runtime::metadata::Type::Float },
            metadata::Type::Bool => quote! { plugin_runtime::metadata::Type::Bool },
            metadata::Type::Enum(vec) => {
                quote! { plugin_runtime::metadata::Type::Enum(vec![#(#vec.to_string()),*]) }
            }
            metadata::Type::Complex => quote! { plugin_runtime::metadata::Type::Complex },
        };

        tokens.append_all(generated);
    }
}

// c/c from metadata to add FromMeta
#[derive(FromMeta, PartialEq, Eq, Debug)]
pub enum ConfigType {
    String,
    Bool,
    Integer,
    Float,
}

impl ToTokens for ConfigType {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let generated = match *self {
            ConfigType::String => quote! { plugin_runtime::metadata::ConfigType::String },
            ConfigType::Bool => quote! { plugin_runtime::metadata::ConfigType::Bool },
            ConfigType::Integer => quote! { plugin_runtime::metadata::ConfigType::Integer },
            ConfigType::Float => quote! { plugin_runtime::metadata::ConfigType::Float },
        };

        tokens.append_all(generated);
    }
}

#[derive(Debug, Clone)]
pub struct ConfigTypeError {
    r#type: syn::Type,
}

impl fmt::Display for ConfigTypeError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "Invalid member type provided '{:#?}'", self.r#type)
    }
}

impl TryFrom<&syn::Type> for ConfigType {
    type Error = ConfigTypeError;

    fn try_from(value: &syn::Type) -> Result<Self, Self::Error> {
        if let syn::Type::Path(type_path) = value {
            match type_path.clone().into_token_stream().to_string().as_str() {
                "String" => {
                    return Ok(ConfigType::String);
                }
                "bool" => {
                    return Ok(ConfigType::Bool);
                }
                "i64" => {
                    return Ok(ConfigType::Integer);
                }
                "f64" => {
                    return Ok(ConfigType::Float);
                }
                _ => {}
            }
        }

        Err(ConfigTypeError {
            r#type: value.clone(),
        })
    }
}

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(mylife_plugin), supports(struct_named))]
pub struct MylifePlugin {
    // pub ident: syn::Ident,
    #[darling(default)]
    pub name: Option<String>,

    #[darling(default)]
    pub description: Option<String>,

    pub usage: PluginUsage,
}

#[derive(Debug, FromField)]
#[darling(attributes(mylife_config))]
pub struct MylifeConfig {
    pub ident: Option<syn::Ident>,
    pub ty: syn::Type,

    #[darling(default)]
    pub name: Option<String>,

    #[darling(default)]
    pub description: Option<String>,

    pub r#type: Option<ConfigType>,
}

#[derive(Debug, FromField)]
#[darling(attributes(mylife_state))]
pub struct MylifeState {
    pub ident: Option<syn::Ident>,
    pub ty: syn::Type,

    #[darling(default)]
    pub name: Option<String>,

    #[darling(default)]
    pub description: Option<String>,

    pub r#type: Option<Type>,
}

#[derive(Debug, FromAttributes)]
#[darling(attributes(mylife_action))]
pub struct MylifeAction {
    #[darling(default)]
    pub name: Option<String>,

    #[darling(default)]
    pub description: Option<String>,

    pub r#type: Option<Type>,
}
