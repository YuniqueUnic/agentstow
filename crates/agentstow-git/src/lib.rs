use std::path::{Path, PathBuf};

use agentstow_core::{AgentStowError, Result, normalize_for_display};
use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::process::Command;
use tracing::instrument;

pub const WORKTREE_REVISION: &str = "WORKTREE";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitInfo {
    pub repo_root: PathBuf,
    pub branch: Option<String>,
    pub head: String,
    pub head_short: String,
    pub dirty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitCommit {
    pub revision: String,
    pub short_revision: String,
    pub summary: String,
    pub author_name: String,
    pub authored_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitFileCompare {
    pub repo_root: PathBuf,
    pub repo_relative_path: PathBuf,
    pub base_revision: String,
    pub head_revision: String,
    pub base_label: String,
    pub head_label: String,
    pub base_content: String,
    pub head_content: String,
}

pub struct Git;

impl Git {
    #[instrument(skip_all, fields(start_dir=%normalize_for_display(start_dir)))]
    pub async fn detect(start_dir: &Path) -> Result<GitInfo> {
        let repo_root = repo_root(start_dir).await?;
        let branch = match run_git(start_dir, &["symbolic-ref", "--quiet", "--short", "HEAD"]).await
        {
            Ok(value) => Some(value.trim().to_string()),
            Err(AgentStowError::Git { message })
                if message.contains("not a symbolic ref")
                    || message.contains("fatal: ref HEAD is not a symbolic ref") =>
            {
                None
            }
            Err(error) => return Err(error),
        };
        let head = run_git(start_dir, &["rev-parse", "HEAD"]).await?;
        let head_short = run_git(start_dir, &["rev-parse", "--short", "HEAD"]).await?;
        let porcelain = run_git(start_dir, &["status", "--porcelain"]).await?;
        Ok(GitInfo {
            repo_root,
            branch,
            head: head.trim().to_string(),
            head_short: head_short.trim().to_string(),
            dirty: !porcelain.trim().is_empty(),
        })
    }

    #[instrument(skip_all, fields(start_dir=%normalize_for_display(start_dir), limit))]
    pub async fn history(
        start_dir: &Path,
        path: Option<&Path>,
        limit: usize,
    ) -> Result<Vec<GitCommit>> {
        let repo_root = repo_root(start_dir).await?;
        let format = "%H%x1f%h%x1f%an%x1f%aI%x1f%s%x1e";
        let mut args = vec![
            "log".to_string(),
            format!("--max-count={}", limit.max(1)),
            format!("--format={format}"),
        ];

        if let Some(path) = path {
            args.push("--follow".to_string());
            args.push("--".to_string());
            args.push(to_repo_relative_path(&repo_root, path)?);
        }

        let raw = run_git_owned(start_dir, &args).await?;
        Ok(parse_history(&raw))
    }

    #[instrument(skip_all, fields(start_dir=%normalize_for_display(start_dir), revision))]
    pub async fn resolve_commit(start_dir: &Path, revision: &str) -> Result<GitCommit> {
        let format = "%H%x1f%h%x1f%an%x1f%aI%x1f%s%x1e";
        let args = [
            "log".to_string(),
            "-1".to_string(),
            format!("--format={format}"),
            revision.to_string(),
        ];
        let raw = run_git_owned(start_dir, &args).await?;
        parse_history(&raw)
            .into_iter()
            .next()
            .ok_or_else(|| AgentStowError::Git {
                message: format!("无法解析 revision `{revision}` 的提交信息").into(),
            })
    }

    #[instrument(skip_all, fields(start_dir=%normalize_for_display(start_dir), revision))]
    pub async fn show_file_at_revision(
        start_dir: &Path,
        path: &Path,
        revision: &str,
    ) -> Result<Option<String>> {
        let repo_root = repo_root(start_dir).await?;
        let repo_relative_path = to_repo_relative_path(&repo_root, path)?;
        let spec = format!("{revision}:{repo_relative_path}");
        let args = ["show".to_string(), spec];

        match run_git_owned(start_dir, &args).await {
            Ok(text) => Ok(Some(text)),
            Err(AgentStowError::Git { message }) if is_missing_path_error(&message) => Ok(None),
            Err(error) => Err(error),
        }
    }

    #[instrument(skip_all, fields(start_dir=%normalize_for_display(start_dir), base_revision, head_revision))]
    pub async fn compare_file(
        start_dir: &Path,
        path: &Path,
        base_revision: &str,
        head_revision: &str,
    ) -> Result<GitFileCompare> {
        let repo_root = repo_root(start_dir).await?;
        let repo_relative_path = PathBuf::from(to_repo_relative_path(&repo_root, path)?);
        let base_content = resolve_revision_content(start_dir, path, base_revision).await?;
        let head_content = resolve_revision_content(start_dir, path, head_revision).await?;

        Ok(GitFileCompare {
            repo_root,
            repo_relative_path,
            base_revision: base_revision.to_string(),
            head_revision: head_revision.to_string(),
            base_label: format_revision_label(base_revision),
            head_label: format_revision_label(head_revision),
            base_content,
            head_content,
        })
    }

    #[instrument(skip_all, fields(start_dir=%normalize_for_display(start_dir), revision, path=%normalize_for_display(path)))]
    pub async fn rollback_file(start_dir: &Path, path: &Path, revision: &str) -> Result<String> {
        let content = Self::show_file_at_revision(start_dir, path, revision)
            .await?
            .ok_or_else(|| AgentStowError::Git {
                message: format!(
                    "revision `{revision}` 中不存在文件：{}",
                    normalize_for_display(path)
                )
                .into(),
            })?;

        fs::write(path, &content)
            .await
            .map_err(AgentStowError::from)?;
        Ok(content)
    }
}

async fn resolve_revision_content(start_dir: &Path, path: &Path, revision: &str) -> Result<String> {
    if revision.eq_ignore_ascii_case(WORKTREE_REVISION) {
        return fs::read_to_string(path).await.map_err(AgentStowError::from);
    }

    Ok(Git::show_file_at_revision(start_dir, path, revision)
        .await?
        .unwrap_or_default())
}

async fn repo_root(start_dir: &Path) -> Result<PathBuf> {
    let repo_root = run_git(start_dir, &["rev-parse", "--show-toplevel"]).await?;
    Ok(PathBuf::from(repo_root.trim()))
}

fn to_repo_relative_path(repo_root: &Path, path: &Path) -> Result<String> {
    let canonical_repo_root =
        std::fs::canonicalize(repo_root).unwrap_or_else(|_| repo_root.to_path_buf());
    let candidate_path = if path.is_absolute() {
        std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
    } else {
        path.to_path_buf()
    };

    let relative = if candidate_path.is_absolute() {
        candidate_path
            .strip_prefix(&canonical_repo_root)
            .or_else(|_| candidate_path.strip_prefix(repo_root))
            .map_err(|_| AgentStowError::Git {
                message: format!(
                    "路径不在 git repo 内：{}（repo_root={}）",
                    normalize_for_display(&candidate_path),
                    normalize_for_display(repo_root)
                )
                .into(),
            })?
    } else {
        candidate_path.as_path()
    };

    Ok(relative.to_string_lossy().replace('\\', "/"))
}

fn parse_history(raw: &str) -> Vec<GitCommit> {
    raw.split('\x1e')
        .filter_map(|record| {
            let trimmed = record.trim();
            if trimmed.is_empty() {
                return None;
            }

            let mut parts = trimmed.split('\x1f');
            Some(GitCommit {
                revision: parts.next()?.to_string(),
                short_revision: parts.next()?.to_string(),
                author_name: parts.next()?.to_string(),
                authored_at: parts.next()?.to_string(),
                summary: parts.next()?.to_string(),
            })
        })
        .collect()
}

fn format_revision_label(revision: &str) -> String {
    if revision.eq_ignore_ascii_case(WORKTREE_REVISION) {
        "Worktree".to_string()
    } else {
        revision.to_string()
    }
}

fn is_missing_path_error(message: &str) -> bool {
    message.contains("exists on disk, but not in")
        || message.contains("does not exist in")
        || message.contains("unknown revision or path not in the working tree")
}

async fn run_git(cwd: &Path, args: &[&str]) -> Result<String> {
    let args_owned: Vec<String> = args.iter().map(|arg| (*arg).to_string()).collect();
    run_git_owned(cwd, &args_owned).await
}

async fn run_git_owned(cwd: &Path, args: &[String]) -> Result<String> {
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
