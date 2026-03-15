use assert_fs::prelude::*;
use pretty_assertions::assert_eq;
use serial_test::serial;

use super::fixtures::{block_on, git_stdout, init_git_repo, test_server, write_minimal_workspace};

#[tokio::test]
async fn api_workspace_git_should_return_null_for_non_git_workspace() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_minimal_workspace(&temp);

    let server = test_server(&temp, temp.child("missing-dist").path().to_path_buf());
    let resp = server.get("/api/workspace/git").await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert!(body.is_null());
}

#[test]
#[serial]
fn api_workspace_git_should_expose_branch_and_head_short() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_minimal_workspace(&temp);
    init_git_repo(&temp);

    block_on(async {
        let server = test_server(&temp, temp.child("missing-dist").path().to_path_buf());
        let resp = server.get("/api/workspace/git").await;

        resp.assert_status_ok();
        let body: serde_json::Value = resp.json();
        let branch = body["branch"].as_str().unwrap();
        let head = body["head"].as_str().unwrap();
        let head_short = body["head_short"].as_str().unwrap();
        let repo_root =
            agentstow_core::normalize_for_display(&fs_err::canonicalize(temp.path()).unwrap());

        assert_eq!(body["repo_root"], serde_json::json!(repo_root));
        assert!(!branch.is_empty());
        assert!(!head.is_empty());
        assert!(!head_short.is_empty());
        assert!(head.starts_with(head_short));
        assert_eq!(body["dirty"], serde_json::json!(false));
    });
}

#[test]
#[serial]
fn api_artifact_git_history_should_return_file_commits() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_minimal_workspace(&temp);
    init_git_repo(&temp);
    temp.child("artifacts/hello.txt.tera")
        .write_str("Hello {{ name }} from Git!")
        .unwrap();
    git_stdout(&temp, &["add", "."]);
    git_stdout(&temp, &["commit", "-m", "update hello template"]);

    block_on(async {
        let server = test_server(&temp, temp.child("missing-dist").path().to_path_buf());
        let resp = server
            .get("/api/artifacts/hello/git/history")
            .add_query_param("limit", "5")
            .await;

        resp.assert_status_ok();
        let body: serde_json::Value = resp.json();
        assert_eq!(body["artifact_id"], serde_json::json!("hello"));
        assert_eq!(
            body["repo_relative_path"],
            serde_json::json!("artifacts/hello.txt.tera")
        );
        assert_eq!(body["dirty"], serde_json::json!(false));
        assert!(body["head"].as_str().unwrap().len() == 40);
        assert!(!body["head_short"].as_str().unwrap().is_empty());
        assert_eq!(body["commits"].as_array().unwrap().len(), 2);
        assert_eq!(
            body["commits"][0]["summary"],
            serde_json::json!("update hello template")
        );
    });
}

#[test]
#[serial]
fn api_artifact_git_compare_should_return_revision_and_worktree_contents() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_minimal_workspace(&temp);
    init_git_repo(&temp);
    let initial = git_stdout(&temp, &["rev-parse", "HEAD"]);
    temp.child("artifacts/hello.txt.tera")
        .write_str("Hello {{ name }} from Worktree!")
        .unwrap();

    block_on(async {
        let server = test_server(&temp, temp.child("missing-dist").path().to_path_buf());
        let resp = server
            .get("/api/artifacts/hello/git/compare")
            .add_query_param("base", &initial)
            .await;

        resp.assert_status_ok();
        let body: serde_json::Value = resp.json();
        assert_eq!(body["artifact_id"], serde_json::json!("hello"));
        assert_eq!(body["base_revision"], serde_json::json!(initial));
        assert_eq!(body["head_revision"], serde_json::json!("WORKTREE"));
        assert_eq!(body["base_content"], serde_json::json!("Hello {{ name }}!"));
        assert_eq!(body["changed"], serde_json::json!(true));
        assert_eq!(
            body["head_content"],
            serde_json::json!("Hello {{ name }} from Worktree!")
        );
    });
}

#[test]
#[serial]
fn api_artifact_git_rollback_should_restore_file_content() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_minimal_workspace(&temp);
    init_git_repo(&temp);
    let initial = git_stdout(&temp, &["rev-parse", "HEAD"]);
    temp.child("artifacts/hello.txt.tera")
        .write_str("Hello {{ name }} from rollback!")
        .unwrap();

    block_on(async {
        let server = test_server(&temp, temp.child("missing-dist").path().to_path_buf());
        let resp = server
            .post("/api/artifacts/hello/git/rollback")
            .json(&serde_json::json!({ "revision": initial }))
            .await;

        resp.assert_status_ok();
        let body: serde_json::Value = resp.json();
        assert_eq!(body["artifact_id"], serde_json::json!("hello"));
        assert_eq!(body["commit"]["revision"], serde_json::json!(initial));
        assert_eq!(body["source"]["artifact_id"], serde_json::json!("hello"));
        assert_eq!(
            body["source"]["content"],
            serde_json::json!("Hello {{ name }}!")
        );
        assert_eq!(
            std::fs::read_to_string(temp.child("artifacts/hello.txt.tera").path()).unwrap(),
            "Hello {{ name }}!"
        );

        let watch_resp = server.get("/api/watch-status").await;
        watch_resp.assert_status_ok();
        let watch_body: serde_json::Value = watch_resp.json();
        assert_eq!(watch_body["revision"], serde_json::json!(1));
        assert!(
            watch_body["recent_events"][0]["summary"]
                .as_str()
                .unwrap()
                .starts_with("rollback artifacts/hello.txt.tera <=")
        );
    });
}
