// project opened once
// each opened tab in the ui has its own notifier
// project reloaded when changed externaly (+'reset' notification)
// opened project can add/remove notifiers (for new open/close/session close)
// when an opened project has no notifier anymore, it is closed (removed from the opened projects list)
// project can be renamed while opened


use std::{collections::HashMap, fmt::Debug};

use studio_web_api::project_manager;

use crate::web::SessionEvent;

/// Represents an opened project within the system.
pub trait OpenedProject : Debug + Send + Sync {
    /// Renames the opened project to the specified new name.
    fn rename(&mut self, new_name: &str);

    /// Reloads the opened project, typically used when the project has been updated externally.
    fn reload(&mut self);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct OpenedProjectId(usize);

/// Manages the collection of currently opened projects.
#[derive(Debug)]
pub struct OpenedProjects {
    next_id: usize,
    projects: HashMap<OpenedProjectId, Box<dyn OpenedProject>>,
    names: HashMap<String, OpenedProjectId>,
    notifiers: HashMap<String, OpenedProjectId>,
}

impl OpenedProjects {
    /// Creates a new instance of `OpenedProjects`.
    pub fn new() -> Self {
        Self {
            next_id: 1,
            projects: HashMap::new(),
            names: HashMap::new(),
            notifiers: HashMap::new(),
        }
    }

    /// Reloads the project with the specified type and ID if it has been updated externally.
    pub fn reload_project(&mut self, ty: project_manager::ProjectType, id: &str) {
        let Some(&project_id) = self.names.get(&Self::make_name(ty, id)) else {
            // not opened
            return;
        };

        let project = self.projects.get_mut(&project_id).expect("Project should exist");

        project.reload();
    }

    /// Handles a session event for the opened projects.
    pub fn session_event(&mut self, event: SessionEvent) {
    }

    /// Opens a project of the specified type and name.
    pub fn open_project(&mut self, ty: project_manager::ProjectType, name: &str) {
    }

    /// Closes the project with the specified ID.
    pub fn close_project(&mut self, id: &str) {
    }

    /// Calls a project with the specified ID.
    pub fn call_project(&mut self, id: &str) {
    }

    /// Renames an opened project from `old_name` to `new_name`.
    pub fn rename_project(&mut self, ty: project_manager::ProjectType, old_name: &str, new_name: &str) {
        let old_id = Self::make_name(ty, old_name);
        let Some(&project_id) = self.names.get(&old_id) else {
            // not opened
            return;
        };

        let project = self.projects.get_mut(&project_id).expect("Project should exist");
        project.rename(new_name);

        self.names.remove(&old_id);
        self.names.insert(Self::make_name(ty, new_name), project_id);
    }

    fn make_name(ty: project_manager::ProjectType, name: &str) -> String {
        let ty_str = match ty {
            project_manager::ProjectType::Core => "core",
            project_manager::ProjectType::Ui => "ui",
        };

        format!("{}:{}", ty_str, name)
    }
}