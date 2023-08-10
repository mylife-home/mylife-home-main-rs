use crate::attributes::{RangeValue, Type};
use proc_macro_error::abort_call_site;

pub fn get_type(native_type: &syn::Type, provided_type: &Option<Type>) -> Type {
    let native_type_name = get_native_type_name(native_type);

    if let Some(provided_type) = provided_type {
        match provided_type {
            Type::Range(RangeValue { min, max }) => {
                if native_type_name != "i64" {
                    abort_call_site!("Expected i64, got '{}'", native_type_name);
                }

                if min >= max {
                    abort_call_site!("Expected min ({}) < max ({})", min, max);
                }
            }
            Type::Text => {
                if native_type_name != "String" {
                    abort_call_site!("Expected String, got '{}'", native_type_name);
                }
            }
            Type::Float => {
                if native_type_name != "f64" {
                    abort_call_site!("Expected Float64, got '{}'", native_type_name);
                }
            }
            Type::Bool => {
                if native_type_name != "bool" {
                    abort_call_site!("Expected Bool, got '{}'", native_type_name);
                }
            }
            Type::Enum(path_list) => {
                if native_type_name != "String" {
                    abort_call_site!("Expected String, got '{}'", native_type_name);
                }

                if path_list.len() < 2 {
                    abort_call_site!("Expected at least 2 values in enum, got '{:?}'", path_list.as_slice());
                }

                println!("{:?}", path_list.as_slice());
            }
            Type::Complex => abort_call_site!("Complex value not supported for now"),
        }

        return provided_type.clone();
    } else {
        return match native_type_name.as_str() {
            "f64" => Type::Float,
            "bool" => Type::Bool,
            "String" => Type::Text, // If only String default to Text (drop Enum)
            unsupported => {
                abort_call_site!("Unable to deduce type with native type '{}'", unsupported)
            }
        };
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

pub fn make_plugin_name(name: &syn::Ident) -> String {
    use convert_case::{Case, Casing};
    name.to_string().to_case(Case::Kebab)
}

pub fn make_member_name(name: &syn::Ident) -> String {
    use convert_case::{Case, Casing};
    name.to_string().to_case(Case::Camel)
}
