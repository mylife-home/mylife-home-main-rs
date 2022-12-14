use std::fmt;

use darling::{FromDeriveInput, FromField, FromMeta, FromVariant, ToTokens};
use proc_macro2::TokenStream;
use quote::{quote, TokenStreamExt};

#[derive(PartialEq, Eq, Debug)]
pub struct VecString(Vec<String>);

/// Parsing literal array into string, i.e. `[a,b,b]`.
impl FromMeta for VecString {
    fn from_value(value: &syn::Lit) -> darling::Result<Self> {
        let expr_array = syn::ExprArray::from_value(value)?;
        // To meet rust <1.36 borrow checker rules on expr_array.elems
        let v = expr_array
            .elems
            .iter()
            .map(|expr| match expr {
                syn::Expr::Lit(lit) => String::from_value(&lit.lit),
                _ => Err(
                    darling::Error::custom("Expected array of unsigned integers")
                        .with_span(expr),
                ),
            })
            .collect::<darling::Result<Vec<String>>>();
        v.and_then(|v| Ok(VecString(v)))
    }
}

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
        let gen = match *self {
            PluginUsage::Sensor => quote! { plugin_runtime::metadata::PluginUsage::Sensor },
            PluginUsage::Actuator => quote! { plugin_runtime::metadata::PluginUsage::Actuator },
            PluginUsage::Logic => quote! { plugin_runtime::metadata::PluginUsage::Logic },
            PluginUsage::Ui => quote! { plugin_runtime::metadata::PluginUsage::Ui },
        };

        tokens.append_all(gen);
    }
}

#[derive(FromMeta, PartialEq, Eq, Debug)]
pub struct RangeValue {
    min: i64,
    max: i64,
}

// c/c from metadata to add FromMeta
#[derive(FromMeta, PartialEq, Eq, Debug)]
pub enum Type {
    Range(RangeValue),
    Text,
    Float,
    Bool,
    Enum(VecString),
    Complex,
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
        let gen = match *self {
            ConfigType::String => quote! { plugin_runtime::metadata::ConfigType::String },
            ConfigType::Bool => quote! { plugin_runtime::metadata::ConfigType::Bool },
            ConfigType::Integer => quote! { plugin_runtime::metadata::ConfigType::Integer },
            ConfigType::Float => quote! { plugin_runtime::metadata::ConfigType::Float },
        };

        tokens.append_all(gen);
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
    pub ident: syn::Ident,

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

#[derive(Debug, FromVariant)]
#[darling(attributes(mylife_action))]
pub struct MylifeAction {
    pub ident: syn::Ident,
    pub ty: syn::Type,

    #[darling(default)]
    pub name: Option<String>,

    #[darling(default)]
    pub description: Option<String>,

    pub r#type: Option<Type>,
}