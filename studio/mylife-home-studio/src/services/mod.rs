use std::env;
use std::path::{Path, PathBuf};

use common::utils::config;
use common::{InitData, utils::actors::SpawnedActors};

use crate::services::git::{Credentials, GitBuilder, SshKey, UserPass};
use crate::services::project_manager::ProjectManagerConfig;
use crate::web::DispatcherBuilder;

mod git;
mod logging;
mod online;
mod project_manager;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
/// The remote settings, each of them optional so that a deployment can leave
/// the ones it does not use to an unset environment variable.
struct GitConfig {
    pub app_url: String,
    /// Set together to reach an https remote.
    #[serde(default, deserialize_with = "config::empty_as_none")]
    pub username: Option<String>,
    #[serde(default, deserialize_with = "config::empty_as_none")]
    pub token: Option<String>,
    /// The absolute path of the key an ssh remote is reached with. Without
    /// one, the ssh agent and the default keys under `~/.ssh` are tried.
    #[serde(default, deserialize_with = "config::empty_as_none")]
    pub ssh_key: Option<PathBuf>,
    #[serde(default, deserialize_with = "config::empty_as_none")]
    pub ssh_key_passphrase: Option<String>,
    /// Takes the remote's host key on trust, for a deployment that carries no
    /// `known_hosts`.
    #[serde(default)]
    pub accept_unknown_host: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct PathsConfig {
    pub root: PathBuf,
    pub project_manager: ProjectManagerPaths,
}

impl PathsConfig {
    pub fn resolve(&mut self) {
        self.root = Self::resolve_path(
            &env::current_dir().expect("failed to get current directory"),
            &self.root,
        );

        self.project_manager.core = Self::resolve_path(&self.root, &self.project_manager.core);
        self.project_manager.ui = Self::resolve_path(&self.root, &self.project_manager.ui);
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
struct ProjectManagerPaths {
    pub ui: PathBuf,
    pub core: PathBuf,
}

pub async fn init(
    actors: &mut SpawnedActors,
    dispatcher: &mut DispatcherBuilder,
    init_data: &InitData,
) {
    let git_config: GitConfig = config::section("git");
    let mut paths: PathsConfig = config::section("paths");
    paths.resolve();

    // Git is started last: the services register the paths they own on the
    // builder as they come up, and the actor is built knowing all of them.
    let mut git_builder = GitBuilder::new(paths.root.clone(), Some(git_config.app_url));

    let mut credentials = Credentials {
        accept_unknown_host: git_config.accept_unknown_host,
        ..Credentials::default()
    };

    if let (Some(username), Some(token)) = (git_config.username, git_config.token) {
        credentials.userpass = Some(UserPass { username, token });
    }

    if let Some(path) = git_config.ssh_key {
        credentials.ssh_key = Some(SshKey {
            path,
            passphrase: git_config.ssh_key_passphrase,
        });
    }

    git_builder.credentials(credentials);

    online::init(actors, dispatcher, init_data).await;
    logging::init(actors, dispatcher).await;

    project_manager::init(
        actors,
        dispatcher,
        &mut git_builder,
        ProjectManagerConfig {
            core_projects_store_path: paths.project_manager.core,
            ui_projects_store_path: paths.project_manager.ui,
        },
    )
    .await;

    git::init(actors, dispatcher, git_builder).await;
}
