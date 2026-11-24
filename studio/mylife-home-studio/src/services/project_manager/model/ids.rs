use std::fmt;
use std::hash::Hash;

/// Shared behaviour of every id newtype, so `Collection` can report a failure
/// against the right kind of id, in the right project's error type, without
/// being told either one.
///
/// `Borrow<str>` is deliberately *not* a supertrait: with it in scope,
/// `HashMap::get(&K)` becomes ambiguous between `Q = str` and `Q = K`. The
/// individual id types still implement it, for lookups by `&str`.
pub trait Id: Clone + Eq + Hash + Ord + fmt::Display + fmt::Debug {
    /// The error the project this id belongs to fails with.
    type Error: IdError;

    /// What this id names, in that project's own set of kinds.
    const KIND: <Self::Error as IdError>::Kind;

    fn as_str(&self) -> &str;
}

/// The two failures `Collection` raises on its own. A project's error type
/// implements this and keeps its remaining variants to itself.
pub trait IdError {
    /// The project's id kinds, carried in the message.
    type Kind: Copy + fmt::Display;

    fn not_found(kind: Self::Kind, id: &str) -> Self;

    fn already_exists(kind: Self::Kind, id: &str) -> Self;
}

/// Defines one id newtype over `Arc<str>`, validated on construction by
/// `$validate` and reporting as `$kind`.
///
/// The expansion names `Id`, `IdError`, `Arc`, `Borrow` and `fmt`, so a module
/// using it has those in scope.
macro_rules! define_id {
    ($name:ident, $error:ty, $kind:expr, $validate:path, $doc:expr) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(Arc<str>);

        impl $name {
            pub fn new(value: impl AsRef<str>) -> Result<Self, $error> {
                let value = value.as_ref();
                $validate($kind, value)?;
                Ok(Self(Arc::from(value)))
            }
        }

        impl Id for $name {
            type Error = $error;

            const KIND: <$error as IdError>::Kind = $kind;

            fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Borrow<str> for $name {
            fn borrow(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl TryFrom<String> for $name {
            type Error = $error;

            fn try_from(value: String) -> Result<Self, $error> {
                Self::new(value)
            }
        }

        impl TryFrom<&str> for $name {
            type Error = $error;

            fn try_from(value: &str) -> Result<Self, $error> {
                Self::new(value)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> String {
                value.0.to_string()
            }
        }

        impl From<&$name> for String {
            fn from(value: &$name) -> String {
                value.0.to_string()
            }
        }
    };
}

pub(crate) use define_id;
