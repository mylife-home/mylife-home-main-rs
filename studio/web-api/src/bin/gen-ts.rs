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

    // Tweaks:
    // move "model.ts" (generated from ui model) to ui/model.ts,
    // move dependencies accordingly
    // and re-export everything from ui/model.ts to ui-model.ts

    fs::create_dir_all(&out_dir.join("ui")).expect("Failed to create ui directory");
    fs::rename(
        &out_dir.join("model.ts"),
        out_dir.join("ui").join("model.ts"),
    )
    .expect("Failed to rename model.ts to ui/model.ts");

    replace_in_files(out_dir, "./model", "./ui/model");

    append_to_file(&out_dir.join("ui-model.ts"), "\n// Added by generator tool\nexport * from './ui/model';");

    println!("TypeScript bindings generated.");
}

fn cleanup(out_dir: &Path) {
    for file in fs::read_dir(out_dir).expect("Failed to read current directory") {
        let file = file.expect("Failed to read file");
        let path = file.path();
        if path.is_file() && path.extension().map(|ext| ext == "ts").unwrap_or(false) {
            fs::remove_file(&path).expect("Failed to remove file");
        }
    }
}

fn replace_in_files(out_dir: &Path, from: &str, to: &str) {
    for file in fs::read_dir(out_dir).expect("Failed to read current directory") {
        let file = file.expect("Failed to read file");
        let path = file.path();
        if path.is_file() && path.extension().map(|ext| ext == "ts").unwrap_or(false) {
            let file_path = path.to_str().expect("Failed to convert path to string");
            replace_in_file(file_path, from, to);
        }
    }
}

fn replace_in_file(file_path: &str, from: &str, to: &str) {
    let content = fs::read_to_string(file_path).expect("Failed to read file");
    let new_content = content.replace(from, to);
    fs::write(file_path, new_content).expect("Failed to write file");
}

fn append_to_file(file_path: &Path, content: &str) {
    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(file_path)
        .expect("Failed to open file for appending");
    use std::io::Write;
    writeln!(file, "{}", content).expect("Failed to write to file");
}
