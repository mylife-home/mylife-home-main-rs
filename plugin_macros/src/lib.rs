use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};
use darling::{FromDeriveInput, FromField, FromMeta};


#[derive(Debug, FromMeta)]
enum MPluginUsage {
  Sensor,
  Actuator,
  Logic,
  Ui,
}


#[derive(Debug)]
enum MType {
  Range(i64, i64),
  Text,
  Float,
  Bool,
  Enum(Vec<String>),
  Complex,
}

impl FromMeta for MType {

}

#[derive(Debug, FromMeta)]
enum MConfigType {
  String,
  Bool,
  Integer,
  Float,
}


#[derive(Debug, FromDeriveInput)]
#[darling(attributes(plugin_settings), supports(struct_any))]
struct PluginSettingsAttribute {
    ident: syn::Ident,

    #[darling(default)]
    name: Option<String>,

    #[darling(default)]
    decription: Option<String>,

    usage: MPluginUsage,
}

#[derive(Debug, FromField)]
#[darling(attributes(config))]
struct ConfigAttribute {
    ident: Option<syn::Ident>,
    ty: syn::Type,

    #[darling(default)]
    name: Option<String>,

    #[darling(default)]
    decription: Option<String>,

    #[darling(default)]
    value_type: Option<MConfigType>, // TODO: rename type
}

#[derive(Debug, FromField)]
#[darling(attributes(state))]
struct StateAttribute {
    ident: Option<syn::Ident>,
    ty: syn::Type,

    #[darling(default)]
    name: Option<String>,

    #[darling(default)]
    decription: Option<String>,

    #[darling(default)]
    value_type: Option<MType>, // TODO: rename type
}

#[derive(Debug, FromField)]
#[darling(attributes(action))]
struct ActionAttribute {
    ident: Option<syn::Ident>,
    ty: syn::Type,

    #[darling(default)]
    name: Option<String>,

    #[darling(default)]
    decription: Option<String>,

    #[darling(default)]
    value_type: Option<MType>, // TODO: rename type
}

#[proc_macro_derive(MylifePlugin, attributes(plugin_settings, config, state, action))]
pub fn mylife_plugin(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input);
    let plugin_settings = PluginSettingsAttribute::from_derive_input(&input).expect("Wrong options");
    println!("PLUGIN SETTINGS: \"{:?}\"", plugin_settings);

    println!("PLUGIN attr: \"{:?}\"", input);
    TokenStream::new()
}
