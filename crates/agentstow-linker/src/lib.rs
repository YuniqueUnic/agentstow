use std::collections::HashSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use agentstow_core::{
    AgentStowError, ArtifactId, ArtifactKind, InstallMethod, ProfileName, Result, TargetName,
    ensure_parent_dir, normalize_for_display,
};
use serde::Serialize;
use tracing::{info, instrument};

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
    /// 适用于 file artifact：写入 bytes（copy），或写入 RenderStore 后再 symlink。
    FileBytes(Vec<u8>),
    /// 适用于 dir artifact：直接 link/copy 目录。
    Path(PathBuf),
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ApplyOptions {
    pub force: bool,
}

#[instrument(skip_all, fields(target=%job.target, method=?job.method, target_path=%normalize_for_display(&job.target_path)))]
pub fn apply_job(
    job: &LinkJob,
    render_store: &RenderStore,
    opt: ApplyOptions,
) -> Result<LinkPlanItem> {
    match job.method {
        InstallMethod::Symlink => apply_symlink(job, render_store, opt),
        InstallMethod::Junction => apply_junction(job, opt),
        InstallMethod::Copy => apply_copy(job, opt),
    }
}

pub fn plan_job(job: &LinkJob, render_store: &RenderStore) -> Result<LinkPlanItem> {
    let desired = match job.method {
        InstallMethod::Symlink => match (&job.artifact_kind, &job.desired) {
            (ArtifactKind::File, InstallSource::FileBytes(_bytes)) => DesiredInstall::Symlink {
                source_path: render_store.rendered_file_path(&job.artifact_id, &job.profile),
            },
            (ArtifactKind::Dir, InstallSource::Path(p)) => DesiredInstall::Symlink {
                source_path: p.clone(),
            },
            (kind, _) => {
                return Err(AgentStowError::Link {
                    message: format!("symlink 不支持的 artifact kind/source 组合: {kind:?}").into(),
                });
            }
        },
        InstallMethod::Junction => match (&job.artifact_kind, &job.desired) {
            (ArtifactKind::Dir, InstallSource::Path(p)) => DesiredInstall::Junction {
                source_path: p.clone(),
            },
            _ => {
                return Err(AgentStowError::Link {
                    message: "junction 仅支持 dir artifact".into(),
                });
            }
        },
        InstallMethod::Copy => match (&job.artifact_kind, &job.desired) {
            (ArtifactKind::File, InstallSource::FileBytes(bytes)) => {
                let desired_hash = blake3::hash(bytes);
                DesiredInstall::Copy {
                    blake3: desired_hash.to_hex().to_string(),
                    bytes_len: bytes.len(),
                }
            }
            (ArtifactKind::Dir, InstallSource::Path(_)) => DesiredInstall::Copy {
                blake3: "<dir>".to_string(),
                bytes_len: 0,
            },
            _ => {
                return Err(AgentStowError::Link {
                    message: "copy 不支持的 artifact kind/source 组合".into(),
                });
            }
        },
    };

    Ok(LinkPlanItem {
        target: job.target.clone(),
        artifact_id: job.artifact_id.clone(),
        profile: job.profile.clone(),
        artifact_kind: job.artifact_kind,
        method: job.method,
        target_path: job.target_path.clone(),
        desired,
    })
}

fn apply_symlink(
    job: &LinkJob,
    render_store: &RenderStore,
    opt: ApplyOptions,
) -> Result<LinkPlanItem> {
    let (source_path, desired) = match (&job.artifact_kind, &job.desired) {
        (ArtifactKind::File, InstallSource::FileBytes(bytes)) => {
            let rendered_path =
                render_store.write_rendered_file(&job.artifact_id, &job.profile, bytes)?;
            (
                rendered_path,
                DesiredInstall::Symlink {
                    source_path: render_store.rendered_file_path(&job.artifact_id, &job.profile),
                },
            )
        }
        (ArtifactKind::Dir, InstallSource::Path(p)) => (
            p.clone(),
            DesiredInstall::Symlink {
                source_path: p.clone(),
            },
        ),
        (kind, _) => {
            return Err(AgentStowError::Link {
                message: format!("symlink 不支持的 artifact kind/source 组合: {kind:?}").into(),
            });
        }
    };

    ensure_parent_dir(&job.target_path)?;

    if job.target_path.exists() {
        if is_correct_symlink(&job.target_path, &source_path)? {
            info!(
                "target 已是正确 symlink，跳过: {}",
                normalize_for_display(&job.target_path)
            );
            return Ok(LinkPlanItem {
                target: job.target.clone(),
                artifact_id: job.artifact_id.clone(),
                profile: job.profile.clone(),
                artifact_kind: job.artifact_kind,
                method: job.method,
                target_path: job.target_path.clone(),
                desired,
            });
        }
        if !opt.force {
            return Err(AgentStowError::LinkConflict {
                message: format!(
                    "target 已存在且不是期望的 symlink: {}",
                    normalize_for_display(&job.target_path)
                )
                .into(),
            });
        }
    }

    let parent = job.target_path.parent().unwrap_or_else(|| Path::new("."));
    let tmp_path = create_unused_path(parent, ".agentstow.link.")?;
    let link_target = relative_or_absolute_link_target(&job.target_path, &source_path);
    create_symlink(&link_target, &tmp_path, job.artifact_kind)?;
    if let Err(e) = rename_into_place(
        &tmp_path,
        &job.target_path,
        opt,
        job.artifact_kind == ArtifactKind::Dir,
    ) {
        let _ = remove_existing(&tmp_path);
        return Err(e);
    }

    Ok(LinkPlanItem {
        target: job.target.clone(),
        artifact_id: job.artifact_id.clone(),
        profile: job.profile.clone(),
        artifact_kind: job.artifact_kind,
        method: job.method,
        target_path: job.target_path.clone(),
        desired,
    })
}

fn apply_junction(job: &LinkJob, opt: ApplyOptions) -> Result<LinkPlanItem> {
    if job.artifact_kind != ArtifactKind::Dir {
        return Err(AgentStowError::Link {
            message: "junction 仅支持 dir artifact".into(),
        });
    }
    let InstallSource::Path(source_path) = &job.desired else {
        return Err(AgentStowError::Link {
            message: "junction 需要 source path".into(),
        });
    };

    ensure_parent_dir(&job.target_path)?;

    if job.target_path.exists() {
        if check_junction(&job.target_path, source_path).unwrap_or(false) {
            return Ok(LinkPlanItem {
                target: job.target.clone(),
                artifact_id: job.artifact_id.clone(),
                profile: job.profile.clone(),
                artifact_kind: job.artifact_kind,
                method: job.method,
                target_path: job.target_path.clone(),
                desired: DesiredInstall::Junction {
                    source_path: source_path.clone(),
                },
            });
        }
        if !opt.force {
            return Err(AgentStowError::LinkConflict {
                message: format!(
                    "target 已存在且不是期望的 junction: {}",
                    normalize_for_display(&job.target_path)
                )
                .into(),
            });
        }
    }

    #[cfg(windows)]
    {
        let parent = job.target_path.parent().unwrap_or_else(|| Path::new("."));
        let tmp_path = create_unused_path(parent, ".agentstow.junction.")?;
        junction::create(source_path, &tmp_path).map_err(AgentStowError::from)?;
        if let Err(e) = rename_into_place(&tmp_path, &job.target_path, opt, true) {
            let _ = remove_existing(&tmp_path);
            return Err(e);
        }
        return Ok(LinkPlanItem {
            target: job.target.clone(),
            artifact_id: job.artifact_id.clone(),
            profile: job.profile.clone(),
            artifact_kind: job.artifact_kind,
            method: job.method,
            target_path: job.target_path.clone(),
            desired: DesiredInstall::Junction {
                source_path: source_path.clone(),
            },
        });
    }

    #[cfg(not(windows))]
    {
        Err(AgentStowError::Link {
            message: "junction 仅支持 Windows".into(),
        })
    }
}

fn apply_copy(job: &LinkJob, opt: ApplyOptions) -> Result<LinkPlanItem> {
    match (&job.artifact_kind, &job.desired) {
        (ArtifactKind::File, InstallSource::FileBytes(bytes)) => {
            let desired_hash = blake3::hash(bytes);
            ensure_parent_dir(&job.target_path)?;

            if job.target_path.is_file() {
                let existing = fs_err::read(&job.target_path).map_err(AgentStowError::from)?;
                if blake3::hash(&existing) == desired_hash {
                    return Ok(LinkPlanItem {
                        target: job.target.clone(),
                        artifact_id: job.artifact_id.clone(),
                        profile: job.profile.clone(),
                        artifact_kind: job.artifact_kind,
                        method: job.method,
                        target_path: job.target_path.clone(),
                        desired: DesiredInstall::Copy {
                            blake3: desired_hash.to_hex().to_string(),
                            bytes_len: bytes.len(),
                        },
                    });
                }
                if !opt.force {
                    return Err(AgentStowError::LinkConflict {
                        message: format!(
                            "target 已存在且内容不同: {}",
                            normalize_for_display(&job.target_path)
                        )
                        .into(),
                    });
                }
            } else if job.target_path.exists() && !opt.force {
                return Err(AgentStowError::LinkConflict {
                    message: format!(
                        "target 已存在且不是文件: {}",
                        normalize_for_display(&job.target_path)
                    )
                    .into(),
                });
            } else if job.target_path.exists() {
                remove_existing(&job.target_path)?;
            }

            atomic_write_file(&job.target_path, bytes, true)?;

            Ok(LinkPlanItem {
                target: job.target.clone(),
                artifact_id: job.artifact_id.clone(),
                profile: job.profile.clone(),
                artifact_kind: job.artifact_kind,
                method: job.method,
                target_path: job.target_path.clone(),
                desired: DesiredInstall::Copy {
                    blake3: desired_hash.to_hex().to_string(),
                    bytes_len: bytes.len(),
                },
            })
        }
        (ArtifactKind::Dir, InstallSource::Path(source_dir)) => {
            if !source_dir.is_dir() {
                return Err(AgentStowError::Link {
                    message: format!("source 不是目录: {}", normalize_for_display(source_dir))
                        .into(),
                });
            }
            if job.target_path.exists() && !opt.force {
                return Err(AgentStowError::LinkConflict {
                    message: format!("target 已存在: {}", normalize_for_display(&job.target_path))
                        .into(),
                });
            }
            ensure_parent_dir(&job.target_path)?;
            let parent = job.target_path.parent().unwrap_or_else(|| Path::new("."));
            let staging = tempfile::Builder::new()
                .prefix(".agentstow.dir.")
                .tempdir_in(parent)
                .map_err(AgentStowError::from)?;
            copy_dir_recursive(source_dir, staging.path())?;
            rename_into_place(staging.path(), &job.target_path, opt, true)?;
            Ok(LinkPlanItem {
                target: job.target.clone(),
                artifact_id: job.artifact_id.clone(),
                profile: job.profile.clone(),
                artifact_kind: job.artifact_kind,
                method: job.method,
                target_path: job.target_path.clone(),
                desired: DesiredInstall::Copy {
                    blake3: "<dir>".to_string(),
                    bytes_len: 0,
                },
            })
        }
        _ => Err(AgentStowError::Link {
            message: "copy 不支持的 artifact kind/source 组合".into(),
        }),
    }
}

fn is_correct_symlink(target: &Path, desired_source: &Path) -> Result<bool> {
    let meta = fs_err::symlink_metadata(target).map_err(AgentStowError::from)?;
    if !meta.file_type().is_symlink() {
        return Ok(false);
    }
    let link = fs_err::read_link(target).map_err(AgentStowError::from)?;
    let resolved = if link.is_absolute() {
        link
    } else {
        target.parent().unwrap_or_else(|| Path::new(".")).join(link)
    };

    Ok(dunce::simplified(&resolved) == dunce::simplified(desired_source))
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
        return same_file::is_same_file(target, desired_source).map_err(AgentStowError::from);
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

fn relative_or_absolute_link_target(target_path: &Path, source_path: &Path) -> PathBuf {
    let parent = target_path.parent().unwrap_or_else(|| Path::new("."));
    pathdiff::diff_paths(source_path, parent).unwrap_or_else(|| source_path.to_path_buf())
}

fn create_unused_path(parent: &Path, prefix: &str) -> Result<PathBuf> {
    let tmp = tempfile::Builder::new()
        .prefix(prefix)
        .tempfile_in(parent)
        .map_err(AgentStowError::from)?;
    let path = tmp.path().to_path_buf();
    tmp.close().map_err(AgentStowError::from)?;
    Ok(path)
}

fn rename_into_place(
    tmp_path: &Path,
    target_path: &Path,
    opt: ApplyOptions,
    target_is_dir_like: bool,
) -> Result<()> {
    // Must handle Windows semantics (rename() doesn't overwrite) and directory replacement cases.
    if target_path.exists() {
        if !opt.force {
            return Err(AgentStowError::LinkConflict {
                message: format!("target 已存在: {}", normalize_for_display(target_path)).into(),
            });
        }

        #[cfg(unix)]
        {
            // For file-like targets, we prefer atomic rename-over when possible.
            if !target_is_dir_like && fs_err::rename(tmp_path, target_path).is_ok() {
                return Ok(());
            }
        }

        remove_existing(target_path)?;
    }

    fs_err::rename(tmp_path, target_path).map_err(AgentStowError::from)?;
    Ok(())
}

fn create_symlink(link_target: &Path, target_path: &Path, kind: ArtifactKind) -> Result<()> {
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

fn remove_existing(path: &Path) -> Result<()> {
    let meta = fs_err::symlink_metadata(path).map_err(AgentStowError::from)?;
    if meta.is_dir() && !meta.file_type().is_symlink() {
        fs_err::remove_dir_all(path).map_err(AgentStowError::from)
    } else {
        fs_err::remove_file(path).map_err(AgentStowError::from)
    }
}

fn atomic_write_file(path: &Path, bytes: &[u8], sync: bool) -> Result<()> {
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

    // Best effort overwrite semantics:
    // - Unix rename() overwrites atomically.
    // - Windows：`tempfile::NamedTempFile::persist` 底层使用 MoveFileEx(REPLACE_EXISTING)
    //   来原子替换已存在目标（详见 tempfile 文档/实现）。
    tmp.persist(path).map_err(|e| AgentStowError::Io(e.error))?;
    Ok(())
}

fn copy_dir_recursive(source: &Path, dest: &Path) -> Result<()> {
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

fn compare_dir_trees(source: &Path, target: &Path) -> Result<bool> {
    let mut source_entries = HashSet::<OsString>::new();

    for entry in fs_err::read_dir(source).map_err(AgentStowError::from)? {
        let entry = entry.map_err(AgentStowError::from)?;
        let file_name = entry.file_name();
        source_entries.insert(file_name.clone());

        let source_path = entry.path();
        let target_path = target.join(&file_name);
        if !target_path.exists() {
            return Ok(false);
        }

        let file_type = entry.file_type().map_err(AgentStowError::from)?;
        if file_type.is_dir() {
            if !target_path.is_dir() || !compare_dir_trees(&source_path, &target_path)? {
                return Ok(false);
            }
        } else if file_type.is_file() {
            if !target_path.is_file() {
                return Ok(false);
            }
            let source_bytes = fs_err::read(&source_path).map_err(AgentStowError::from)?;
            let target_bytes = fs_err::read(&target_path).map_err(AgentStowError::from)?;
            if source_bytes != target_bytes {
                return Ok(false);
            }
        } else {
            return Ok(false);
        }
    }

    for entry in fs_err::read_dir(target).map_err(AgentStowError::from)? {
        let entry = entry.map_err(AgentStowError::from)?;
        if !source_entries.contains(&entry.file_name()) {
            return Ok(false);
        }
    }

    Ok(true)
}

#[cfg(test)]
mod tests;
