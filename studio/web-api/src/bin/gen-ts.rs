//! Writes the TypeScript the web app imports, one file per module, from the
//! wire types themselves. Nothing is added to what ts-rs emits: a shape the
//! app needs that no Rust type can express belongs in the app.

use std::{fs, path::Path};

use studio_web_api::TsExport;
use ts_rs::Config;

fn main() {
    let config = Config::from_env();

    let out_dir = config.out_dir();

    cleanup(out_dir);

    for item in inventory::iter::<TsExport>() {
        (item.export_impl)(&config)
            .unwrap_or_else(|e| panic!("failed to export {}: {}", item.type_name, e));
        println!("Generated binding for {}", item.type_name);
    }

    println!("TypeScript bindings generated.");
}

/// Drops what the last run wrote, so a type that is gone leaves nothing behind.
fn cleanup(out_dir: &Path) {
    if !out_dir.exists() {
        fs::create_dir_all(out_dir).expect("Failed to create output directory");
    }

    for file in fs::read_dir(out_dir).expect("Failed to read current directory") {
        let file = file.expect("Failed to read file");
        let path = file.path();
        if path.is_file() && path.extension().map(|ext| ext == "ts").unwrap_or(false) {
            fs::remove_file(&path).expect("Failed to remove file");
        }
    }
}
