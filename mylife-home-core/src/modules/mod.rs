mod loader;

use std::{collections::HashMap, sync::Arc};

pub use loader::{Plugin, ModuleLoadError};

pub type Repository = HashMap<String, Arc<Plugin>>;

static mut REPOSITORY: Option<Repository> = None;

pub fn init(module_path: &str) -> Result<(), Box<dyn std::error::Error>> {
  let plugins = loader::load_modules(module_path)?;

  // Note: 
  // - This is called at init, before access
  // - This is the only possible mutation
  // I guess this is OK
  unsafe { REPOSITORY = Some(plugins) };

  Ok(())
}

pub fn repository() -> &'static Repository {
  // Note: no mutation
  unsafe { REPOSITORY.as_ref().expect("Repository not initialized!") }
}