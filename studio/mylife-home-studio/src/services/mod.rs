use std::env;
use std::path::{Path, PathBuf};

use common::utils::config;
use common::{InitData, utils::actors::SpawnedActors};

use crate::services::project_manager::ProjectManagerConfig;
use crate::web::DispatcherBuilder;

mod git;
mod logging;
mod online;
mod project_manager;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GitConfig {
    pub app_url: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PathsConfig {
    pub root: PathBuf,
    pub project_manager: ProjectManagerPaths,
    pub deploy: DeployPaths,
}

impl PathsConfig {
    pub fn resolve(&mut self) {
        self.root = Self::resolve_path(
            &env::current_dir().expect("failed to get current directory"),
            &self.root,
        );

        self.project_manager.core = Self::resolve_path(&self.root, &self.project_manager.core);
        self.project_manager.ui = Self::resolve_path(&self.root, &self.project_manager.ui);

        self.deploy.files = Self::resolve_path(&self.root, &self.deploy.files);
        self.deploy.recipes = Self::resolve_path(&self.root, &self.deploy.recipes);
        self.deploy.pinned_recipes_file =
            Self::resolve_path(&self.root, &self.deploy.pinned_recipes_file);
    }

    fn resolve_path(base: &Path, child: &Path) -> PathBuf {
        if child.is_absolute() {
            child.to_path_buf()
        } else {
            base.join(child)
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectManagerPaths {
    pub ui: PathBuf,
    pub core: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeployPaths {
    pub files: PathBuf,
    pub recipes: PathBuf,
    pub pinned_recipes_file: PathBuf,
}

pub async fn init(
    actors: &mut SpawnedActors,
    dispatcher: &mut DispatcherBuilder,
    init_data: &InitData,
) {
    let git: GitConfig = config::section("git");
    let mut paths: PathsConfig = config::section("paths");
    paths.resolve();

    online::init(actors, dispatcher, init_data).await;
    git::init(actors, dispatcher).await;
    logging::init(actors, dispatcher).await;

    project_manager::init(
        actors,
        dispatcher,
        ProjectManagerConfig {
            core_projects_store_path: paths.project_manager.core,
            ui_projects_store_path: paths.project_manager.ui,
        },
    )
    .await;
}
