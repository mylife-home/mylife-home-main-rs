use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_derive(MylifePlugin, attributes(mylife_plugin, mylife_config, mylife_state))]
pub fn derive_mylife_plugin(input: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();
    let name = &ast.ident;

    let gen = quote! {
        impl plugin_runtime::MylifePlugin for #name {
            fn runtime() -> Box<dyn plugin_runtime::MyLifePluginRuntime>{
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
