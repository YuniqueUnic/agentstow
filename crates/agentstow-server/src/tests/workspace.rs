use assert_fs::prelude::*;
use pretty_assertions::assert_eq;

use crate::{WatchMode, WatchStatusSnapshot, WatchTraceEvent, WatchTraceLevel};

use super::fixtures::{
    test_server, test_server_unconfigured, test_server_with_watch, write_minimal_workspace,
};

#[tokio::test]
async fn api_health_should_return_ok() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_minimal_workspace(&temp);

    let server = test_server(&temp, temp.child("missing-dist").path().to_path_buf());
    let resp = server.get("/api/health").await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body, serde_json::json!({ "ok": true }));
}

#[tokio::test]
async fn api_watch_status_should_return_manual_snapshot() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_minimal_workspace(&temp);
    let watch_status = WatchStatusSnapshot::manual(
        vec![temp.path().display().to_string()],
        Some("native watcher 不可用".to_string()),
    );
    let server = test_server_with_watch(
        &temp,
        temp.child("missing-dist").path().to_path_buf(),
        watch_status,
    );

    let resp = server.get("/api/watch-status").await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["mode"], serde_json::json!("manual"));
    assert_eq!(body["healthy"], serde_json::json!(false));
    assert_eq!(body["revision"], serde_json::json!(0));
    assert_eq!(
        body["watch_roots"],
        serde_json::json!([temp.path().display().to_string()])
    );
    assert_eq!(body["recent_events"], serde_json::json!([]));
}

#[tokio::test]
async fn api_watch_status_should_expose_poll_fallback_details() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_minimal_workspace(&temp);
    let server = test_server_with_watch(
        &temp,
        temp.child("missing-dist").path().to_path_buf(),
        WatchStatusSnapshot {
            mode: WatchMode::Poll,
            healthy: true,
            revision: 7,
            poll_interval_ms: Some(2_000),
            last_event: Some("Modify(Data) · agentstow.toml".to_string()),
            last_event_at: Some("2026-03-13T12:00:00Z".to_string()),
            last_error: Some("native watcher 不可用，已回退到 polling".to_string()),
            watch_roots: vec![temp.path().display().to_string()],
            recent_events: vec![
                WatchTraceEvent {
                    revision: 7,
                    level: WatchTraceLevel::Change,
                    summary: "1 条事件 · Modify(Data(Content)) · agentstow.toml".to_string(),
                    at: "2026-03-13T12:00:00Z".to_string(),
                },
                WatchTraceEvent {
                    revision: 0,
                    level: WatchTraceLevel::Error,
                    summary: "native watcher 不可用，已回退到 polling".to_string(),
                    at: "2026-03-13T11:59:58Z".to_string(),
                },
            ],
        },
    );

    let resp = server.get("/api/watch-status").await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["mode"], serde_json::json!("poll"));
    assert_eq!(body["healthy"], serde_json::json!(true));
    assert_eq!(body["revision"], serde_json::json!(7));
    assert_eq!(body["poll_interval_ms"], serde_json::json!(2_000));
    assert_eq!(
        body["last_error"],
        serde_json::json!("native watcher 不可用，已回退到 polling")
    );
    assert_eq!(
        body["recent_events"][0]["level"],
        serde_json::json!("change")
    );
    assert_eq!(
        body["recent_events"][0]["summary"],
        serde_json::json!("1 条事件 · Modify(Data(Content)) · agentstow.toml")
    );
    assert_eq!(
        body["recent_events"][1]["level"],
        serde_json::json!("error")
    );
}

#[tokio::test]
async fn api_manifest_should_list_workspace_entities() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_minimal_workspace(&temp);

    let server = test_server(&temp, temp.child("missing-dist").path().to_path_buf());
    let resp = server.get("/api/manifest").await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["profiles"], serde_json::json!(["base"]));
    assert_eq!(body["artifacts"], serde_json::json!(["hello"]));
    assert_eq!(body["targets"], serde_json::json!([]));
}

#[tokio::test]
async fn api_manifest_should_return_json_error_when_manifest_is_missing() {
    let temp = assert_fs::TempDir::new().unwrap();
    let server = test_server(&temp, temp.child("missing-dist").path().to_path_buf());

    let resp = server.get("/api/manifest").await;

    resp.assert_status_bad_request();
    let body: serde_json::Value = resp.json();
    let message = body["message"].as_str().unwrap();
    assert!(!message.trim().is_empty());
}

#[tokio::test]
async fn api_manifest_source_should_read_and_update_manifest() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_minimal_workspace(&temp);

    let server = test_server(&temp, temp.child("missing-dist").path().to_path_buf());

    let resp = server.get("/api/manifest/source").await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert!(
        body["source_path"]
            .as_str()
            .unwrap()
            .ends_with("agentstow.toml")
    );
    assert!(
        body["content"]
            .as_str()
            .unwrap()
            .contains("[artifacts.hello]")
    );

    let updated_manifest = r#"
[profiles.base]
vars = { name = "Server" }

[artifacts.hello]
kind = "file"
source = "artifacts/hello.txt.tera"
template = true
validate_as = "none"

[targets.hello]
artifact = "hello"
profile = "base"
target_path = "proj/AGENTS.md"
method = "copy"
"#;

    let resp = server
        .put("/api/manifest/source")
        .json(&serde_json::json!({ "content": updated_manifest }))
        .await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert!(
        body["content"]
            .as_str()
            .unwrap()
            .contains("[targets.hello]")
    );

    let updated = std::fs::read_to_string(temp.child("agentstow.toml").path()).unwrap();
    assert_eq!(updated, updated_manifest);

    let watch_resp = server.get("/api/watch-status").await;
    watch_resp.assert_status_ok();
    let watch_body: serde_json::Value = watch_resp.json();
    assert_eq!(watch_body["revision"], serde_json::json!(1));
    assert_eq!(
        watch_body["recent_events"][0]["summary"],
        serde_json::json!("save agentstow.toml")
    );
}

#[tokio::test]
async fn api_workspace_should_start_unconfigured_then_select_workspace() {
    let workspace = assert_fs::TempDir::new().unwrap();
    write_minimal_workspace(&workspace);

    let temp = assert_fs::TempDir::new().unwrap();
    let server = test_server_unconfigured(temp.child("missing-dist").path().to_path_buf());

    let resp = server.get("/api/workspace").await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert!(body["workspace_root"].is_null());
    assert_eq!(body["manifest_present"], serde_json::json!(false));

    let resp = server
        .post("/api/workspace")
        .json(&serde_json::json!({
            "workspace_root": workspace.path().display().to_string()
        }))
        .await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["manifest_present"], serde_json::json!(true));

    let resp = server.get("/api/manifest").await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["artifacts"], serde_json::json!(["hello"]));
}
