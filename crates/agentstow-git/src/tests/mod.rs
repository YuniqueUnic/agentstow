use pretty_assertions::assert_eq;

use super::*;

#[tokio::test]
async fn detect_should_return_head() {
    let cwd = std::env::current_dir().unwrap();
    let info = Git::detect(&cwd).await.unwrap();
    assert_eq!(info.head.len(), 40);
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
