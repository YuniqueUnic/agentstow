use std::path::PathBuf;

use agentstow_core::{AgentStowError, ArtifactId, Result, normalize_for_display};
use agentstow_git::Git;
use agentstow_web_types::{
    ArtifactGitCompareResponse, ArtifactGitHistoryResponse, ArtifactGitRollbackResponse,
    GitCommitSummaryResponse, WorkspaceGitSummaryResponse,
};

use super::WorkspaceQueryService;
use super::common::workspace_relative_display;

impl WorkspaceQueryService {
    pub(crate) async fn workspace_git(&self) -> Result<Option<WorkspaceGitSummaryResponse>> {
        match Git::detect(&self.workspace_root).await {
            Ok(info) => Ok(Some(WorkspaceGitSummaryResponse {
                repo_root: normalize_for_display(&info.repo_root),
                branch: info.branch,
                head: info.head,
                head_short: info.head_short,
                dirty: info.dirty,
            })),
            Err(AgentStowError::Git { message })
                if message.contains("not a git repository")
                    || message
                        .contains("not a git repository (or any of the parent directories)") =>
            {
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    pub(crate) async fn artifact_git_history(
        &self,
        artifact_id: &ArtifactId,
        limit: usize,
    ) -> Result<ArtifactGitHistoryResponse> {
        let source = self.artifact_source(artifact_id)?;
        let source_path = canonical_source_path(&source.source_path);
        let git = Git::detect(&self.workspace_root).await?;
        let commits = Git::history(
            &self.workspace_root,
            Some(&source_path),
            limit.clamp(1, 100),
        )
        .await?;
        let repo_relative_path = workspace_relative_display(&git.repo_root, &source_path);

        Ok(ArtifactGitHistoryResponse {
            artifact_id: artifact_id.as_str().to_string(),
            source_path: source.source_path,
            repo_relative_path,
            branch: git.branch,
            head: git.head,
            head_short: git.head_short,
            dirty: git.dirty,
            commits: commits
                .into_iter()
                .map(|commit| GitCommitSummaryResponse {
                    revision: commit.revision,
                    short_revision: commit.short_revision,
                    summary: commit.summary,
                    author_name: commit.author_name,
                    authored_at: commit.authored_at,
                })
                .collect(),
        })
    }

    pub(crate) async fn artifact_git_compare(
        &self,
        artifact_id: &ArtifactId,
        base_revision: &str,
        head_revision: &str,
    ) -> Result<ArtifactGitCompareResponse> {
        let source = self.artifact_source(artifact_id)?;
        let source_path = canonical_source_path(&source.source_path);
        let compare = Git::compare_file(
            &self.workspace_root,
            &source_path,
            base_revision,
            head_revision,
        )
        .await?;
        let changed = compare.base_content != compare.head_content;

        Ok(ArtifactGitCompareResponse {
            artifact_id: artifact_id.as_str().to_string(),
            source_path: source.source_path,
            repo_relative_path: compare
                .repo_relative_path
                .to_string_lossy()
                .replace('\\', "/"),
            base_revision: compare.base_revision,
            head_revision: compare.head_revision,
            base_label: compare.base_label,
            head_label: compare.head_label,
            base_content: compare.base_content,
            head_content: compare.head_content,
            changed,
        })
    }

    pub(crate) async fn artifact_git_rollback(
        &self,
        artifact_id: &ArtifactId,
        revision: &str,
    ) -> Result<ArtifactGitRollbackResponse> {
        let current_source = self.artifact_source(artifact_id)?;
        let source_path = canonical_source_path(&current_source.source_path);
        Git::rollback_file(&self.workspace_root, &source_path, revision).await?;
        let commit = Git::resolve_commit(&self.workspace_root, revision).await?;
        let source = self.artifact_source(artifact_id)?;

        Ok(ArtifactGitRollbackResponse {
            artifact_id: artifact_id.as_str().to_string(),
            commit: GitCommitSummaryResponse {
                revision: commit.revision,
                short_revision: commit.short_revision,
                summary: commit.summary,
                author_name: commit.author_name,
                authored_at: commit.authored_at,
            },
            source,
        })
    }
}

fn canonical_source_path(source_path: &str) -> PathBuf {
    fs_err::canonicalize(source_path).unwrap_or_else(|_| PathBuf::from(source_path))
}
