use agentstow_core::{
    AgentStowDirs, AgentStowError, InstallMethod, ProfileName, Result, TargetName,
    normalize_for_display,
};
use agentstow_linker::{
    ApplyOptions, RenderStore, apply_job, build_link_instance_record, build_link_job_from_manifest,
    check_link_record_health, plan_job, preflight_job,
};
use agentstow_manifest::Manifest;
use agentstow_state::StateDb;
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

    let mut jobs = Vec::new();
    let mut plan_items = Vec::new();

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

        let job =
            build_link_job_from_manifest(manifest, target_name, target, &profile, &render_store)?;

        let item = plan_job(&job, &render_store)?;
        plan_items.push(item);
        jobs.push(job);
    }

    let mut applied_items = Vec::new();
    if !plan_only {
        let apply_options = ApplyOptions { force };
        for job in &jobs {
            preflight_job(job, &render_store, apply_options)?;
        }
        for job in &jobs {
            let applied = apply_job(job, &render_store, apply_options)?;
            applied_items.push(applied.clone());
            record_link_instance(manifest, &dirs, job, &render_store)?;
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

fn record_link_instance(
    manifest: &Manifest,
    dirs: &AgentStowDirs,
    job: &agentstow_linker::LinkJob,
    store: &RenderStore,
) -> Result<()> {
    let db = StateDb::open(dirs)?;
    let record = build_link_instance_record(manifest, job, store, OffsetDateTime::now_utc())?;
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
        let ok = check_link_record_health(manifest, &record).unwrap_or(false);
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

async fn link_repair(
    manifest: &Manifest,
    json: bool,
    default_profile: Option<&ProfileName>,
    only_targets: &[String],
    force: bool,
) -> Result<()> {
    link_apply_or_plan(manifest, json, default_profile, only_targets, force, false).await
}
