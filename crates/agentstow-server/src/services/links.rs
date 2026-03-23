use std::collections::HashSet;

use time::OffsetDateTime;

use agentstow_core::{
    AgentStowDirs, AgentStowError, ProfileName, Result, TargetName, normalize_for_display,
};
use agentstow_linker::{
    ApplyOptions, DesiredInstall, LinkPlanItem, RenderStore, apply_job, build_link_instance_record,
    build_link_job_from_manifest, check_link_job_health, check_link_record_health, plan_job,
    preflight_job,
};
use agentstow_manifest::{Manifest, TargetDef};
use agentstow_state::{LinkInstanceRecord, StateDb};
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
            let job = build_link_job_from_manifest(
                &manifest,
                target_name,
                target,
                &profile,
                &render_store,
            )?;
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

        let mut jobs = Vec::new();
        for (target_name, target) in selected {
            let profile = resolve_target_profile(target_name, target, default_profile.as_ref())?;
            let job = build_link_job_from_manifest(
                &manifest,
                target_name,
                target,
                &profile,
                &render_store,
            )?;
            let healthy_before = check_link_job_health(&job, &render_store).unwrap_or(false);
            jobs.push((job, healthy_before));
        }

        let apply_options = ApplyOptions { force: req.force };
        for (job, _) in &jobs {
            preflight_job(job, &render_store, apply_options)?;
        }

        let mut items = Vec::new();
        for (job, healthy_before) in jobs {
            let applied = apply_job(&job, &render_store, apply_options)?;
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

        prune_removed_link_records(&db, &manifest)?;

        Ok(LinkOperationResponse { items })
    }

    pub(crate) fn link_repair(&self, req: LinkRepairRequest) -> Result<LinkOperationResponse> {
        let manifest = self.load_manifest()?;
        let dirs = AgentStowDirs::from_env()?;
        let render_store = render_store(&dirs, &manifest);
        let db = StateDb::open(&dirs)?;
        let default_profile = parse_default_profile(req.default_profile)?;
        let selected = select_manifest_targets(&manifest, &req.targets)?;

        let mut pending = Vec::new();
        let mut items = Vec::new();
        for (target_name, target) in selected {
            let profile = resolve_target_profile(target_name, target, default_profile.as_ref())?;
            let job = build_link_job_from_manifest(
                &manifest,
                target_name,
                target,
                &profile,
                &render_store,
            )?;
            let planned = plan_job(&job, &render_store)?;
            let healthy_before = check_link_job_health(&job, &render_store).unwrap_or(false);

            if healthy_before {
                items.push(LinkOperationItemResponse {
                    action: LinkOperationActionResponse::Skipped,
                    item: link_plan_item_response(&planned),
                    message: Some("already_healthy".to_string()),
                });
                continue;
            }
            pending.push(job);
        }

        let apply_options = ApplyOptions { force: req.force };
        for job in &pending {
            preflight_job(job, &render_store, apply_options)?;
        }

        for job in pending {
            let applied = apply_job(&job, &render_store, apply_options)?;
            record_link_instance(&db, &manifest, &job, &render_store)?;

            items.push(LinkOperationItemResponse {
                action: LinkOperationActionResponse::Repaired,
                item: link_plan_item_response(&applied),
                message: Some("repaired".to_string()),
            });
        }

        prune_removed_link_records(&db, &manifest)?;

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

fn record_link_instance(
    db: &StateDb,
    manifest: &Manifest,
    job: &agentstow_linker::LinkJob,
    store: &RenderStore,
) -> Result<()> {
    let record = build_link_instance_record(manifest, job, store, OffsetDateTime::now_utc())?;
    db.upsert_link_instance(&record)?;
    Ok(())
}

fn prune_removed_link_records(db: &StateDb, manifest: &Manifest) -> Result<()> {
    let keep_target_paths: HashSet<_> = manifest
        .targets
        .values()
        .map(|target| target.absolute_target_path(&manifest.workspace_root))
        .collect();
    db.prune_link_instances_not_in(&manifest.workspace_root, &keep_target_paths)?;
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

    if !manifest.artifacts.contains_key(&record.artifact_id) {
        return LinkStatusResponseItem {
            artifact_id,
            profile,
            target_path,
            method,
            ok: false,
            message: "artifact_missing".to_string(),
        };
    }
    let status = check_link_record_health(manifest, record).unwrap_or(false);

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
