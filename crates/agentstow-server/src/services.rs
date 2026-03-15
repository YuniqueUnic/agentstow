mod artifact;
mod common;
mod env;
mod git;
mod issues;
mod links;
mod mcp;
mod profile;
mod scripts;
mod summary;
mod workspace;

use std::path::PathBuf;

use agentstow_core::{AgentStowDirs, Result};
use agentstow_manifest::Manifest;
use agentstow_state::StateDb;
use agentstow_web_types::LinkStatusResponseItem;

pub(crate) use common::watch_status_response;

#[derive(Debug, Clone)]
pub(crate) struct WorkspaceQueryService {
    workspace_root: PathBuf,
}

impl WorkspaceQueryService {
    pub(crate) fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    fn load_manifest(&self) -> Result<Manifest> {
        Manifest::load_from_dir(&self.workspace_root)
    }

    fn open_state_db(&self) -> Result<StateDb> {
        let dirs = AgentStowDirs::from_env()?;
        StateDb::open(&dirs)
    }

    fn compute_link_status(&self, manifest: &Manifest) -> Result<Vec<LinkStatusResponseItem>> {
        let db = self.open_state_db()?;
        let records = db.list_link_instances(&self.workspace_root)?;
        Ok(records
            .into_iter()
            .map(|record| links::link_status_item(manifest, &record))
            .collect())
    }
}
