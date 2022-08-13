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
    #[darling(default)]
    name: Option<String>,

    #[darling(default)]
    decription: Option<String>,

    #[darling(default, rename = "value")]
    value_type: Option<MConfigType>,
}

#[derive(Debug, FromField)]
#[darling(attributes(state))]
struct StateAttribute {
    #[darling(default)]
    name: Option<String>,

    #[darling(default)]
    decription: Option<String>,

    #[darling(default, rename = "value")]
    value_type: Option<MType>,
}

#[derive(Debug, FromField)]
#[darling(attributes(action))]
struct ActionAttribute {
    #[darling(default)]
    name: Option<String>,

    #[darling(default)]
    decription: Option<String>,

    #[darling(default, rename = "value")]
    value_type: Option<MType>,
}

#[proc_macro_derive(MylifePlugin, attributes(plugin_settings, config, state, action))]
pub fn mylife_plugin(input: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(input);
    let plugin_settings = PluginSettingsAttribute::from_derive_input(&input).expect("Wrong options");
    println!("PLUGIN SETTINGS: \"{:?}\"", plugin_settings);

    if let syn::Data::Struct(s) = input.data {
        match s.fields {
            syn::Fields::Named(fields) => {
                for field in fields.named.iter() {
                    if let syn::Type::Path(syn::TypePath { qself: _, path }) = &field.ty {
                        let field_type = path.segments.last().unwrap();
                        println!("FIELD {} {} {:?}", field.ident.as_ref().unwrap(), field_type.ident, field_type.arguments);
                    }

                    let config = ConfigAttribute::from_field(field).unwrap();
                    let state = StateAttribute::from_field(field).unwrap();
                    let action = ActionAttribute::from_field(field).unwrap();
                    println!("CONFIG: \"{:?}\"", config);
                    println!("STATE: \"{:?}\"", state);
                    println!("ACTION: \"{:?}\"", action);
                }
            },
            syn::Fields::Unnamed(_) => panic!("Unhandled struct field type: Unnamed"),
            syn::Fields::Unit => panic!("Unhandled struct field type: Unit"),
        }
    }

    TokenStream::new()
}
