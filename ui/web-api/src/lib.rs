use ts_rs::Config;

pub mod socket;

// Generation helpers

pub struct TsExport {
    pub type_name: &'static str,
    pub export_impl: fn(config: &Config) -> Result<(), ts_rs::ExportError>,
}

inventory::collect!(TsExport);

#[macro_export]
macro_rules! register_ts {
    ($ty:ty) => {
        inventory::submit! {
            $crate::TsExport {
                type_name: stringify!($ty),
                export_impl: <$ty as ts_rs::TS>::export_all
            }
        }
    };
}
