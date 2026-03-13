use std::path::{Path, PathBuf};

use agentstow_core::{AgentStowError, Result, normalize_for_display};
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tracing::instrument;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitInfo {
    pub repo_root: PathBuf,
    pub head: String,
    pub dirty: bool,
}

pub struct Git;

impl Git {
    #[instrument(skip_all, fields(start_dir=%normalize_for_display(start_dir)))]
    pub async fn detect(start_dir: &Path) -> Result<GitInfo> {
        let repo_root = run_git(start_dir, &["rev-parse", "--show-toplevel"]).await?;
        let head = run_git(start_dir, &["rev-parse", "HEAD"]).await?;
        let porcelain = run_git(start_dir, &["status", "--porcelain"]).await?;
        Ok(GitInfo {
            repo_root: PathBuf::from(repo_root.trim()),
            head: head.trim().to_string(),
            dirty: !porcelain.trim().is_empty(),
        })
    }
}

async fn run_git(cwd: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .await
        .map_err(|e| AgentStowError::Git {
            message: format!("执行 git 失败：{e}").into(),
        })?;
    if !out.status.success() {
        return Err(AgentStowError::Git {
            message: format!(
                "git {:?} 失败：exit={:?}, stderr={}",
                args,
                out.status.code(),
                String::from_utf8_lossy(&out.stderr)
            )
            .into(),
        });
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

#[cfg(test)]
mod tests;
