use std::slice;

use attributes::ConfigType;
use darling::{FromAttributes, FromDeriveInput, FromField, ToTokens};
use proc_macro2::TokenStream;
use proc_macro_error::{abort, abort_call_site, emit_warning, proc_macro_error};
use quote::{format_ident, quote};

mod attributes;
mod helpers;

// TODO: path.get_ident() does not work if `plugin_runtime::Toto`
// TODO: abort_call_site => find real call site

#[proc_macro_derive(MylifePlugin, attributes(mylife_plugin, mylife_config, mylife_state))]
#[proc_macro_error]
pub fn derive_mylife_plugin(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input: syn::DeriveInput = syn::parse_macro_input!(input);
    let name = &input.ident;
    let mut streams = Vec::new();
    let mut errors = darling::Error::accumulator();

    match errors.handle(attributes::MylifePlugin::from_derive_input(&input)) {
        Some(attr_plugin) => {
            streams.push(process_plugin(name, &attr_plugin));
        }
        None => (),
    };

    let fields = if let syn::Data::Struct(data) = &input.data {
        &data.fields
    } else {
        abort_call_site!("Unexpected parsing error (expected struct)");
    };

    for field in fields.iter() {
        for attr in field.attrs.iter() {
            let attr_ident = attr.path.get_ident().unwrap();
            match attr_ident.to_string().as_str() {
                "mylife_config" => {
                    match errors.handle(attributes::MylifeConfig::from_field(&field)) {
                        Some(attr_config) => {
                            streams.push(process_config(name, &attr_config));
                        }
                        None => (),
                    };
                }

                "mylife_state" => {
                    match errors.handle(attributes::MylifeState::from_field(&field)) {
                        Some(attr_state) => {
                            streams.push(process_state(name, &attr_state));
                        }
                        None => (),
                    };
                }

                unknown => {
                    emit_warning!(attr_ident, "Ignored attribute : {}", unknown);
                }
            }
        }
    }

    match errors.finish() {
        Ok(_) => (),
        Err(err) => {
            return err.write_errors().into();
        }
    }

    let inventory_name = format_ident!("__MylifeInternalsInventory{}__", name);

    let gen = quote! {
        impl plugin_runtime::MylifePlugin for #name {
            fn runtime() -> Box<dyn plugin_runtime::runtime::MylifePluginRuntime> {
                let mut builder = plugin_runtime::macros_backend::PluginRuntimeBuilder::new();

                for item in inventory::iter::<#inventory_name> {
                    let callback = item.0;
                    callback(&mut builder);
                }

                builder.build()
            }
        }

        pub struct #inventory_name(plugin_runtime::macros_backend::BuilderPartCallback<#name>);
        inventory::collect!(#inventory_name);

        inventory::submit!(#inventory_name(|builder| {
            #(#streams)*
        }));
    };

    helpers::dump_output(&gen);

    gen.into()
}

#[proc_macro_attribute]
#[proc_macro_error]
pub fn mylife_actions(
    _attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let input: syn::ItemImpl = syn::parse_macro_input!(input);
    let mut errors = darling::Error::accumulator();

    let name = if let syn::Type::Path(path) = input.self_ty.as_ref() {
        path.path.get_ident().unwrap()
    } else {
        abort_call_site!("Unexpected parsing error");
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
                        match errors.handle(attributes::MylifeAction::from_attributes(
                            slice::from_ref(attr),
                        )) {
                            Some(attr_action) => {
                                streams.push(process_action(name, &method.sig, &attr_action));
                            }
                            None => (),
                        };

                        return false;
                    }

                    unknown => {
                        emit_warning!(attr_ident, "Ignored attribute : {}", unknown);
                        return true;
                    }
                }
            });
        }
    }

    match errors.finish() {
        Ok(_) => (),
        Err(err) => {
            return err.write_errors().into();
        }
    }

    let gen = quote! {
        #input

        inventory::submit!(#inventory_name(|builder| {
            #(#streams)*
        }));
    };
    
    helpers::dump_output(&gen);

    gen.into()
}

fn process_plugin(name: &syn::Ident, attr: &attributes::MylifePlugin) -> TokenStream {
    let struct_name = helpers::make_plugin_name(name);
    let name = attr.name.as_ref().unwrap_or(&struct_name);
    let description = attributes::option_string_to_tokens(&attr.description);
    let usage = &attr.usage;

    quote! {
        builder.set_plugin(#name, #description, #usage);
    }
}

fn process_config(plugin_name: &syn::Ident, attr: &attributes::MylifeConfig) -> TokenStream {
    let var_name = helpers::make_member_name(
        attr.ident
            .as_ref()
            .expect("Unexpected unnamed config member"),
    );

    let name = attr.name.as_ref().unwrap_or(&var_name);
    let description = attributes::option_string_to_tokens(&attr.description);
    let r#type = ConfigType::try_from(&attr.ty).unwrap();
    let target_ident = &attr.ident;

    let setter = quote! {
        |target: &mut #plugin_name, arg: plugin_runtime::runtime::ConfigValue| -> std::result::Result<(), Box<dyn std::error::Error>> {
            target.#target_ident = arg.try_into()?;

            std::result::Result::Ok(())
        }
    };

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
            #r#type,
            #setter
        );
    }
}

fn process_state(plugin_name: &syn::Ident, attr: &attributes::MylifeState) -> TokenStream {
    let var_name = helpers::make_member_name(
        attr.ident
            .as_ref()
            .expect("Unexpected unnamed state member"),
    );

    let name = attr.name.as_ref().unwrap_or(&var_name);
    let description = attributes::option_string_to_tokens(&attr.description);
    let var_type = get_state_type(&attr.ty);
    let r#type = helpers::get_type(var_type, &attr.r#type);
    let target_ident = &attr.ident;

    let register = quote! {
        |target: &mut #plugin_name, listener: std::boxed::Box<dyn std::ops::Fn(plugin_runtime::runtime::Value)>| {
            let runtime_type: plugin_runtime::metadata::Type = #r#type;
            target.#target_ident.runtime_register(listener, runtime_type);
        }
    };

    let getter = quote! {
        |target: &#plugin_name| -> plugin_runtime::runtime::Value {
            use plugin_runtime::runtime::TypedInto;

            lazy_static::lazy_static! {
                static ref runtime_type: plugin_runtime::metadata::Type = #r#type;
            }

            let native_value = target.#target_ident.get();
            native_value.clone().typed_into(&runtime_type)
        }
    };

    quote! {
        builder.add_state(
            #name,
            #description,
            #r#type,
            #register,
            #getter
        );
    }
}

fn process_action(
    plugin_name: &syn::Ident,
    sig: &syn::Signature,
    attr: &attributes::MylifeAction,
) -> TokenStream {
    let var_name = helpers::make_member_name(&sig.ident);

    let name = attr.name.as_ref().unwrap_or(&var_name);
    let description = attributes::option_string_to_tokens(&attr.description);
    let var_type = &get_action_type(sig);
    let r#type = helpers::get_type(var_type, &attr.r#type);
    let target_ident = &sig.ident;

    let has_output = match &sig.output {
        syn::ReturnType::Default => false,
        syn::ReturnType::Type(_, _) => true, // Note: if type does not implement Result<(), Error> it will fail to compile (TODO: test)
    };

    let end_ident = if has_output {
        quote! { ? }
    } else {
        quote! {}
    };

    let executor = quote! {
        |target: &mut #plugin_name, arg: plugin_runtime::runtime::Value| -> std::result::Result<(), Box<dyn std::error::Error>> {
            use plugin_runtime::runtime::TypedTryInto;

            lazy_static::lazy_static! {
                static ref runtime_type: plugin_runtime::metadata::Type = #r#type;
            }

            let value: #var_type = arg.clone().typed_try_into(&runtime_type)?;
            target.#target_ident(value)#end_ident;

            std::result::Result::Ok(())
        }
    };

    quote! {
        builder.add_action(
            #name,
            #description,
            #r#type,
            #executor
        );
    }
}

// State<bool> => get bool
fn get_state_type(var_type: &syn::Type) -> &syn::Type {
    if let syn::Type::Path(path) = var_type {
        let seg = path.path.segments.last().unwrap();
        if seg.ident.to_string() != "State" {
            abort!(
                seg.ident.span(),
                "mylife_state variable must be of type State"
            );
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

    abort_call_site!(
        "Wrong value type '{}', expected 'State<type>'",
        var_type.to_token_stream()
    );
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
