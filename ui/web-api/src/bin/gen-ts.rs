use std::{fs, path::Path};

use ts_rs::Config;
use ui_web_api::TsExport;

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
