use std::path::{Path, PathBuf};

use agentstow_core::{ArtifactId, ArtifactKind, InstallMethod, ProfileName, Result, TargetName};
use agentstow_render::RenderedDir;
use serde::Serialize;

use crate::fsops::{atomic_write_file, materialize_rendered_dir, remove_existing};

#[derive(Debug, Clone, Serialize)]
pub struct LinkPlanItem {
    pub target: TargetName,
    pub artifact_id: ArtifactId,
    pub profile: ProfileName,
    pub artifact_kind: ArtifactKind,
    pub method: InstallMethod,
    pub target_path: PathBuf,
    pub desired: DesiredInstall,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DesiredInstall {
    Symlink { source_path: PathBuf },
    Junction { source_path: PathBuf },
    Copy { blake3: String, bytes_len: usize },
}

#[derive(Debug, Clone)]
pub struct RenderStore {
    root: PathBuf,
    workspace_key: String,
}

impl RenderStore {
    pub fn new(root: PathBuf, workspace_root: &Path) -> Self {
        let key = blake3::hash(workspace_root.to_string_lossy().as_bytes());
        Self {
            root,
            workspace_key: key.to_hex()[..12].to_string(),
        }
    }

    pub fn rendered_file_path(&self, artifact_id: &ArtifactId, profile: &ProfileName) -> PathBuf {
        self.root
            .join(&self.workspace_key)
            .join("rendered")
            .join(format!(
                "{}__{}.out",
                artifact_id.as_str(),
                profile.as_str()
            ))
    }

    pub fn write_rendered_file(
        &self,
        artifact_id: &ArtifactId,
        profile: &ProfileName,
        bytes: &[u8],
    ) -> Result<PathBuf> {
        let path = self.rendered_file_path(artifact_id, profile);
        atomic_write_file(&path, bytes, true)?;
        Ok(path)
    }

    pub fn rendered_dir_path(&self, artifact_id: &ArtifactId, profile: &ProfileName) -> PathBuf {
        self.root
            .join(&self.workspace_key)
            .join("rendered")
            .join(format!("{}__{}", artifact_id.as_str(), profile.as_str()))
    }

    pub fn write_rendered_dir(
        &self,
        artifact_id: &ArtifactId,
        profile: &ProfileName,
        rendered: &RenderedDir,
    ) -> Result<PathBuf> {
        let path = self.rendered_dir_path(artifact_id, profile);
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        fs_err::create_dir_all(parent).map_err(agentstow_core::AgentStowError::from)?;

        let staging = tempfile::Builder::new()
            .prefix(".agentstow.rendered-dir.")
            .tempdir_in(parent)
            .map_err(agentstow_core::AgentStowError::from)?;
        materialize_rendered_dir(staging.path(), rendered)?;

        if path.exists() {
            remove_existing(&path)?;
        }
        fs_err::rename(staging.path(), &path).map_err(agentstow_core::AgentStowError::from)?;
        Ok(path)
    }
}

#[derive(Debug, Clone)]
pub struct LinkJob {
    pub target: TargetName,
    pub artifact_id: ArtifactId,
    pub profile: ProfileName,
    pub artifact_kind: ArtifactKind,
    pub method: InstallMethod,
    pub target_path: PathBuf,
    pub desired: InstallSource,
}

#[derive(Debug, Clone)]
pub enum InstallSource {
    FileBytes(Vec<u8>),
    Path(PathBuf),
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ApplyOptions {
    pub force: bool,
}
