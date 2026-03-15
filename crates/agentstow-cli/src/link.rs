use agentstow_core::{
    AgentStowDirs, AgentStowError, InstallMethod, ProfileName, Result, TargetName,
    normalize_for_display,
};
use agentstow_linker::{ApplyOptions, InstallSource, LinkJob, RenderStore, apply_job, plan_job};
use agentstow_manifest::Manifest;
use agentstow_render::Renderer;
use agentstow_state::{LinkInstanceRecord, StateDb};
use agentstow_validate::Validator;
use serde::Serialize;
use time::OffsetDateTime;

use crate::bootstrap::CommandContext;
use crate::cli::{LinkArgs, LinkSubcommand};
use crate::output::print_json;

pub(crate) async fn link_command(
    ctx: &CommandContext,
    manifest: &Manifest,
    args: LinkArgs,
) -> Result<()> {
    let default_profile = ctx.default_profile_unchecked();

    match args.cmd {
        Some(LinkSubcommand::Status) => link_status(manifest, ctx.json()),
        Some(LinkSubcommand::Repair { targets, force }) => {
            link_repair(
                manifest,
                ctx.json(),
                default_profile.as_ref(),
                &targets,
                force,
            )
            .await
        }
        None => {
            link_apply_or_plan(
                manifest,
                ctx.json(),
                default_profile.as_ref(),
                &args.targets,
                args.force,
                args.plan,
            )
            .await
        }
    }
}

async fn link_apply_or_plan(
    manifest: &Manifest,
    json: bool,
    default_profile: Option<&ProfileName>,
    only_targets: &[String],
    force: bool,
    plan_only: bool,
) -> Result<()> {
    let dirs = AgentStowDirs::from_env()?;
    let render_store = RenderStore::new(dirs.cache_dir.join("agentstow"), &manifest.workspace_root);

    let mut plan_items = Vec::new();
    let mut applied_items = Vec::new();

    let selected: Vec<(&TargetName, &agentstow_manifest::TargetDef)> = if only_targets.is_empty() {
        manifest.targets.iter().collect()
    } else {
        only_targets
            .iter()
            .map(|target| {
                let name = TargetName::parse(target.clone())?;
                let (key, def) = manifest.targets.get_key_value(&name).ok_or_else(|| {
                    AgentStowError::Manifest {
                        message: format!("target 不存在：{target}").into(),
                    }
                })?;
                Ok((key, def))
            })
            .collect::<Result<Vec<_>>>()?
    };

    for (target_name, target) in selected {
        let profile = target
            .profile
            .clone()
            .or_else(|| default_profile.cloned())
            .ok_or_else(|| AgentStowError::InvalidArgs {
                message: format!(
                    "target 未配置 profile，且未指定全局 --profile: {}",
                    target_name.as_str()
                )
                .into(),
            })?;

        let job = build_link_job(manifest, target_name, target, &profile)?;

        let item = plan_job(&job, &render_store)?;
        plan_items.push(item);

        if !plan_only {
            let applied = apply_job(&job, &render_store, ApplyOptions { force })?;
            applied_items.push(applied.clone());
            record_link_instance(manifest, &dirs, &job, &render_store)?;
        }
    }

    if json {
        if plan_only {
            print_json(&plan_items)?;
        } else {
            print_json(&applied_items)?;
        }
        return Ok(());
    }

    for item in if plan_only {
        &plan_items
    } else {
        &applied_items
    } {
        println!(
            "- {} -> {} ({:?})",
            item.target.as_str(),
            normalize_for_display(&item.target_path),
            item.method
        );
    }
    Ok(())
}

fn build_link_job(
    manifest: &Manifest,
    target_name: &TargetName,
    target: &agentstow_manifest::TargetDef,
    profile: &ProfileName,
) -> Result<LinkJob> {
    let artifact = manifest.artifacts.get(&target.artifact).unwrap();
    let target_path = target.absolute_target_path(&manifest.workspace_root);

    let desired = match artifact.kind {
        agentstow_core::ArtifactKind::File => {
            let rendered = Renderer::render_file(manifest, &target.artifact, profile)?;
            Validator::validate_rendered_file(artifact, &rendered.bytes)?;
            InstallSource::FileBytes(rendered.bytes)
        }
        agentstow_core::ArtifactKind::Dir => {
            InstallSource::Path(artifact.source_path(&manifest.workspace_root))
        }
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

fn record_link_instance(
    manifest: &Manifest,
    dirs: &AgentStowDirs,
    job: &LinkJob,
    store: &RenderStore,
) -> Result<()> {
    let db = StateDb::open(dirs)?;
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

#[derive(Debug, Serialize)]
struct LinkStatusItem {
    target_path: String,
    method: InstallMethod,
    ok: bool,
    message: String,
}

fn link_status(manifest: &Manifest, json: bool) -> Result<()> {
    let dirs = AgentStowDirs::from_env()?;
    let db = StateDb::open(&dirs)?;
    let records = db.list_link_instances(&manifest.workspace_root)?;

    let mut output = Vec::new();
    for record in records {
        let ok =
            match record.method {
                InstallMethod::Symlink => match record.rendered_path.as_ref() {
                    Some(source) => agentstow_linker::check_symlink(&record.target_path, source)
                        .unwrap_or(false),
                    None => false,
                },
                InstallMethod::Junction => match record.rendered_path.as_ref() {
                    Some(source) => agentstow_linker::check_junction(&record.target_path, source)
                        .unwrap_or(false),
                    None => false,
                },
                InstallMethod::Copy => check_copy_target_health(manifest, &record)?,
            };
        output.push(LinkStatusItem {
            target_path: normalize_for_display(&record.target_path),
            method: record.method,
            ok,
            message: if ok {
                "healthy".to_string()
            } else {
                "unhealthy".to_string()
            },
        });
    }

    if json {
        print_json(&output)?;
        return Ok(());
    }

    for item in &output {
        let tag = if item.ok { "ok" } else { "bad" };
        println!("[{tag}] {} ({:?})", item.target_path, item.method);
    }
    Ok(())
}

fn check_copy_target_health(manifest: &Manifest, record: &LinkInstanceRecord) -> Result<bool> {
    let Some(artifact_def) = manifest.artifacts.get(&record.artifact_id) else {
        return Ok(false);
    };

    match artifact_def.kind {
        agentstow_core::ArtifactKind::File => {
            if !record.target_path.is_file() {
                return Ok(false);
            }
            let existing = fs_err::read(&record.target_path).map_err(AgentStowError::from)?;
            let desired = Renderer::render_file(manifest, &record.artifact_id, &record.profile)?;
            Ok(existing == desired.bytes)
        }
        agentstow_core::ArtifactKind::Dir => {
            let source_dir = artifact_def.source_path(&manifest.workspace_root);
            agentstow_linker::check_copy_dir(&record.target_path, &source_dir)
        }
    }
}

async fn link_repair(
    manifest: &Manifest,
    json: bool,
    default_profile: Option<&ProfileName>,
    only_targets: &[String],
    force: bool,
) -> Result<()> {
    link_apply_or_plan(manifest, json, default_profile, only_targets, force, false).await
}
