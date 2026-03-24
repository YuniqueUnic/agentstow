use std::path::{Path, PathBuf};

use agentstow_core::{
    AgentStowError, ArtifactKind, Result, ensure_parent_dir, normalize_for_display,
};
use agentstow_render::{RenderedDir, RenderedDirEntryKind};

use crate::types::ApplyOptions;

pub(crate) fn relative_or_absolute_link_target(target_path: &Path, source_path: &Path) -> PathBuf {
    let parent = target_path.parent().unwrap_or_else(|| Path::new("."));
    pathdiff::diff_paths(source_path, parent).unwrap_or_else(|| source_path.to_path_buf())
}

pub(crate) fn create_unused_path(parent: &Path, prefix: &str) -> Result<PathBuf> {
    let tmp = tempfile::Builder::new()
        .prefix(prefix)
        .tempfile_in(parent)
        .map_err(AgentStowError::from)?;
    let path = tmp.path().to_path_buf();
    tmp.close().map_err(AgentStowError::from)?;
    Ok(path)
}

pub(crate) fn rename_into_place(
    tmp_path: &Path,
    target_path: &Path,
    opt: ApplyOptions,
    target_is_dir_like: bool,
) -> Result<()> {
    if target_path.exists() {
        if !opt.force {
            return Err(AgentStowError::LinkConflict {
                message: format!("target 已存在：{}", normalize_for_display(target_path)).into(),
            });
        }

        #[cfg(unix)]
        {
            if !target_is_dir_like && fs_err::rename(tmp_path, target_path).is_ok() {
                return Ok(());
            }
        }

        remove_existing(target_path)?;
    }

    fs_err::rename(tmp_path, target_path).map_err(AgentStowError::from)?;
    Ok(())
}

pub(crate) fn create_symlink(
    link_target: &Path,
    target_path: &Path,
    kind: ArtifactKind,
) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs as unix_fs;
        match kind {
            ArtifactKind::File => {
                unix_fs::symlink(link_target, target_path).map_err(AgentStowError::from)
            }
            ArtifactKind::Dir => {
                unix_fs::symlink(link_target, target_path).map_err(AgentStowError::from)
            }
        }
    }

    #[cfg(windows)]
    {
        use std::os::windows::fs as win_fs;
        match kind {
            ArtifactKind::File => {
                win_fs::symlink_file(link_target, target_path).map_err(AgentStowError::from)
            }
            ArtifactKind::Dir => {
                win_fs::symlink_dir(link_target, target_path).map_err(AgentStowError::from)
            }
        }
    }
}

pub(crate) fn remove_existing(path: &Path) -> Result<()> {
    let meta = fs_err::symlink_metadata(path).map_err(AgentStowError::from)?;
    if meta.is_dir() && !meta.file_type().is_symlink() {
        fs_err::remove_dir_all(path).map_err(AgentStowError::from)
    } else {
        fs_err::remove_file(path).map_err(AgentStowError::from)
    }
}

pub(crate) fn atomic_write_file(path: &Path, bytes: &[u8], sync: bool) -> Result<()> {
    ensure_parent_dir(path)?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let mut tmp = tempfile::Builder::new()
        .prefix(".agentstow.")
        .tempfile_in(parent)
        .map_err(AgentStowError::from)?;
    use std::io::Write as _;
    tmp.write_all(bytes).map_err(AgentStowError::from)?;
    tmp.flush().map_err(AgentStowError::from)?;
    if sync {
        tmp.as_file().sync_all().map_err(AgentStowError::from)?;
    }

    tmp.persist(path).map_err(|e| AgentStowError::Io(e.error))?;
    Ok(())
}

pub(crate) fn materialize_rendered_dir(root: &Path, rendered: &RenderedDir) -> Result<()> {
    fs_err::create_dir_all(root).map_err(AgentStowError::from)?;
    for entry in &rendered.entries {
        let path = root.join(&entry.relative_path);
        match entry.kind {
            RenderedDirEntryKind::Dir => {
                fs_err::create_dir_all(&path).map_err(AgentStowError::from)?;
            }
            RenderedDirEntryKind::File => {
                atomic_write_file(&path, &entry.bytes, true)?;
            }
        }
    }
    Ok(())
}

pub(crate) fn copy_dir_recursive(source: &Path, dest: &Path) -> Result<()> {
    fs_err::create_dir_all(dest).map_err(AgentStowError::from)?;
    for entry in fs_err::read_dir(source).map_err(AgentStowError::from)? {
        let entry = entry.map_err(AgentStowError::from)?;
        let src_path = entry.path();
        let dst_path = dest.join(entry.file_name());
        let file_type = entry.file_type().map_err(AgentStowError::from)?;
        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else if file_type.is_file() {
            let bytes = fs_err::read(&src_path).map_err(AgentStowError::from)?;
            atomic_write_file(&dst_path, &bytes, true)?;
        }
    }
    Ok(())
}
