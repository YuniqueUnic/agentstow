use agentstow_core::{AgentStowError, Result, normalize_for_display};
use agentstow_git::Git;
use std::process::Stdio;

use crate::bootstrap::CommandContext;
use crate::cli::{WorkspaceArgs, WorkspaceSubcommand};
use crate::output::print_json;

pub(crate) async fn workspace_command(ctx: &CommandContext, args: WorkspaceArgs) -> Result<()> {
    match args.cmd {
        WorkspaceSubcommand::Status => workspace_status(ctx).await,
        WorkspaceSubcommand::Init { git_init } => workspace_init(ctx, git_init).await,
    }
}

async fn workspace_status(ctx: &CommandContext) -> Result<()> {
    let info = Git::detect(ctx.cwd()).await?;
    if ctx.json() {
        print_json(&info)?;
        return Ok(());
    }

    println!("Repo root: {}", normalize_for_display(&info.repo_root));
    if let Some(branch) = &info.branch {
        println!("Branch: {branch}");
    }
    println!("HEAD short: {}", info.head_short);
    println!("HEAD: {}", info.head);
    println!("Dirty: {}", info.dirty);
    Ok(())
}

async fn workspace_init(ctx: &CommandContext, git_init: bool) -> Result<()> {
    let workspace_root = ctx.workspace_init_root();
    let output = agentstow_manifest::init_workspace_skeleton(&workspace_root)?;

    let mut git_inited = false;
    if git_init && !workspace_root.join(".git").exists() {
        let output = tokio::process::Command::new("git")
            .arg("init")
            .current_dir(&workspace_root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(AgentStowError::from)?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AgentStowError::Other(anyhow::anyhow!(
                "git init 失败（exit={}）{}{}",
                output.status.code().unwrap_or(-1),
                if stderr.trim().is_empty() { "" } else { "：" },
                stderr.trim()
            )));
        }
        git_inited = true;
    }

    if ctx.json() {
        print_json(&serde_json::json!({
            "workspace_root": normalize_for_display(&output.workspace_root),
            "manifest_path": normalize_for_display(&output.manifest_path),
            "created": output.created,
            "git_inited": git_inited,
        }))?;
        return Ok(());
    }

    println!(
        "Workspace root: {}",
        normalize_for_display(&output.workspace_root)
    );
    println!("Manifest: {}", normalize_for_display(&output.manifest_path));
    println!("Created: {}", output.created);
    if git_init {
        println!("Git init: {}", git_inited);
    }
    Ok(())
}
