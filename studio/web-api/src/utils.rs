use serde::{Deserialize, Serialize};

/// A utility enum representing a field that can be missing, null, or have a value.
/// 
/// This is useful for representing optional fields in partial updates, where a field may be intentionally omitted, explicitly set to null, or have a concrete value.
/// 
/// This won't be translated to TypeScript directly, as the representing version of this would be Partial<T>.
/// 
/// Example type:
/// 
/// ```rust
/// #[derive(Debug, Clone, Serialize, Deserialize, TS)]
/// #[ts(export, export_to = "file.ts")]
/// #[serde(rename_all = "camelCase")]
/// struct MyObject {
///     pub field: String,
/// }
/// 
/// #[derive(Debug, Clone, Serialize, Deserialize, TS)]
/// #[ts(export_to = "file.ts")]
/// #[ts(type = "Partial<MyObject>")]
/// struct MyPartialObject {
///     #[serde(default, skip_serializing_if = "PartialField::is_missing")]
///     pub field: PartialField<String>,
/// }
/// ```
#[derive(Debug, Default, Clone)]
pub enum PartialField<T> {
    #[default]
    Missing,
    Value(T),
}

// no Serialize for now
impl<T> PartialField<T> {
    pub fn is_missing(&self) -> bool {
        matches!(self, PartialField::Missing)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for PartialField<T> {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        T::deserialize(d).map(PartialField::Value)
    }
}

impl<T: Serialize> Serialize for PartialField<T> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            PartialField::Value(v) => v.serialize(s),
            PartialField::Missing => Err(serde::ser::Error::custom(
                "PartialField::Missing must be skipped with skip_serializing_if",
            )),
        }
    }
}
