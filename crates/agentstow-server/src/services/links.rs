use time::OffsetDateTime;

use agentstow_core::{
    AgentStowDirs, AgentStowError, ArtifactKind, InstallMethod, ProfileName, Result, TargetName,
    normalize_for_display,
};
use agentstow_linker::{
    ApplyOptions, DesiredInstall, InstallSource, LinkJob, LinkPlanItem, RenderStore, apply_job,
    plan_job,
};
use agentstow_manifest::{Manifest, TargetDef};
use agentstow_render::Renderer;
use agentstow_state::{LinkInstanceRecord, StateDb};
use agentstow_validate::Validator;
use agentstow_web_types::{
    LinkApplyRequest, LinkDesiredInstallResponse, LinkOperationActionResponse,
    LinkOperationItemResponse, LinkOperationResponse, LinkPlanItemResponse, LinkPlanRequest,
    LinkRecordResponse, LinkRepairRequest, LinkStatusResponseItem,
};

use super::WorkspaceQueryService;
use super::common::{artifact_kind_response, install_method_response};

impl WorkspaceQueryService {
    pub(crate) fn link_records(&self) -> Result<Vec<LinkRecordResponse>> {
        let db = self.open_state_db()?;
        let records = db.list_link_instances(&self.workspace_root)?;
        Ok(records.into_iter().map(link_record_response).collect())
    }

    pub(crate) fn link_status(&self) -> Result<Vec<LinkStatusResponseItem>> {
        let manifest = self.load_manifest()?;
        self.compute_link_status(&manifest)
    }

    pub(crate) fn link_plan(&self, req: LinkPlanRequest) -> Result<LinkOperationResponse> {
        let manifest = self.load_manifest()?;
        let dirs = AgentStowDirs::from_env()?;
        let render_store = render_store(&dirs, &manifest);
        let default_profile = parse_default_profile(req.default_profile)?;
        let selected = select_manifest_targets(&manifest, &req.targets)?;

        let mut items = Vec::new();
        for (target_name, target) in selected {
            let profile = resolve_target_profile(target_name, target, default_profile.as_ref())?;
            let job = build_link_job(&manifest, target_name, target, &profile)?;
            let planned = plan_job(&job, &render_store)?;
            items.push(LinkOperationItemResponse {
                action: LinkOperationActionResponse::Planned,
                item: link_plan_item_response(&planned),
                message: None,
            });
        }

        Ok(LinkOperationResponse { items })
    }

    pub(crate) fn link_apply(&self, req: LinkApplyRequest) -> Result<LinkOperationResponse> {
        let manifest = self.load_manifest()?;
        let dirs = AgentStowDirs::from_env()?;
        let render_store = render_store(&dirs, &manifest);
        let db = StateDb::open(&dirs)?;
        let default_profile = parse_default_profile(req.default_profile)?;
        let selected = select_manifest_targets(&manifest, &req.targets)?;

        let mut items = Vec::new();
        for (target_name, target) in selected {
            let profile = resolve_target_profile(target_name, target, default_profile.as_ref())?;
            let job = build_link_job(&manifest, target_name, target, &profile)?;
            let healthy_before = link_job_is_healthy(&job, &render_store).unwrap_or(false);
            let applied = apply_job(&job, &render_store, ApplyOptions { force: req.force })?;
            record_link_instance(&db, &manifest, &job, &render_store)?;

            items.push(LinkOperationItemResponse {
                action: if healthy_before {
                    LinkOperationActionResponse::Skipped
                } else {
                    LinkOperationActionResponse::Applied
                },
                item: link_plan_item_response(&applied),
                message: Some(if healthy_before {
                    "already_healthy".to_string()
                } else {
                    "applied".to_string()
                }),
            });
        }

        Ok(LinkOperationResponse { items })
    }

    pub(crate) fn link_repair(&self, req: LinkRepairRequest) -> Result<LinkOperationResponse> {
        let manifest = self.load_manifest()?;
        let dirs = AgentStowDirs::from_env()?;
        let render_store = render_store(&dirs, &manifest);
        let db = StateDb::open(&dirs)?;
        let default_profile = parse_default_profile(req.default_profile)?;
        let selected = select_manifest_targets(&manifest, &req.targets)?;

        let mut items = Vec::new();
        for (target_name, target) in selected {
            let profile = resolve_target_profile(target_name, target, default_profile.as_ref())?;
            let job = build_link_job(&manifest, target_name, target, &profile)?;
            let planned = plan_job(&job, &render_store)?;
            let healthy_before = link_job_is_healthy(&job, &render_store).unwrap_or(false);

            if healthy_before {
                items.push(LinkOperationItemResponse {
                    action: LinkOperationActionResponse::Skipped,
                    item: link_plan_item_response(&planned),
                    message: Some("already_healthy".to_string()),
                });
                continue;
            }

            let applied = apply_job(&job, &render_store, ApplyOptions { force: req.force })?;
            record_link_instance(&db, &manifest, &job, &render_store)?;

            items.push(LinkOperationItemResponse {
                action: LinkOperationActionResponse::Repaired,
                item: link_plan_item_response(&applied),
                message: Some("repaired".to_string()),
            });
        }

        Ok(LinkOperationResponse { items })
    }
}

fn parse_default_profile(default_profile: Option<String>) -> Result<Option<ProfileName>> {
    default_profile.map(ProfileName::parse).transpose()
}

fn resolve_target_profile(
    target_name: &TargetName,
    target: &TargetDef,
    default_profile: Option<&ProfileName>,
) -> Result<ProfileName> {
    target
        .profile
        .clone()
        .or_else(|| default_profile.cloned())
        .ok_or_else(|| AgentStowError::InvalidArgs {
            message: format!(
                "target 未配置 profile，且未提供 default_profile: {}",
                target_name.as_str()
            )
            .into(),
        })
}

fn render_store(dirs: &AgentStowDirs, manifest: &Manifest) -> RenderStore {
    RenderStore::new(dirs.cache_dir.join("agentstow"), &manifest.workspace_root)
}

fn select_manifest_targets<'a>(
    manifest: &'a Manifest,
    only_targets: &[String],
) -> Result<Vec<(&'a TargetName, &'a TargetDef)>> {
    if only_targets.is_empty() {
        return Ok(manifest.targets.iter().collect());
    }

    only_targets
        .iter()
        .map(|target| {
            let name = TargetName::parse(target.clone())?;
            let (target_name, def) =
                manifest
                    .targets
                    .get_key_value(&name)
                    .ok_or_else(|| AgentStowError::Manifest {
                        message: format!("target 不存在：{target}").into(),
                    })?;
            Ok((target_name, def))
        })
        .collect()
}

fn build_link_job(
    manifest: &Manifest,
    target_name: &TargetName,
    target: &TargetDef,
    profile: &ProfileName,
) -> Result<LinkJob> {
    let artifact =
        manifest
            .artifacts
            .get(&target.artifact)
            .ok_or_else(|| AgentStowError::Manifest {
                message: format!("artifact 不存在：{}", target.artifact.as_str()).into(),
            })?;
    let target_path = target.absolute_target_path(&manifest.workspace_root);

    let desired = match artifact.kind {
        ArtifactKind::File => {
            let rendered = Renderer::render_file(manifest, &target.artifact, profile)?;
            Validator::validate_rendered_file(artifact, &rendered.bytes)?;
            InstallSource::FileBytes(rendered.bytes)
        }
        ArtifactKind::Dir => InstallSource::Path(artifact.source_path(&manifest.workspace_root)),
    };

    Ok(LinkJob {
        target: target_name.clone(),
        artifact_id: target.artifact.clone(),
        profile: profile.clone(),
        artifact_kind: artifact.kind,
        method: target.method,
        target_path,
        desired,
    })
}

fn link_job_is_healthy(job: &LinkJob, render_store: &RenderStore) -> Result<bool> {
    match job.method {
        InstallMethod::Symlink => match (&job.artifact_kind, &job.desired) {
            (ArtifactKind::File, InstallSource::FileBytes(_)) => {
                let desired_source =
                    render_store.rendered_file_path(&job.artifact_id, &job.profile);
                if !desired_source.is_file() {
                    return Ok(false);
                }
                agentstow_linker::check_symlink(&job.target_path, &desired_source)
            }
            (ArtifactKind::Dir, InstallSource::Path(path)) => {
                if !path.exists() {
                    return Ok(false);
                }
                agentstow_linker::check_symlink(&job.target_path, path)
            }
            _ => Ok(false),
        },
        InstallMethod::Junction => match (&job.artifact_kind, &job.desired) {
            (ArtifactKind::Dir, InstallSource::Path(path)) => {
                agentstow_linker::check_junction(&job.target_path, path)
            }
            _ => Ok(false),
        },
        InstallMethod::Copy => match (&job.artifact_kind, &job.desired) {
            (ArtifactKind::File, InstallSource::FileBytes(bytes)) => {
                if !job.target_path.is_file() {
                    return Ok(false);
                }
                let existing = fs_err::read(&job.target_path).map_err(AgentStowError::from)?;
                Ok(existing == *bytes)
            }
            (ArtifactKind::Dir, InstallSource::Path(path)) => {
                agentstow_linker::check_copy_dir(&job.target_path, path)
            }
            _ => Ok(false),
        },
    }
}

fn record_link_instance(
    db: &StateDb,
    manifest: &Manifest,
    job: &LinkJob,
    store: &RenderStore,
) -> Result<()> {
    let (rendered_path, blake3) = match (&job.method, &job.desired) {
        (InstallMethod::Symlink, InstallSource::FileBytes(bytes)) => (
            Some(store.rendered_file_path(&job.artifact_id, &job.profile)),
            Some(blake3::hash(bytes).to_hex().to_string()),
        ),
        (InstallMethod::Copy, InstallSource::FileBytes(bytes)) => {
            (None, Some(blake3::hash(bytes).to_hex().to_string()))
        }
        (InstallMethod::Symlink, InstallSource::Path(path))
        | (InstallMethod::Junction, InstallSource::Path(path)) => (Some(path.clone()), None),
        (InstallMethod::Copy, InstallSource::Path(_)) => (None, None),
        _ => (None, None),
    };

    let record = LinkInstanceRecord {
        workspace_root: manifest.workspace_root.clone(),
        artifact_id: job.artifact_id.clone(),
        profile: job.profile.clone(),
        target_path: job.target_path.clone(),
        method: job.method,
        rendered_path,
        blake3,
        updated_at: OffsetDateTime::now_utc(),
    };
    db.upsert_link_instance(&record)?;
    Ok(())
}

fn link_plan_item_response(planned: &LinkPlanItem) -> LinkPlanItemResponse {
    LinkPlanItemResponse {
        target: planned.target.as_str().to_string(),
        artifact_id: planned.artifact_id.as_str().to_string(),
        profile: planned.profile.as_str().to_string(),
        artifact_kind: artifact_kind_response(planned.artifact_kind),
        method: install_method_response(planned.method),
        target_path: normalize_for_display(&planned.target_path),
        desired: match &planned.desired {
            DesiredInstall::Symlink { source_path } => LinkDesiredInstallResponse::Symlink {
                source_path: normalize_for_display(source_path),
            },
            DesiredInstall::Junction { source_path } => LinkDesiredInstallResponse::Junction {
                source_path: normalize_for_display(source_path),
            },
            DesiredInstall::Copy { blake3, bytes_len } => LinkDesiredInstallResponse::Copy {
                blake3: blake3.clone(),
                bytes_len: *bytes_len,
            },
        },
    }
}

fn link_record_response(record: LinkInstanceRecord) -> LinkRecordResponse {
    LinkRecordResponse {
        artifact_id: record.artifact_id.as_str().to_string(),
        profile: record.profile.as_str().to_string(),
        target_path: normalize_for_display(&record.target_path),
        method: install_method_response(record.method),
        rendered_path: record
            .rendered_path
            .as_ref()
            .map(|path| normalize_for_display(path)),
        blake3: record.blake3,
        updated_at: record
            .updated_at
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default(),
    }
}

pub(crate) fn link_status_item(
    manifest: &Manifest,
    record: &LinkInstanceRecord,
) -> LinkStatusResponseItem {
    let target_path = normalize_for_display(&record.target_path);
    let method = install_method_response(record.method);
    let artifact_id = record.artifact_id.as_str().to_string();
    let profile = record.profile.as_str().to_string();

    let Some(artifact_def) = manifest.artifacts.get(&record.artifact_id) else {
        return LinkStatusResponseItem {
            artifact_id,
            profile,
            target_path,
            method,
            ok: false,
            message: "artifact_missing".to_string(),
        };
    };

    let status = match record.method {
        InstallMethod::Symlink => match record.rendered_path.as_ref() {
            Some(source_path) => {
                agentstow_linker::check_symlink(&record.target_path, source_path).unwrap_or(false)
            }
            None => false,
        },
        InstallMethod::Junction => match record.rendered_path.as_ref() {
            Some(source_path) => {
                agentstow_linker::check_junction(&record.target_path, source_path).unwrap_or(false)
            }
            None => false,
        },
        InstallMethod::Copy => match artifact_def.kind {
            ArtifactKind::File => {
                if !record.target_path.is_file() {
                    false
                } else {
                    match fs_err::read(&record.target_path).map_err(AgentStowError::from) {
                        Ok(existing_bytes) => match Renderer::render_file(
                            manifest,
                            &record.artifact_id,
                            &record.profile,
                        ) {
                            Ok(rendered) => existing_bytes == rendered.bytes,
                            Err(_) => false,
                        },
                        Err(_) => false,
                    }
                }
            }
            ArtifactKind::Dir => {
                let desired_source = artifact_def.source_path(&manifest.workspace_root);
                agentstow_linker::check_copy_dir(&record.target_path, &desired_source)
                    .unwrap_or(false)
            }
        },
    };

    LinkStatusResponseItem {
        artifact_id,
        profile,
        target_path,
        method,
        ok: status,
        message: if status {
            "healthy".to_string()
        } else {
            "unhealthy".to_string()
        },
    }
}
