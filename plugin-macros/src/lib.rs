use darling::{FromDeriveInput, FromField};
use proc_macro2::TokenStream;
use quote::{quote, format_ident};

mod attributes {
    use darling::{FromDeriveInput, FromMeta, FromField, FromVariant};

    #[derive(PartialEq, Eq, Debug)]
    struct VecString(Vec<String>);

    /// Parsing literal array into stirng, i.e. `[a,b,b]`.
    impl FromMeta for VecString {
        fn from_value(value: &syn::Lit) -> darling::Result<Self> {
            let expr_array = syn::ExprArray::from_value(value)?;
            // To meet rust <1.36 borrow checker rules on expr_array.elems
            let v =
                expr_array
                    .elems
                    .iter()
                    .map(|expr| match expr {
                        syn::Expr::Lit(lit) => String::from_value(&lit.lit),
                        _ => Err(darling::Error::custom("Expected array of unsigned integers")
                            .with_span(expr)),
                    })
                    .collect::<darling::Result<Vec<String>>>();
            v.and_then(|v| Ok(VecString(v)))
        }
    }

    // c/c from metadata to add FromMeta
    #[derive(FromMeta, PartialEq, Eq, Debug)]
    pub enum PluginUsage {
        Sensor,
        Actuator,
        Logic,
        Ui,
    }

    #[derive(FromMeta, PartialEq, Eq, Debug)]
    pub struct RangeValue {
        min: i64,
        max: i64
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

        pub r#type: Option<String>, // TODO
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

        pub r#type: Option<String>, // TODO
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

        pub r#type: Option<String>, // TODO
    }
}

#[proc_macro_derive(MylifePlugin, attributes(mylife_plugin, mylife_config, mylife_state))]
pub fn derive_mylife_plugin(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input: syn::DeriveInput = syn::parse_macro_input!(input);
    let name = &input.ident;
    let mut streams = Vec::new();

    let attr_plugin = attributes::MylifePlugin::from_derive_input(&input).unwrap();
    streams.push(process_plugin(name, &attr_plugin));

    let fields = 
    if let syn::Data::Struct(data) = &input.data {
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
                    streams.push(process_config(name, &attr_config));
                }

                "mylife_state" => {
                    let attr_state = attributes::MylifeState::from_field(&field).unwrap();
                    streams.push(process_state(name, &attr_state));
                }

                unknown => {
                    println!("Ignored attribute : {}", unknown);
                }
            }
        }
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

                let mut meta_builder = plugin_runtime::macros_backend::PluginMetadataBuilder::new();
                meta_builder.set_name(String::from("tmp"));
                meta_builder.set_usage(plugin_runtime::metadata::PluginUsage::Logic);

                let meta = meta_builder.build().expect("Failed to build meta"); // TODO

                plugin_runtime::macros_backend::MyLifePluginRuntimeImpl::<ComponentImpl>::new(meta)
            }

            fn fail(error: Box<dyn std::error::Error>) {
                unimplemented!();
            }
        }

        pub struct #inventory_name(plugin_runtime::macros_backend::Definition);
        inventory::collect!(#inventory_name);

        #(#streams)*
    };

    gen.into()
}

#[proc_macro_attribute]
pub fn mylife_actions(_attr: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input: syn::ItemImpl = syn::parse_macro_input!(input);

    let name = 
    if let syn::Type::Path(path) = input.self_ty.as_ref() {
        path.path.get_ident().unwrap()
    } else {
        panic!("pan");
    };

    let inventory_name = format_ident!("__MylifeInternalsInventory{}__", name);

    let gen = quote! {
        #input

        inventory::submit!(#inventory_name(plugin_runtime::macros_backend::Definition::new_action("test", Some("test"), plugin_runtime::metadata::Type::Bool)));
    };

    gen.into()
}

fn process_plugin(name: &syn::Ident, attr: &attributes::MylifePlugin) -> TokenStream {
    println!("plugin {} => {:?}", name.to_string(), attr);

    let struct_name = name.to_string();
    let plugin_name = attr.name.as_ref().unwrap_or(&struct_name);

    quote!{
        builder.set_name(#plugin_name)
    }
}

fn process_config(name: &syn::Ident, attr: &attributes::MylifeConfig) -> TokenStream {
    println!("config {} => {:?}", name.to_string(), attr);

    TokenStream::new()
}

fn process_state(name: &syn::Ident, attr: &attributes::MylifeState) -> TokenStream {
    println!("state {} => {:?}", name.to_string(), attr);

    TokenStream::new()
}

fn process_action(name: &syn::Ident, attr: &attributes::MylifeAction) -> TokenStream {
    println!("action {} => {:?}", name.to_string(), attr);

    TokenStream::new()
}
