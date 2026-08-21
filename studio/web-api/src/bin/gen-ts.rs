use std::{fs, path::Path};

use studio_web_api::TsExport;
use ts_rs::Config;

/// A block of hand-maintained TypeScript to append to a generated file after
/// ts-rs writes it (re-exports, namespace aliases, const companions, ...).
struct Appendix {
    file: &'static str,
    content: &'static str,
}

// TODO: remove all these hacks
const APPENDICES: &[Appendix] = &[
    Appendix {
        file: "ui-model.ts",
        content: "export * from './ui/model';",
    },
    Appendix {
        file: "project-manager.ts",
        content: "\
export * as coreImportData from './project-manager-core-import-data';
export * as coreValidation from './project-manager-core-validation';

import type { ControlDisplayMapItem } from './ui-model';
export type UiControlDisplayData = ControlDisplay;
export type UiControlDisplayMapItemData = ControlDisplayMapItem;
export type UiActionData = Action;
export type UiElementPath = UiElementPathNode[];",
    },
    Appendix {
        file: "component-model.ts",
        content: "\
export const MemberType = { STATE: 'state', ACTION: 'action' } as const;
export const ConfigType = { STRING: 'string', BOOL: 'bool', INTEGER: 'integer', FLOAT: 'float' } as const;
export const PluginUsage = { SENSOR: 'sensor', ACTUATOR: 'actuator', LOGIC: 'logic', UI: 'ui' } as const;",
    },
    Appendix {
        file: "git.ts",
        content: "\
import parseDiff from 'parse-diff';

export namespace diff {
  export type File = parseDiff.File;
  export type Chunk = parseDiff.Chunk;
  export type NormalChange = parseDiff.NormalChange;
  export type AddChange = parseDiff.AddChange;
  export type DeleteChange = parseDiff.DeleteChange;
  export type ChangeType = parseDiff.ChangeType;
  export type Change = parseDiff.Change;
}

export const DEFAULT_STATUS: GitStatus = {
  appUrl: null,
  branch: '<unknown>',
  changedFeatures: [],
  ahead: null,
  behind: null,
};",
    },
];

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

    for appendix in APPENDICES {
        let path = out_dir.join(appendix.file);
        append_block(&path, appendix.content);
    }

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

fn append_block(path: &Path, content: &str) {
    let block = format!("\n// Added by generator tool\n{content}\n");
    append_to_file(path, &block);
}