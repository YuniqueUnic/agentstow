use std::collections::HashSet;
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

use agentstow_core::{ArtifactKind, InstallMethod, Result};
use agentstow_manifest::Manifest;
use agentstow_render::Renderer;
use agentstow_state::LinkInstanceRecord;

use crate::types::{InstallSource, LinkJob, RenderStore};

pub fn check_link_job_health(job: &LinkJob, render_store: &RenderStore) -> Result<bool> {
    match job.method {
        InstallMethod::Symlink => match (&job.artifact_kind, &job.desired) {
            (ArtifactKind::File, InstallSource::FileBytes(_)) => {
                let desired_source =
                    render_store.rendered_file_path(&job.artifact_id, &job.profile);
                check_existing_link_health(&job.target_path, &desired_source, check_symlink)
            }
            (ArtifactKind::Dir, InstallSource::Path(path)) => {
                check_existing_link_health(&job.target_path, path, check_symlink)
            }
            _ => Ok(false),
        },
        InstallMethod::Junction => match (&job.artifact_kind, &job.desired) {
            (ArtifactKind::Dir, InstallSource::Path(path)) => {
                check_junction(&job.target_path, path)
            }
            _ => Ok(false),
        },
        InstallMethod::Copy => match (&job.artifact_kind, &job.desired) {
            (ArtifactKind::File, InstallSource::FileBytes(bytes)) => {
                check_copy_file(&job.target_path, bytes)
            }
            (ArtifactKind::Dir, InstallSource::Path(path)) => {
                check_copy_dir(&job.target_path, path)
            }
            _ => Ok(false),
        },
    }
}

pub fn check_link_record_health(manifest: &Manifest, record: &LinkInstanceRecord) -> Result<bool> {
    let Some(artifact_def) = manifest.artifacts.get(&record.artifact_id) else {
        return Ok(false);
    };

    match record.method {
        InstallMethod::Symlink => match record.rendered_path.as_deref() {
            Some(source_path) => {
                check_existing_link_health(&record.target_path, source_path, check_symlink)
            }
            None => Ok(false),
        },
        InstallMethod::Junction => match record.rendered_path.as_deref() {
            Some(source_path) => check_junction(&record.target_path, source_path),
            None => Ok(false),
        },
        InstallMethod::Copy => match artifact_def.kind {
            ArtifactKind::File => {
                let rendered =
                    Renderer::render_file(manifest, &record.artifact_id, &record.profile)?;
                check_copy_file(&record.target_path, &rendered.bytes)
            }
            ArtifactKind::Dir => {
                let desired_source = record
                    .rendered_path
                    .as_deref()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| artifact_def.source_path(&manifest.workspace_root));
                check_copy_dir(&record.target_path, &desired_source)
            }
        },
    }
}

pub fn check_symlink(target: &Path, desired_source: &Path) -> Result<bool> {
    is_correct_symlink(target, desired_source)
}

pub fn check_junction(target: &Path, desired_source: &Path) -> Result<bool> {
    #[cfg(windows)]
    {
        if !target.exists() || !desired_source.exists() {
            return Ok(false);
        }
        if !target.is_dir() || !desired_source.is_dir() {
            return Ok(false);
        }
        return same_file::is_same_file(target, desired_source)
            .map_err(agentstow_core::AgentStowError::from);
    }

    #[cfg(not(windows))]
    {
        let _ = (target, desired_source);
        Ok(false)
    }
}

pub fn check_copy_dir(target: &Path, desired_source: &Path) -> Result<bool> {
    if !target.is_dir() || !desired_source.is_dir() {
        return Ok(false);
    }

    compare_dir_trees(desired_source, target)
}

pub(crate) fn is_correct_symlink(target: &Path, desired_source: &Path) -> Result<bool> {
    let meta = fs_err::symlink_metadata(target).map_err(agentstow_core::AgentStowError::from)?;
    if !meta.file_type().is_symlink() {
        return Ok(false);
    }
    let resolved = resolve_link_path(target)?;
    let desired_source = normalize_path_without_following_symlinks(desired_source);

    if !resolved.exists() || !desired_source.exists() {
        return Ok(false);
    }

    Ok(resolved == desired_source)
}

pub(crate) fn is_target_symlink_path_match(target: &Path, desired_source: &Path) -> Result<bool> {
    let meta = match fs_err::symlink_metadata(target) {
        Ok(meta) => meta,
        Err(_) => return Ok(false),
    };
    if !meta.file_type().is_symlink() {
        return Ok(false);
    }

    Ok(resolve_link_path(target)? == normalize_path_without_following_symlinks(desired_source))
}

pub(crate) fn check_copy_file(target: &Path, desired_bytes: &[u8]) -> Result<bool> {
    if !target.is_file() {
        return Ok(false);
    }
    let existing = fs_err::read(target).map_err(agentstow_core::AgentStowError::from)?;
    Ok(existing == desired_bytes)
}

fn check_existing_link_health(
    target: &Path,
    desired_source: &Path,
    checker: fn(&Path, &Path) -> Result<bool>,
) -> Result<bool> {
    if !desired_source.exists() {
        return Ok(false);
    }
    if fs_err::symlink_metadata(target).is_err() {
        return Ok(false);
    }
    checker(target, desired_source)
}

fn resolve_link_path(target: &Path) -> Result<PathBuf> {
    let link = fs_err::read_link(target).map_err(agentstow_core::AgentStowError::from)?;
    let candidate = if link.is_absolute() {
        link
    } else {
        target.parent().unwrap_or_else(|| Path::new(".")).join(link)
    };
    Ok(normalize_path_without_following_symlinks(&candidate))
}

fn normalize_path_without_following_symlinks(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(Path::new(std::path::MAIN_SEPARATOR_STR)),
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    normalized.push(Component::ParentDir.as_os_str());
                }
            }
            Component::Normal(part) => normalized.push(part),
        }
    }

    normalized
}

fn compare_dir_trees(source: &Path, target: &Path) -> Result<bool> {
    let mut source_entries = HashSet::<OsString>::new();

    for entry in fs_err::read_dir(source).map_err(agentstow_core::AgentStowError::from)? {
        let entry = entry.map_err(agentstow_core::AgentStowError::from)?;
        let file_name = entry.file_name();
        source_entries.insert(file_name.clone());

        let source_path = entry.path();
        let target_path = target.join(&file_name);
        if !target_path.exists() {
            return Ok(false);
        }

        let file_type = entry
            .file_type()
            .map_err(agentstow_core::AgentStowError::from)?;
        if file_type.is_dir() {
            if !target_path.is_dir() || !compare_dir_trees(&source_path, &target_path)? {
                return Ok(false);
            }
        } else if file_type.is_file() {
            if !target_path.is_file() {
                return Ok(false);
            }
            let source_bytes =
                fs_err::read(&source_path).map_err(agentstow_core::AgentStowError::from)?;
            let target_bytes =
                fs_err::read(&target_path).map_err(agentstow_core::AgentStowError::from)?;
            if source_bytes != target_bytes {
                return Ok(false);
            }
        } else {
            return Ok(false);
        }
    }

    for entry in fs_err::read_dir(target).map_err(agentstow_core::AgentStowError::from)? {
        let entry = entry.map_err(agentstow_core::AgentStowError::from)?;
        if !source_entries.contains(&entry.file_name()) {
            return Ok(false);
        }
    }

    Ok(true)
}
