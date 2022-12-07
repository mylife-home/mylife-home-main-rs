use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_derive(MylifePlugin, attributes(mylife_plugin, mylife_config, mylife_state))]
pub fn derive_mylife_plugin(input: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();
    let name = &ast.ident;

    let gen = quote! {
        impl plugin_runtime::MylifePlugin for #name {
            fn runtime() -> Box<dyn plugin_runtime::runtime::MyLifePluginRuntime> {

                pub struct Component {

                }

                let meta = plugin_runtime::macros_backend::PluginMetadataBuilder::new().build();

                plugin_runtime::macros_backend::MyLifePluginRuntimeImpl<Component>::new(meta)
            }

            fn fail(error: Box<dyn std::error::Error>) {
                unimplemented!();
            }
        }
    };

    gen.into()
}

#[proc_macro_attribute]
pub fn mylife_action(_attr: TokenStream, input: TokenStream) -> TokenStream {
    // mylife_action
    input
}
