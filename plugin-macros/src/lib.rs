use std::slice;

use attributes::{ConfigType, Type, RangeValue, VecString};
use darling::{FromDeriveInput, FromField, FromAttributes};
use proc_macro2::TokenStream;
use proc_macro_error::{abort, abort_call_site, proc_macro_error};
use quote::{format_ident, quote};

mod attributes;

// TODO: add tests on attributes/whole input
// TODO: path.get_ident() does not work if `plugin_runtime::Toto`

#[proc_macro_derive(MylifePlugin, attributes(mylife_plugin, mylife_config, mylife_state))]
#[proc_macro_error]
pub fn derive_mylife_plugin(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input: syn::DeriveInput = syn::parse_macro_input!(input);
    let name = &input.ident;
    let mut streams = Vec::new();

    let attr_plugin = attributes::MylifePlugin::from_derive_input(&input).unwrap();
    streams.push(process_plugin(name, &attr_plugin));

    let fields = if let syn::Data::Struct(data) = &input.data {
        &data.fields
    } else {
        abort_call_site!("pan");
    };

    for field in fields.iter() {
        for attr in field.attrs.iter() {
            let attr_ident = attr.path.get_ident().unwrap();
            match attr_ident.to_string().as_str() {
                "mylife_config" => {
                    let attr_config = attributes::MylifeConfig::from_field(&field).unwrap();
                    streams.push(process_config(&attr_config));
                }

                "mylife_state" => {
                    let attr_state = attributes::MylifeState::from_field(&field).unwrap();
                    streams.push(process_state(&attr_state));
                }

                unknown => {
                    println!("Ignored attribute : {}", unknown);
                }
            }
        }
    }

    for stream in streams.iter() {
        println!("{}", stream);
    }

    let inventory_name = format_ident!("__MylifeInternalsInventory{}__", name);

    let gen = quote! {
        impl plugin_runtime::MylifePlugin for #name {
            fn runtime() -> Box<dyn plugin_runtime::runtime::MyLifePluginRuntime> {

                pub struct ComponentImpl {

                }

                impl plugin_runtime::runtime::MylifeComponent for ComponentImpl {
                    fn set_on_fail(&mut self, handler: fn(error: Box<dyn std::error::Error>)) {

                    }

                    fn set_on_state(&mut self, handler: fn(state: &plugin_runtime::runtime::Value)) {

                    }

                    fn configure(&mut self, config: &plugin_runtime::runtime::Config) {

                    }

                    fn execute_action(&mut self, action: &plugin_runtime::runtime::Value) {

                    }
                }

                impl Default for ComponentImpl {
                    fn default() -> Self {
                        ComponentImpl{}
                    }
                }

                let mut builder = plugin_runtime::macros_backend::PluginRuntimeBuilder::new();

                for item in inventory::iter::<#inventory_name> {
                    let callback = item.0;
                    callback(&mut builder);
                }

                let meta = builder.build().expect("Failed to build meta"); // TODO

                plugin_runtime::macros_backend::MyLifePluginRuntimeImpl::<ComponentImpl>::new(meta)
            }

            fn fail(error: Box<dyn std::error::Error>) {
                unimplemented!();
            }
        }

        pub struct #inventory_name(plugin_runtime::macros_backend::BuilderPartCallback);
        inventory::collect!(#inventory_name);

        inventory::submit!(#inventory_name(|builder| {
            #(#streams)*
        }));

    };

    gen.into()
}

#[proc_macro_attribute]
#[proc_macro_error]
pub fn mylife_actions(
    _attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let input: syn::ItemImpl = syn::parse_macro_input!(input);

    let name = if let syn::Type::Path(path) = input.self_ty.as_ref() {
        path.path.get_ident().unwrap()
    } else {
        abort_call_site!("pan");
    };

    let inventory_name = format_ident!("__MylifeInternalsInventory{}__", name);
    let mut streams = Vec::new();

    if false {
        streams.push(TokenStream::new());
    }

    let mut input = input.clone();

    for item in input.items.iter_mut() {
        if let syn::ImplItem::Method(method) = item {

            method.attrs.retain(|attr| {
                let attr_ident = attr.path.get_ident().unwrap();
                match attr_ident.to_string().as_str() {
                    "mylife_action" => {
                        let attr_action = attributes::MylifeAction::from_attributes(slice::from_ref(attr)).unwrap();
                        streams.push(process_action(&method.sig, &attr_action));
                        return false;
                    }
    
                    unknown => {
                        println!("Ignored attribute : {}", unknown);
                        return true;
                    }
                }
            });
        }
    }

    let gen = quote! {
        #input

        inventory::submit!(#inventory_name(|builder| {
            #(#streams)*
        }));
    };

    gen.into()
}

fn process_plugin(name: &syn::Ident, attr: &attributes::MylifePlugin) -> TokenStream {
    let struct_name = make_plugin_name(name);
    let name = attr.name.as_ref().unwrap_or(&struct_name);
    let description = attributes::option_string_to_tokens(&attr.description);
    let usage = &attr.usage;

    quote! {
        builder.set_plugin(#name, #description, #usage);
    }
}

fn process_config(attr: &attributes::MylifeConfig) -> TokenStream {
    let var_name = make_member_name(
        attr.ident
            .as_ref()
            .expect("Unexpected unnamed config member"),
    );

    let name = attr.name.as_ref().unwrap_or(&var_name);
    let description = attributes::option_string_to_tokens(&attr.description);
    let r#type = ConfigType::try_from(&attr.ty).unwrap();

    if let Some(provided_type) = &attr.r#type {
        if r#type != *provided_type {
            abort_call_site!(
                "Wrong type provided for config '{}': should be '{:#?}'",
                name,
                r#type
            );
        }
    }

    quote! {
        builder.add_config(
            #name,
            #description,
            #r#type
        );
    }
}

fn process_state(attr: &attributes::MylifeState) -> TokenStream {
    // println!("state {} => {:?}", name.to_string(), attr);

    let var_name = make_member_name(
        attr.ident
            .as_ref()
            .expect("Unexpected unnamed state member"),
    );

    let name = attr.name.as_ref().unwrap_or(&var_name);
    let description = attributes::option_string_to_tokens(&attr.description);
    let var_type = get_state_type(&attr.ty);
    let r#type = get_type(var_type, &attr.r#type);

    quote! {
        builder.add_state(
            #name,
            #description,
            #r#type
        );
    }
}

// State<bool> => get bool
fn get_state_type(var_type: &syn::Type) -> &syn::Type {
    if let syn::Type::Path(path) = var_type {
        let seg = path.path.segments.last().unwrap();
        if seg.ident.to_string() != "State" {
            abort!(seg.ident.span(), "mylife_state variable must be of type State");
        }

        if let syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
            args,
            colon2_token: _,
            lt_token: _,
            gt_token: _,
        }) = &seg.arguments
        {
            if let syn::GenericArgument::Type(arg_type) = args.first().unwrap() {
                return arg_type;
            }
        }
    }

    todo!();
}

fn get_type(native_type: &syn::Type, provided_type: &Option<Type>) -> Type {
    let native_type_name = get_native_type_name(native_type);

    if let Some(provided_type) = provided_type {
        match provided_type {
            Type::Range(RangeValue {min, max}) => abort_call_site!("TODO"),
            Type::Text => {
                if native_type_name != "String" {
                    abort_call_site!("Expected String, got '{}'", native_type_name);
                }
            },
            Type::Float => {
                if native_type_name != "f64" {
                    abort_call_site!("Expected Float64, got '{}'", native_type_name);
                }
            },
            Type::Bool =>  {
                if native_type_name != "bool" {
                    abort_call_site!("Expected Bool, got '{}'", native_type_name);
                }
            },
            Type::Enum(VecString(vec)) => abort_call_site!("TODO"),
            Type::Complex => abort_call_site!("TODO"),
        }

        return provided_type.clone();
    } else {
        return match native_type_name.as_str() {
            "f64" => Type::Float,
            "bool" => Type::Bool,
            unsupported => abort_call_site!("Unable to deduce type with native type '{}'", unsupported)
        }
    }
}

fn get_native_type_name(native_type: &syn::Type) -> String {
    if let syn::Type::Path(path) = native_type {
        if let Some(ident) = path.path.get_ident() {
            return ident.to_string();
        }
    }

    abort_call_site!("Invalid type '{:?}'", native_type);
}

fn process_action(sig: &syn::Signature, attr: &attributes::MylifeAction) -> TokenStream {
    let var_name = make_member_name(&sig.ident);

    let name = attr.name.as_ref().unwrap_or(&var_name);
    let description = attributes::option_string_to_tokens(&attr.description);
    let var_type = &get_action_type(sig);
    let r#type = get_type(var_type, &attr.r#type);
    let has_output = true; // TODO: sig.output

    quote! {
        builder.add_action(
            #name,
            #description,
            #r#type
        );
    }
}

// fn toto(&mut self, arg: bool) => get bool
fn get_action_type(sig: &syn::Signature) -> syn::Type {
    if sig.inputs.len() != 2 {
        abort!(sig.ident.span(), "Invalid method args");
    }

    if let syn::FnArg::Receiver(_) = &sig.inputs[0] {
    } else {
        abort!(sig.ident.span(), "Invalid method args");
    }

    if let syn::FnArg::Typed(syn::PatType { ty, .. }) = &sig.inputs[1] {
        ty.as_ref().clone()
    } else {
        abort!(sig.ident.span(), "Invalid method args");
    }
}

fn make_plugin_name(name: &syn::Ident) -> String {
    use convert_case::{Case, Casing};
    name.to_string().to_case(Case::Kebab)
}

fn make_member_name(name: &syn::Ident) -> String {
    use convert_case::{Case, Casing};
    name.to_string().to_case(Case::Camel)
}
