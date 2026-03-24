use std::path::Path;

use agentstow_core::{
    AgentStowError, ArtifactKind, InstallMethod, Result, ensure_parent_dir, normalize_for_display,
};
use tracing::{info, instrument};

use crate::fsops::{
    atomic_write_file, copy_dir_recursive, create_symlink, create_unused_path,
    relative_or_absolute_link_target, remove_existing, rename_into_place,
};
use crate::health::{check_copy_dir, check_junction, is_correct_symlink};
use crate::types::{
    ApplyOptions, DesiredInstall, InstallSource, LinkJob, LinkPlanItem, RenderStore,
};

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
                    message: format!("symlink 不支持的 artifact kind/source 组合：{kind:?}").into(),
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
                message: format!("symlink 不支持的 artifact kind/source 组合：{kind:?}").into(),
            });
        }
    };

    ensure_parent_dir(&job.target_path)?;

    if job.target_path.exists() {
        if is_correct_symlink(&job.target_path, &source_path)? && !opt.force {
            info!(
                "target 已是正确 symlink，跳过：{}",
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
        let _ = opt;
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
                let existing =
                    fs_err::read(&job.target_path).map_err(agentstow_core::AgentStowError::from)?;
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
                            "target 已存在且内容不同：{}",
                            normalize_for_display(&job.target_path)
                        )
                        .into(),
                    });
                }
            } else if job.target_path.exists() && !opt.force {
                return Err(AgentStowError::LinkConflict {
                    message: format!(
                        "target 已存在且不是文件：{}",
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
                    message: format!("source 不是目录：{}", normalize_for_display(source_dir))
                        .into(),
                });
            }
            if job.target_path.exists() {
                if check_copy_dir(&job.target_path, source_dir)? {
                    return Ok(LinkPlanItem {
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
                    });
                }
                if !opt.force {
                    return Err(AgentStowError::LinkConflict {
                        message: format!(
                            "target 已存在：{}",
                            normalize_for_display(&job.target_path)
                        )
                        .into(),
                    });
                }
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
