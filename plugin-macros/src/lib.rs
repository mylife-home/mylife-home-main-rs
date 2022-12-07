use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_derive(MylifePlugin, attributes(mylife_plugin, mylife_config, mylife_state))]
pub fn derive_mylife_plugin(input: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();
    let name = &ast.ident;

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

                let meta = meta_builder.build();

                plugin_runtime::macros_backend::MyLifePluginRuntimeImpl::<ComponentImpl>::new(meta)
            }

            fn fail(error: Box<dyn std::error::Error>) {
                unimplemented!();
            }
        }
    };

    gen.into()
}

#[proc_macro_attribute]
pub fn mylife_actions(_attr: TokenStream, input: TokenStream) -> TokenStream {
    // mylife_action
    input
}

// TODO: should it be an attribute like proc_macro_derive
#[proc_macro_attribute]
pub fn mylife_action(_attr: TokenStream, input: TokenStream) -> TokenStream {
    input
}
