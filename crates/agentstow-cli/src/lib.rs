use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use agentstow_core::{
    AgentStowDirs, AgentStowError, ArtifactId, InstallMethod, ProfileName, Result, ShellKind,
    TargetName, normalize_for_display,
};
use agentstow_env::Env;
use agentstow_git::Git;
use agentstow_linker::{ApplyOptions, InstallSource, LinkJob, RenderStore, apply_job, plan_job};
use agentstow_manifest::Manifest;
use agentstow_mcp::Mcp;
use agentstow_render::Renderer;
use agentstow_scripts::{ScriptRunRequest, ScriptRunner};
use agentstow_state::{LinkInstanceRecord, StateDb};
use agentstow_validate::Validator;
use clap::{Args, Parser, Subcommand};
use serde::Serialize;
use time::OffsetDateTime;
use tracing::instrument;

#[derive(Debug, Parser)]
#[command(
    name = "agentstow",
    version,
    about = "Git-native source-of-truth manager for AI artifacts"
)]
pub struct Cli {
    /// 输出机器可读 JSON（用于测试/自动化）
    #[arg(long)]
    pub json: bool,

    /// 禁止交互式提示（用于 CI/自动化）
    #[arg(long = "non-interactive")]
    pub non_interactive: bool,

    /// 关闭彩色输出
    #[arg(long = "no-color")]
    pub no_color: bool,

    /// 先切换工作目录再执行
    #[arg(long)]
    pub cwd: Option<PathBuf>,

    /// 指定 workspace 根目录（默认从 cwd 向上寻找 `agentstow.toml`）
    #[arg(long)]
    pub workspace: Option<PathBuf>,

    /// 默认 profile（可被子命令覆盖）
    #[arg(long)]
    pub profile: Option<String>,

    /// 全局超时（毫秒），用于脚本执行等
    #[arg(long)]
    pub timeout: Option<u64>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Render(RenderArgs),
    Validate(ValidateArgs),
    Link(LinkArgs),
    Env(EnvArgs),
    Scripts(ScriptsArgs),
    Mcp(McpArgs),
    Workspace(WorkspaceArgs),
    Serve(ServeArgs),
}

#[derive(Debug, Args)]
pub struct RenderArgs {
    #[arg(long)]
    pub artifact: String,
    #[arg(long)]
    pub profile: Option<String>,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub out: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct ValidateArgs {
    #[arg(long)]
    pub artifact: String,
    #[arg(long)]
    pub profile: Option<String>,
}

#[derive(Debug, Args)]
pub struct LinkArgs {
    /// 仅输出 plan，不执行安装
    #[arg(long)]
    pub plan: bool,
    /// 覆盖冲突 target
    #[arg(long)]
    pub force: bool,
    /// 指定 target（可重复）；为空则对所有 targets 生效
    #[arg(long = "target")]
    pub targets: Vec<String>,
    #[command(subcommand)]
    pub cmd: Option<LinkSubcommand>,
}

#[derive(Debug, Subcommand)]
pub enum LinkSubcommand {
    Status,
    Repair {
        /// 仅修复指定 target（可重复）；为空则修复所有不健康项
        #[arg(long = "target")]
        targets: Vec<String>,
        #[arg(long)]
        force: bool,
    },
}

#[derive(Debug, Args)]
pub struct EnvArgs {
    #[command(subcommand)]
    pub cmd: EnvSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum EnvSubcommand {
    Emit {
        #[arg(long)]
        set: String,
        #[arg(long)]
        shell: ShellKind,
        #[arg(long)]
        stdout: bool,
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

#[derive(Debug, Args)]
pub struct ScriptsArgs {
    #[command(subcommand)]
    pub cmd: ScriptsSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ScriptsSubcommand {
    Run {
        #[arg(long)]
        id: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        stdin: Option<String>,
    },
}

#[derive(Debug, Args)]
pub struct McpArgs {
    #[command(subcommand)]
    pub cmd: McpSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum McpSubcommand {
    Validate,
    Render {
        #[arg(long)]
        stdout: bool,
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

#[derive(Debug, Args)]
pub struct WorkspaceArgs {
    #[command(subcommand)]
    pub cmd: WorkspaceSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum WorkspaceSubcommand {
    Status,
}

#[derive(Debug, Args)]
pub struct ServeArgs {
    #[arg(long, default_value = "127.0.0.1:8787")]
    pub addr: SocketAddr,
}

pub async fn run() -> i32 {
    let cli = Cli::parse();
    let json = cli.json;

    let _ = init_tracing();

    if let Some(cwd) = &cli.cwd
        && let Err(e) = std::env::set_current_dir(cwd)
    {
        let err = AgentStowError::from(e);
        emit_error(json, &err);
        return err.exit_code().as_i32();
    }

    match run_cli(cli).await {
        Ok(()) => 0,
        Err(e) => {
            emit_error(json, &e);
            e.exit_code().as_i32()
        }
    }
}

fn init_tracing() -> Result<()> {
    use tracing_subscriber::EnvFilter;
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .with_target(false)
        .finish();

    // 避免在 tests 中多次 init 导致 panic
    let _ = tracing::subscriber::set_global_default(subscriber);
    Ok(())
}

#[instrument(skip_all)]
async fn run_cli(cli: Cli) -> Result<()> {
    let cwd = std::env::current_dir().map_err(AgentStowError::from)?;

    let Cli {
        json,
        non_interactive: _,
        no_color: _,
        cwd: _,
        workspace,
        profile,
        timeout,
        command,
    } = cli;

    match command {
        Commands::Workspace(args) => match args.cmd {
            WorkspaceSubcommand::Status => {
                let info = Git::detect(&cwd).await?;
                if json {
                    print_json(&info)?;
                    return Ok(());
                }
                println!("Repo root: {}", normalize_for_display(&info.repo_root));
                println!("HEAD: {}", info.head);
                println!("Dirty: {}", info.dirty);
                Ok(())
            }
        },
        Commands::Serve(args) => {
            let workspace_root = resolve_workspace_root(&cwd, workspace.as_deref())?;
            agentstow_server::serve(agentstow_server::ServerConfig {
                workspace_root,
                addr: args.addr,
            })
            .await
        }
        Commands::Render(args) => {
            let manifest = load_manifest(&cwd, workspace.as_deref())?;
            let profile =
                resolve_profile(profile.as_deref(), args.profile.as_deref(), Some(&manifest))?;
            let artifact_id = ArtifactId::parse(args.artifact)?;
            let rendered = Renderer::render_file(&manifest, &artifact_id, &profile)?;
            let artifact_def = manifest.artifacts.get(&rendered.artifact_id).unwrap();
            Validator::validate_rendered_file(artifact_def, &rendered.bytes)?;

            if json {
                let out = serde_json::json!({
                  "artifact_id": rendered.artifact_id.as_str(),
                  "profile": rendered.profile.as_str(),
                  "text": String::from_utf8_lossy(&rendered.bytes),
                });
                println!("{}", serde_json::to_string_pretty(&out).unwrap());
                return Ok(());
            }

            if args.dry_run || args.out.is_none() {
                print!("{}", String::from_utf8_lossy(&rendered.bytes));
                return Ok(());
            }

            let out_path = args.out.unwrap();
            agentstow_core::ensure_parent_dir(&out_path)?;
            fs_err::write(&out_path, &rendered.bytes).map_err(AgentStowError::from)?;
            Ok(())
        }
        Commands::Validate(args) => {
            let manifest = load_manifest(&cwd, workspace.as_deref())?;
            let profile =
                resolve_profile(profile.as_deref(), args.profile.as_deref(), Some(&manifest))?;
            let artifact_id = ArtifactId::parse(args.artifact)?;
            let rendered = Renderer::render_file(&manifest, &artifact_id, &profile)?;
            let artifact_def = manifest.artifacts.get(&rendered.artifact_id).unwrap();
            Validator::validate_rendered_file(artifact_def, &rendered.bytes)?;
            if json {
                print_json(&serde_json::json!({ "ok": true }))?;
            } else {
                println!("ok");
            }
            Ok(())
        }
        Commands::Env(args) => {
            let manifest = load_manifest(&cwd, workspace.as_deref())?;
            match args.cmd {
                EnvSubcommand::Emit {
                    set,
                    shell,
                    stdout,
                    out,
                } => {
                    let env_set =
                        manifest
                            .env_sets
                            .get(&set)
                            .ok_or_else(|| AgentStowError::Manifest {
                                message: format!("env set 不存在: {set}").into(),
                            })?;
                    let vars = Env::resolve_env_set(env_set)?;
                    let script = Env::emit_shell(shell, &vars)?;

                    if json {
                        let out =
                            serde_json::json!({ "shell": format!("{shell:?}"), "script": script });
                        println!("{}", serde_json::to_string_pretty(&out).unwrap());
                        return Ok(());
                    }

                    if stdout || out.is_none() {
                        print!("{script}");
                        return Ok(());
                    }
                    let out_path = out.unwrap();
                    agentstow_core::ensure_parent_dir(&out_path)?;
                    fs_err::write(&out_path, script).map_err(AgentStowError::from)?;
                    Ok(())
                }
            }
        }
        Commands::Scripts(args) => {
            let manifest = load_manifest(&cwd, workspace.as_deref())?;
            match args.cmd {
                ScriptsSubcommand::Run { id, dry_run, stdin } => {
                    let mut script = manifest
                        .scripts
                        .get(&id)
                        .ok_or_else(|| AgentStowError::Manifest {
                            message: format!("script 不存在: {id}").into(),
                        })?
                        .clone();
                    if dry_run {
                        print_json(&script)?;
                        return Ok(());
                    }

                    if let Some(timeout_ms) = timeout {
                        script.timeout_ms = Some(timeout_ms);
                    }

                    let out = ScriptRunner::run(ScriptRunRequest {
                        workspace_root: manifest.workspace_root.clone(),
                        script,
                        stdin_text: stdin,
                    })
                    .await?;
                    if json {
                        print_json(&out)?;
                    } else {
                        if let Some(stdout) = &out.stdout {
                            print!("{stdout}");
                        }
                        if let Some(stderr) = &out.stderr {
                            eprint!("{stderr}");
                        }
                        println!("\n(exit={})", out.exit_code);
                    }
                    Ok(())
                }
            }
        }
        Commands::Mcp(args) => {
            let manifest = load_manifest(&cwd, workspace.as_deref())?;
            match args.cmd {
                McpSubcommand::Validate => {
                    for (name, server) in &manifest.mcp_servers {
                        Mcp::validate_server(name, server)?;
                    }
                    if json {
                        print_json(&serde_json::json!({ "ok": true }))?;
                    } else {
                        println!("ok");
                    }
                    Ok(())
                }
                McpSubcommand::Render { stdout, out } => {
                    let json = Mcp::render_mcp_json(&manifest.mcp_servers)?;
                    if stdout || out.is_none() {
                        println!("{json}");
                        return Ok(());
                    }
                    let out_path = out.unwrap();
                    agentstow_core::ensure_parent_dir(&out_path)?;
                    fs_err::write(&out_path, json).map_err(AgentStowError::from)?;
                    Ok(())
                }
            }
        }
        Commands::Link(args) => {
            let manifest = load_manifest(&cwd, workspace.as_deref())?;
            link_command(json, profile.as_deref(), &manifest, args).await
        }
    }
}

async fn link_command(
    json: bool,
    default_profile: Option<&str>,
    manifest: &Manifest,
    args: LinkArgs,
) -> Result<()> {
    let default_profile = default_profile.map(ProfileName::new_unchecked);

    match args.cmd {
        Some(LinkSubcommand::Status) => link_status(manifest, json).await,
        Some(LinkSubcommand::Repair { targets, force }) => {
            link_repair(manifest, json, default_profile.as_ref(), &targets, force).await
        }
        None => {
            link_apply_or_plan(
                manifest,
                json,
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
            .map(|t| {
                let name = TargetName::parse(t.clone())?;
                let (k, def) = manifest.targets.get_key_value(&name).ok_or_else(|| {
                    AgentStowError::Manifest {
                        message: format!("target 不存在: {t}").into(),
                    }
                })?;
                Ok((k, def))
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
            println!("{}", serde_json::to_string_pretty(&plan_items).unwrap());
        } else {
            println!("{}", serde_json::to_string_pretty(&applied_items).unwrap());
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
        (InstallMethod::Symlink, InstallSource::Path(p))
        | (InstallMethod::Junction, InstallSource::Path(p)) => (Some(p.clone()), None),
        (InstallMethod::Copy, InstallSource::Path(_)) => (None, None),
        _ => (None, None),
    };

    let rec = LinkInstanceRecord {
        workspace_root: manifest.workspace_root.clone(),
        artifact_id: job.artifact_id.clone(),
        profile: job.profile.clone(),
        target_path: job.target_path.clone(),
        method: job.method,
        rendered_path,
        blake3,
        updated_at: OffsetDateTime::now_utc(),
    };
    db.upsert_link_instance(&rec)?;
    Ok(())
}

#[derive(Debug, Serialize)]
struct LinkStatusItem {
    target_path: String,
    method: InstallMethod,
    ok: bool,
    message: String,
}

async fn link_status(manifest: &Manifest, json: bool) -> Result<()> {
    let dirs = AgentStowDirs::from_env()?;
    let db = StateDb::open(&dirs)?;
    let records = db.list_link_instances(&manifest.workspace_root)?;

    let mut out = Vec::new();
    for rec in records {
        let ok = match rec.method {
            InstallMethod::Symlink => match rec.rendered_path.as_ref() {
                Some(src) => {
                    agentstow_linker::check_symlink(&rec.target_path, src).unwrap_or(false)
                }
                None => false,
            },
            InstallMethod::Junction => match rec.rendered_path.as_ref() {
                Some(src) => {
                    agentstow_linker::check_junction(&rec.target_path, src).unwrap_or(false)
                }
                None => false,
            },
            InstallMethod::Copy => check_copy_target_health(manifest, &rec)?,
        };
        out.push(LinkStatusItem {
            target_path: normalize_for_display(&rec.target_path),
            method: rec.method,
            ok,
            message: if ok {
                "healthy".to_string()
            } else {
                "unhealthy".to_string()
            },
        });
    }

    if json {
        print_json(&out)?;
        return Ok(());
    }
    for item in &out {
        let tag = if item.ok { "ok" } else { "bad" };
        println!("[{tag}] {} ({:?})", item.target_path, item.method);
    }
    Ok(())
}

fn check_copy_target_health(manifest: &Manifest, rec: &LinkInstanceRecord) -> Result<bool> {
    let Some(artifact_def) = manifest.artifacts.get(&rec.artifact_id) else {
        return Ok(false);
    };

    match artifact_def.kind {
        agentstow_core::ArtifactKind::File => {
            if !rec.target_path.is_file() {
                return Ok(false);
            }
            let existing = fs_err::read(&rec.target_path).map_err(AgentStowError::from)?;
            let desired = Renderer::render_file(manifest, &rec.artifact_id, &rec.profile)?;
            Ok(existing == desired.bytes)
        }
        agentstow_core::ArtifactKind::Dir => {
            let source_dir = artifact_def.source_path(&manifest.workspace_root);
            agentstow_linker::check_copy_dir(&rec.target_path, &source_dir)
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
    // v1：简单策略：对指定 targets 直接重新 apply（force 可选）
    link_apply_or_plan(manifest, json, default_profile, only_targets, force, false).await
}

fn resolve_workspace_root(cwd: &Path, override_workspace: Option<&Path>) -> Result<PathBuf> {
    if let Some(p) = override_workspace {
        return Ok(p.to_path_buf());
    }
    let manifest_path = Manifest::find_from(cwd)?;
    Ok(manifest_path
        .parent()
        .ok_or_else(|| AgentStowError::Manifest {
            message: "manifest path 没有 parent".into(),
        })?
        .to_path_buf())
}

fn load_manifest(cwd: &Path, override_workspace: Option<&Path>) -> Result<Manifest> {
    let ws = resolve_workspace_root(cwd, override_workspace)?;
    Manifest::load_from_dir(&ws)
}

fn resolve_profile(
    default_profile: Option<&str>,
    override_profile: Option<&str>,
    manifest: Option<&Manifest>,
) -> Result<ProfileName> {
    let chosen = override_profile
        .map(str::to_string)
        .or_else(|| default_profile.map(str::to_string))
        .ok_or_else(|| AgentStowError::InvalidArgs {
            message: "需要指定 --profile（或在 target 内配置 profile）".into(),
        })?;
    let p = ProfileName::parse(chosen)?;
    if let Some(m) = manifest
        && !m.profiles.contains_key(&p)
    {
        return Err(AgentStowError::Manifest {
            message: format!("profile 不存在: {}", p.as_str()).into(),
        });
    }
    Ok(p)
}

fn print_json<T: Serialize>(v: &T) -> Result<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(v).map_err(|e| AgentStowError::Other(e.into()))?
    );
    Ok(())
}

fn emit_error(json: bool, err: &AgentStowError) {
    if json {
        let payload = serde_json::json!({
            "error": err.to_string(),
            "exit_code": err.exit_code().as_i32(),
        });
        if print_json(&payload).is_ok() {
            return;
        }
    }

    eprintln!("{err}");
}

#[cfg(test)]
mod tests;
