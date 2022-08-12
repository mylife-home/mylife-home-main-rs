use plugin_runtime::metadata::{PluginUsage, ConfigType, Type};
use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};
use darling::{FromDeriveInput, FromField};

#[derive(FromDeriveInput, Default)]
#[darling(attributes(plugin), supports(struct_any))]
struct PluginAttribute {
    ident: syn::Ident,

    name: Option<String>,
    decription: Option<String>,
    usage: PluginUsage,
}

#[derive(FromField, Default)]
#[darling(attributes(config))]
struct ConfigAttribute {
    ident: Option<syn::Ident>,
    ty: syn::Type,

    name: Option<String>,
    decription: Option<String>,
    type: Option<ConfigType>,
}

#[derive(FromField, Default)]
#[darling(attributes(state))]
struct StateAttribute {
    ident: Option<syn::Ident>,
    ty: syn::Type,

    name: Option<String>,
    decription: Option<String>,
    value_type: Option<Type>,
}

#[derive(FromField, Default)]
#[darling(attributes(action))]
struct ActionAttribute {
    ident: Option<syn::Ident>,
    ty: syn::Type,

    name: Option<String>,
    decription: Option<String>,
    value_type: Option<Type>,
}

#[proc_macro_derive(MylifePlugin, attributes(plugin, config, state, action))]
pub fn mylife_plugin(input: TokenStream) -> TokenStream {
    println!("PLUGIN attr: \"{}\"", input.to_string());
    TokenStream::new()
}
