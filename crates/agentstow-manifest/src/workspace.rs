use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use agentstow_core::{AgentStowError, Result};

use crate::DEFAULT_MANIFEST_FILE;

#[derive(Debug, Clone)]
pub struct WorkspaceInitOutcome {
    pub workspace_root: PathBuf,
    pub manifest_path: PathBuf,
    pub created: bool,
}

#[derive(Debug, Clone)]
pub struct WorkspaceProbe {
    pub resolved_workspace_root: PathBuf,
    pub manifest_path: PathBuf,
    pub exists: bool,
    pub is_directory: bool,
    pub manifest_present: bool,
    pub git_present: bool,
    pub selectable: bool,
    pub initializable: bool,
    pub reason: Option<String>,
}

pub fn init_workspace_skeleton(workspace_root: &Path) -> Result<WorkspaceInitOutcome> {
    fs_err::create_dir_all(workspace_root).map_err(AgentStowError::from)?;

    let manifest_path = workspace_root.join(DEFAULT_MANIFEST_FILE);
    let created = if manifest_path.exists() {
        false
    } else {
        fs_err::create_dir_all(workspace_root.join("artifacts")).map_err(AgentStowError::from)?;
        fs_err::write(
            workspace_root.join("artifacts/hello.txt.tera"),
            "Hello {{ name }}!",
        )
        .map_err(AgentStowError::from)?;
        fs_err::write(
            &manifest_path,
            r#"[profiles.base]
name = "AgentStow"

[artifacts.hello]
kind = "file"
source = "artifacts/hello.txt.tera"
template = true
validate_as = "none"
"#,
        )
        .map_err(AgentStowError::from)?;
        true
    };

    Ok(WorkspaceInitOutcome {
        workspace_root: workspace_root.to_path_buf(),
        manifest_path,
        created,
    })
}

pub fn probe_workspace_path(requested_path: &Path) -> Result<WorkspaceProbe> {
    let requested_path = absolutize_requested_path(requested_path)?;
    let manifest_file_request = is_manifest_file_path(&requested_path);
    let resolved_workspace_root = if manifest_file_request {
        requested_path
            .parent()
            .ok_or_else(|| AgentStowError::Manifest {
                message: "workspace probe 路径没有 parent".into(),
            })?
            .to_path_buf()
    } else {
        requested_path.clone()
    };

    if requested_path.exists() {
        let metadata = fs_err::metadata(&requested_path).map_err(AgentStowError::from)?;
        if metadata.is_dir() {
            return build_existing_workspace_probe(requested_path);
        }
        if manifest_file_request && metadata.is_file() {
            let resolved_workspace_root =
                fs_err::canonicalize(&resolved_workspace_root).map_err(AgentStowError::from)?;
            return Ok(WorkspaceProbe {
                manifest_path: resolved_workspace_root.join(DEFAULT_MANIFEST_FILE),
                git_present: resolved_workspace_root.join(".git").exists(),
                resolved_workspace_root,
                exists: true,
                is_directory: false,
                manifest_present: true,
                selectable: true,
                initializable: false,
                reason: None,
            });
        }

        return Ok(WorkspaceProbe {
            resolved_workspace_root,
            manifest_path: requested_path,
            exists: true,
            is_directory: false,
            manifest_present: false,
            git_present: false,
            selectable: false,
            initializable: false,
            reason: Some("路径是普通文件，不是 workspace 目录，也不是 agentstow.toml".to_string()),
        });
    }

    let reason = if manifest_file_request {
        "manifest 尚不存在，可初始化其所在目录为 workspace"
    } else {
        "路径不存在，可直接初始化 workspace"
    };

    Ok(WorkspaceProbe {
        manifest_path: if manifest_file_request {
            requested_path
        } else {
            resolved_workspace_root.join(DEFAULT_MANIFEST_FILE)
        },
        resolved_workspace_root,
        exists: false,
        is_directory: false,
        manifest_present: false,
        git_present: false,
        selectable: false,
        initializable: true,
        reason: Some(reason.to_string()),
    })
}

fn build_existing_workspace_probe(path: PathBuf) -> Result<WorkspaceProbe> {
    let resolved_workspace_root = fs_err::canonicalize(&path).map_err(AgentStowError::from)?;
    let manifest_path = resolved_workspace_root.join(DEFAULT_MANIFEST_FILE);
    let manifest_present = manifest_path.is_file();

    Ok(WorkspaceProbe {
        git_present: resolved_workspace_root.join(".git").exists(),
        resolved_workspace_root,
        manifest_path,
        exists: true,
        is_directory: true,
        manifest_present,
        selectable: true,
        initializable: !manifest_present,
        reason: (!manifest_present)
            .then_some("目录存在，但还没有 agentstow.toml，可直接初始化".to_string()),
    })
}

fn absolutize_requested_path(requested_path: &Path) -> Result<PathBuf> {
    if requested_path.is_absolute() {
        return Ok(requested_path.to_path_buf());
    }

    Ok(std::env::current_dir()
        .map_err(AgentStowError::from)?
        .join(requested_path))
}

fn is_manifest_file_path(path: &Path) -> bool {
    path.file_name()
        .is_some_and(|name| name == OsStr::new(DEFAULT_MANIFEST_FILE))
}
