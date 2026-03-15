use std::path::Path;

use pretty_assertions::assert_eq;

use super::*;

fn git(temp: &assert_fs::TempDir, args: &[&str]) -> String {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {:?} should succeed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn init_git_repo(temp: &assert_fs::TempDir) {
    git(temp, &["init"]);
    git(temp, &["config", "user.name", "AgentStow Tests"]);
    git(
        temp,
        &["config", "user.email", "agentstow-tests@example.com"],
    );
}

fn write_file(temp: &assert_fs::TempDir, path: &str, content: &str) {
    let full_path = temp.path().join(path);
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(full_path, content).unwrap();
}

fn commit_all(temp: &assert_fs::TempDir, message: &str) -> String {
    git(temp, &["add", "."]);
    git(temp, &["commit", "-m", message]);
    git(temp, &["rev-parse", "HEAD"])
}

#[tokio::test]
async fn detect_should_return_head() {
    let temp = assert_fs::TempDir::new().unwrap();
    init_git_repo(&temp);
    write_file(&temp, "hello.txt", "hello v1\n");
    commit_all(&temp, "initial");

    let info = Git::detect(temp.path()).await.unwrap();
    assert_eq!(info.head.len(), 40);
    assert!(!info.head_short.is_empty());
    assert!(!info.dirty);
}

#[tokio::test]
async fn detect_should_fail_outside_git_repo() {
    let temp = assert_fs::TempDir::new().unwrap();
    let err = Git::detect(temp.path()).await.unwrap_err();
    assert_eq!(
        err.exit_code(),
        agentstow_core::ExitCode::ExternalCommandFailed
    );
    assert!(err.to_string().contains("git"));
}

#[tokio::test]
async fn history_should_return_commits_for_file() {
    let temp = assert_fs::TempDir::new().unwrap();
    init_git_repo(&temp);
    write_file(&temp, "artifacts/hello.txt", "hello v1\n");
    commit_all(&temp, "initial hello");
    write_file(&temp, "artifacts/hello.txt", "hello v2\n");
    commit_all(&temp, "update hello");

    let commits = Git::history(
        temp.path(),
        Some(&temp.path().join("artifacts/hello.txt")),
        10,
    )
    .await
    .unwrap();

    assert_eq!(commits.len(), 2);
    assert_eq!(commits[0].summary, "update hello");
    assert_eq!(commits[1].summary, "initial hello");
}

#[tokio::test]
async fn compare_file_should_read_revision_and_worktree() {
    let temp = assert_fs::TempDir::new().unwrap();
    init_git_repo(&temp);
    write_file(&temp, "artifacts/hello.txt", "hello v1\n");
    let initial = commit_all(&temp, "initial hello");
    write_file(&temp, "artifacts/hello.txt", "hello worktree\n");

    let compare = Git::compare_file(
        temp.path(),
        &temp.path().join("artifacts/hello.txt"),
        initial.trim(),
        WORKTREE_REVISION,
    )
    .await
    .unwrap();

    assert_eq!(compare.base_content, "hello v1\n");
    assert_eq!(compare.head_content, "hello worktree\n");
    assert_eq!(compare.head_label, "Worktree");
    assert_eq!(compare.repo_relative_path, Path::new("artifacts/hello.txt"));
}

#[tokio::test]
async fn rollback_file_should_restore_content_from_revision() {
    let temp = assert_fs::TempDir::new().unwrap();
    init_git_repo(&temp);
    write_file(&temp, "artifacts/hello.txt", "hello v1\n");
    let initial = commit_all(&temp, "initial hello");
    write_file(&temp, "artifacts/hello.txt", "hello v2\n");
    commit_all(&temp, "update hello");
    write_file(&temp, "artifacts/hello.txt", "scratch\n");

    let restored = Git::rollback_file(
        temp.path(),
        &temp.path().join("artifacts/hello.txt"),
        initial.trim(),
    )
    .await
    .unwrap();

    assert_eq!(restored, "hello v1\n");
    assert_eq!(
        std::fs::read_to_string(temp.path().join("artifacts/hello.txt")).unwrap(),
        "hello v1\n"
    );
}
