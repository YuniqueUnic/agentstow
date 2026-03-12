use pretty_assertions::assert_eq;

use super::*;

#[tokio::test]
async fn detect_should_return_head() {
    let cwd = std::env::current_dir().unwrap();
    let info = Git::detect(&cwd).await.unwrap();
    assert_eq!(info.head.len(), 40);
}
