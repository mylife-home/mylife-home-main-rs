use attributes::ConfigType;
use darling::{FromDeriveInput, FromField};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

mod attributes;

// TODO: add tests on attributes/whole input

#[proc_macro_derive(MylifePlugin, attributes(mylife_plugin, mylife_config, mylife_state))]
pub fn derive_mylife_plugin(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input: syn::DeriveInput = syn::parse_macro_input!(input);
    let name = &input.ident;
    let mut streams = Vec::new();

    let attr_plugin = attributes::MylifePlugin::from_derive_input(&input).unwrap();
    streams.push(process_plugin(name, &attr_plugin));

    let fields = if let syn::Data::Struct(data) = &input.data {
        &data.fields
    } else {
        panic!("pan");
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
pub fn mylife_actions(
    _attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let input: syn::ItemImpl = syn::parse_macro_input!(input);

    let name = if let syn::Type::Path(path) = input.self_ty.as_ref() {
        path.path.get_ident().unwrap()
    } else {
        panic!("pan");
    };

    let inventory_name = format_ident!("__MylifeInternalsInventory{}__", name);
    let mut streams = Vec::new();

    if false {
        streams.push(TokenStream::new());
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
            panic!(
                "Wrong type provided for config '{}': should be '{:#?}'",
                name, r#type
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

    TokenStream::new()
}

fn process_action(name: &syn::Ident, attr: &attributes::MylifeAction) -> TokenStream {
    // println!("action {} => {:?}", name.to_string(), attr);

    TokenStream::new()
}

fn make_plugin_name(name: &syn::Ident) -> String {
    use convert_case::{Case, Casing};
    name.to_string().to_case(Case::Kebab)
}

fn make_member_name(name: &syn::Ident) -> String {
    use convert_case::{Case, Casing};
    name.to_string().to_case(Case::Camel)
}
