use agentstow_core::{AgentStowError, ArtifactId, ArtifactKind, Result};
use agentstow_env::Env;
use agentstow_mcp::Mcp;
use agentstow_render::{RenderedDirEntryKind, Renderer};
use agentstow_scripts::{ScriptRunRequest, ScriptRunner};
use agentstow_validate::Validator;
use tracing::instrument;

use crate::bootstrap::CommandContext;
use crate::cli::{
    Commands, EnvArgs, EnvSubcommand, McpArgs, McpSubcommand, RenderArgs, ScriptsArgs,
    ScriptsSubcommand, ServeArgs, ValidateArgs,
};
use crate::link;
use crate::output::{print_json, write_bytes_file, write_rendered_dir, write_text_file};
use crate::workspace;

#[instrument(skip_all)]
pub(crate) async fn run_cli(command: Commands, ctx: &CommandContext) -> Result<()> {
    match command {
        Commands::Workspace(args) => workspace::workspace_command(ctx, args).await,
        Commands::Serve(args) => serve_command(ctx, args).await,
        Commands::Render(args) => render_command(ctx, args),
        Commands::Validate(args) => validate_command(ctx, args),
        Commands::Env(args) => env_command(ctx, args),
        Commands::Scripts(args) => scripts_command(ctx, args).await,
        Commands::Mcp(args) => mcp_command(ctx, args),
        Commands::Link(args) => {
            let manifest = ctx.load_manifest()?;
            link::link_command(ctx, &manifest, args).await
        }
    }
}

async fn serve_command(ctx: &CommandContext, args: ServeArgs) -> Result<()> {
    let workspace_root = ctx.resolve_workspace_root_optional()?;
    agentstow_server::serve(agentstow_server::ServerConfig {
        workspace_root,
        addr: args.addr,
    })
    .await
}

fn render_command(ctx: &CommandContext, args: RenderArgs) -> Result<()> {
    let manifest = ctx.load_manifest()?;
    let profile = ctx.resolve_profile(args.profile.as_deref(), Some(&manifest))?;
    let artifact_id = ArtifactId::parse(args.artifact)?;
    let artifact_def =
        manifest
            .artifacts
            .get(&artifact_id)
            .ok_or_else(|| AgentStowError::Manifest {
                message: format!("artifact 不存在：{}", artifact_id.as_str()).into(),
            })?;

    match artifact_def.kind {
        ArtifactKind::File => {
            let rendered = Renderer::render_file(&manifest, &artifact_id, &profile)?;
            Validator::validate_rendered_file(artifact_def, &rendered.bytes)?;

            if ctx.json() {
                print_json(&serde_json::json!({
                    "artifact_id": rendered.artifact_id.as_str(),
                    "profile": rendered.profile.as_str(),
                    "kind": "file",
                    "text": String::from_utf8_lossy(&rendered.bytes),
                }))?;
                return Ok(());
            }

            if args.dry_run || args.out.is_none() {
                print!("{}", String::from_utf8_lossy(&rendered.bytes));
                return Ok(());
            }

            let out_path = args.out.expect("guarded by is_none check");
            write_bytes_file(&out_path, &rendered.bytes)
        }
        ArtifactKind::Dir => {
            let rendered = Renderer::render_dir(&manifest, &artifact_id, &profile)?;

            if ctx.json() {
                let entries = rendered
                    .entries
                    .iter()
                    .map(|entry| {
                        serde_json::json!({
                            "path": entry.relative_path.to_string_lossy(),
                            "kind": match entry.kind {
                                RenderedDirEntryKind::Dir => "dir",
                                RenderedDirEntryKind::File => "file",
                            },
                            "bytes_len": entry.bytes.len(),
                        })
                    })
                    .collect::<Vec<_>>();
                print_json(&serde_json::json!({
                    "artifact_id": rendered.artifact_id.as_str(),
                    "profile": rendered.profile.as_str(),
                    "kind": "dir",
                    "entries": entries,
                }))?;
                return Ok(());
            }

            if args.dry_run || args.out.is_none() {
                for entry in &rendered.entries {
                    let kind = match entry.kind {
                        RenderedDirEntryKind::Dir => "dir",
                        RenderedDirEntryKind::File => "file",
                    };
                    println!("{kind}\t{}", entry.relative_path.to_string_lossy());
                }
                return Ok(());
            }

            if let Some(out_path) = args.out {
                return write_rendered_dir(&out_path, &rendered);
            }
            Ok(())
        }
    }
}

fn validate_command(ctx: &CommandContext, args: ValidateArgs) -> Result<()> {
    let manifest = ctx.load_manifest()?;
    let profile = ctx.resolve_profile(args.profile.as_deref(), Some(&manifest))?;
    let artifact_id = ArtifactId::parse(args.artifact)?;
    let artifact_def =
        manifest
            .artifacts
            .get(&artifact_id)
            .ok_or_else(|| AgentStowError::Manifest {
                message: format!("artifact 不存在：{}", artifact_id.as_str()).into(),
            })?;

    match artifact_def.kind {
        ArtifactKind::File => {
            let rendered = Renderer::render_file(&manifest, &artifact_id, &profile)?;
            Validator::validate_rendered_file(artifact_def, &rendered.bytes)?;
        }
        ArtifactKind::Dir => {
            let _ = Renderer::render_dir(&manifest, &artifact_id, &profile)?;
        }
    }
    if ctx.json() {
        print_json(&serde_json::json!({ "ok": true }))?;
    } else {
        println!("ok");
    }
    Ok(())
}

fn env_command(ctx: &CommandContext, args: EnvArgs) -> Result<()> {
    let manifest = ctx.load_manifest()?;
    match args.cmd {
        EnvSubcommand::Emit {
            set,
            shell,
            stdout,
            out,
        } => {
            let env_set = manifest
                .env_sets
                .get(&set)
                .ok_or_else(|| AgentStowError::Manifest {
                    message: format!("env set 不存在：{set}").into(),
                })?;
            let vars = Env::resolve_env_set(env_set)?;
            let script = Env::emit_shell(shell, &vars)?;

            if ctx.json() {
                print_json(&serde_json::json!({ "shell": shell, "script": script }))?;
                return Ok(());
            }

            if stdout || out.is_none() {
                print!("{script}");
                return Ok(());
            }

            let out_path = out.expect("guarded by is_none check");
            write_text_file(&out_path, &script)
        }
    }
}

async fn scripts_command(ctx: &CommandContext, args: ScriptsArgs) -> Result<()> {
    let manifest = ctx.load_manifest()?;
    match args.cmd {
        ScriptsSubcommand::Run { id, dry_run, stdin } => {
            let mut script = manifest
                .scripts
                .get(&id)
                .ok_or_else(|| AgentStowError::Manifest {
                    message: format!("script 不存在：{id}").into(),
                })?
                .clone();
            if dry_run {
                print_json(&script)?;
                return Ok(());
            }

            if let Some(timeout_ms) = ctx.timeout() {
                script.timeout_ms = Some(timeout_ms);
            }

            let output = ScriptRunner::run(ScriptRunRequest {
                workspace_root: manifest.workspace_root.clone(),
                script,
                stdin_text: stdin,
            })
            .await?;
            if ctx.json() {
                print_json(&output)?;
            } else {
                if let Some(stdout) = &output.stdout {
                    print!("{stdout}");
                }
                if let Some(stderr) = &output.stderr {
                    eprint!("{stderr}");
                }
                println!("\n(exit={})", output.exit_code);
            }
            Ok(())
        }
    }
}

fn mcp_command(ctx: &CommandContext, args: McpArgs) -> Result<()> {
    let manifest = ctx.load_manifest()?;
    match args.cmd {
        McpSubcommand::Validate => {
            for (name, server) in &manifest.mcp_servers {
                Mcp::validate_server(name, server)?;
            }
            if ctx.json() {
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

            let out_path = out.expect("guarded by is_none check");
            write_text_file(&out_path, &json)
        }
    }
}
