use agentstow_core::{AgentStowError, ArtifactKind, InstallMethod, Result, normalize_for_display};

use crate::health::{check_copy_dir, check_junction, is_target_symlink_path_match};
use crate::types::{ApplyOptions, InstallSource, LinkJob, RenderStore};

pub fn preflight_job(job: &LinkJob, render_store: &RenderStore, opt: ApplyOptions) -> Result<()> {
    match job.method {
        InstallMethod::Symlink => preflight_symlink(job, render_store, opt),
        InstallMethod::Junction => preflight_junction(job, opt),
        InstallMethod::Copy => preflight_copy(job, opt),
    }
}

fn preflight_symlink(job: &LinkJob, render_store: &RenderStore, opt: ApplyOptions) -> Result<()> {
    let source_path = match (&job.artifact_kind, &job.desired) {
        (ArtifactKind::File, InstallSource::FileBytes(_)) => {
            render_store.rendered_file_path(&job.artifact_id, &job.profile)
        }
        (ArtifactKind::Dir, InstallSource::Path(path)) => path.clone(),
        (kind, _) => {
            return Err(AgentStowError::Link {
                message: format!("symlink 不支持的 artifact kind/source 组合：{kind:?}").into(),
            });
        }
    };

    if job.target_path.exists()
        && !is_target_symlink_path_match(&job.target_path, &source_path)?
        && !opt.force
    {
        return Err(AgentStowError::LinkConflict {
            message: format!(
                "target 已存在且不是期望的 symlink: {}",
                normalize_for_display(&job.target_path)
            )
            .into(),
        });
    }

    Ok(())
}

fn preflight_junction(job: &LinkJob, opt: ApplyOptions) -> Result<()> {
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

    if job.target_path.exists() && !check_junction(&job.target_path, source_path)? && !opt.force {
        return Err(AgentStowError::LinkConflict {
            message: format!(
                "target 已存在且不是期望的 junction: {}",
                normalize_for_display(&job.target_path)
            )
            .into(),
        });
    }

    Ok(())
}

fn preflight_copy(job: &LinkJob, opt: ApplyOptions) -> Result<()> {
    match (&job.artifact_kind, &job.desired) {
        (ArtifactKind::File, InstallSource::FileBytes(bytes)) => {
            if job.target_path.is_file() {
                let existing =
                    fs_err::read(&job.target_path).map_err(agentstow_core::AgentStowError::from)?;
                if blake3::hash(&existing) != blake3::hash(bytes) && !opt.force {
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
            }
        }
        (ArtifactKind::Dir, InstallSource::Path(source_dir)) => {
            if !source_dir.is_dir() {
                return Err(AgentStowError::Link {
                    message: format!("source 不是目录：{}", normalize_for_display(source_dir))
                        .into(),
                });
            }
            if job.target_path.exists()
                && !check_copy_dir(&job.target_path, source_dir)?
                && !opt.force
            {
                return Err(AgentStowError::LinkConflict {
                    message: format!("target 已存在：{}", normalize_for_display(&job.target_path))
                        .into(),
                });
            }
        }
        _ => {
            return Err(AgentStowError::Link {
                message: "copy 不支持的 artifact kind/source 组合".into(),
            });
        }
    }

    Ok(())
}
