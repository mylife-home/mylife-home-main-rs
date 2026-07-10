use ts_rs::Config;
use ui_web_api::TsExport;

fn main() {
    let config = Config::from_env();

    for item in inventory::iter::<TsExport>() {
        (item.export_impl)(&config)
            .unwrap_or_else(|e| panic!("failed to export {}: {}", item.type_name, e));
        println!("Generated binding for {}", item.type_name);
    }

    println!("TypeScript bindings generated.");
}
