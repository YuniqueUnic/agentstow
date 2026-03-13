use assert_fs::prelude::*;
use axum::http::StatusCode;
use axum_test::TestServer;
use pretty_assertions::assert_eq;
use serial_test::serial;

use super::{build_app_with_ui_dist, ui_dist_missing_page};

fn write_minimal_workspace(temp: &assert_fs::TempDir) {
    temp.child("artifacts").create_dir_all().unwrap();
    temp.child("artifacts/hello.txt.tera")
        .write_str("Hello {{ name }}!")
        .unwrap();
    temp.child("agentstow.toml")
        .write_str(
            r#"
[profiles.base]
vars = { name = "Server" }

[artifacts.hello]
kind = "file"
source = "artifacts/hello.txt.tera"
template = true
validate_as = "none"
"#,
        )
        .unwrap();
}

fn write_ui_dist(temp: &assert_fs::TempDir) -> std::path::PathBuf {
    temp.child("web-dist/assets").create_dir_all().unwrap();
    temp.child("web-dist/index.html")
        .write_str(
            "<!doctype html><html><body><div id=\"app\">agentstow-web</div><script src=\"/assets/app.js\"></script></body></html>",
        )
        .unwrap();
    temp.child("web-dist/assets/app.js")
        .write_str("console.log('agentstow-web');")
        .unwrap();
    temp.child("web-dist").path().to_path_buf()
}

fn test_server(temp: &assert_fs::TempDir, ui_dist_dir: std::path::PathBuf) -> TestServer {
    TestServer::new(build_app_with_ui_dist(
        temp.path().to_path_buf(),
        ui_dist_dir,
    ))
}

#[tokio::test]
async fn ui_root_should_serve_built_index_html() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_minimal_workspace(&temp);
    let server = test_server(&temp, write_ui_dist(&temp));

    let resp = server.get("/").await;

    resp.assert_status_ok();
    resp.assert_text_contains("agentstow-web");
}

#[tokio::test]
async fn ui_non_api_path_should_fallback_to_index_html() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_minimal_workspace(&temp);
    let server = test_server(&temp, write_ui_dist(&temp));

    let resp = server.get("/links/overview").await;

    resp.assert_status_ok();
    resp.assert_text_contains("agentstow-web");
}

#[tokio::test]
async fn ui_assets_should_serve_built_static_files() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_minimal_workspace(&temp);
    let server = test_server(&temp, write_ui_dist(&temp));

    let resp = server.get("/assets/app.js").await;

    resp.assert_status_ok();
    resp.assert_text_contains("agentstow-web");
}

#[tokio::test]
async fn ui_should_return_helpful_message_when_dist_is_missing() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_minimal_workspace(&temp);
    let missing_dist = temp.child("missing-dist").path().to_path_buf();
    let server = test_server(&temp, missing_dist.clone());

    let resp = server.get("/").await;

    resp.assert_status(StatusCode::SERVICE_UNAVAILABLE);
    resp.assert_text(ui_dist_missing_page(&missing_dist.join("index.html")));
}

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
async fn api_render_should_validate_rendered_output() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("artifacts").create_dir_all().unwrap();
    temp.child("artifacts/bad.json.tera")
        .write_str("{ invalid: {{ value }} }")
        .unwrap();
    temp.child("agentstow.toml")
        .write_str(
            r#"
[profiles.base]
vars = { value = 1 }

[artifacts.bad]
kind = "file"
source = "artifacts/bad.json.tera"
template = true
validate_as = "json"
"#,
        )
        .unwrap();

    let server = test_server(&temp, temp.child("missing-dist").path().to_path_buf());
    let resp = server
        .get("/api/render")
        .add_query_param("artifact", "bad")
        .add_query_param("profile", "base")
        .await;

    resp.assert_status_bad_request();
    let body: serde_json::Value = resp.json();
    assert!(body["message"].as_str().unwrap().contains("校验失败"));
}

#[tokio::test]
async fn api_render_should_return_json_error_for_invalid_artifact_id() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_minimal_workspace(&temp);

    let server = test_server(&temp, temp.child("missing-dist").path().to_path_buf());
    let resp = server
        .get("/api/render")
        .add_query_param("artifact", "bad/id")
        .add_query_param("profile", "base")
        .await;

    resp.assert_status_bad_request();
    let body: serde_json::Value = resp.json();
    assert!(body["message"].as_str().unwrap().contains("参数错误"));
}

#[tokio::test]
async fn api_render_should_return_json_error_for_missing_profile() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_minimal_workspace(&temp);

    let server = test_server(&temp, temp.child("missing-dist").path().to_path_buf());
    let resp = server
        .get("/api/render")
        .add_query_param("artifact", "hello")
        .add_query_param("profile", "ghost")
        .await;

    resp.assert_status_bad_request();
    let body: serde_json::Value = resp.json();
    assert!(body["message"].as_str().unwrap().contains("profile 不存在"));
}

#[test]
#[serial]
fn api_links_should_serialize_link_records_with_shared_dto() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_minimal_workspace(&temp);
    temp.child("proj/hello.txt")
        .write_str("Hello Server!")
        .unwrap();
    temp.child("cache").create_dir_all().unwrap();
    temp.child("home").create_dir_all().unwrap();

    temp_env::with_var("AGENTSTOW_HOME", Some(temp.child("home").path()), || {
        let dirs = agentstow_core::AgentStowDirs::from_env().unwrap();
        let db = agentstow_state::StateDb::open(&dirs).unwrap();
        db.upsert_link_instance(&agentstow_state::LinkInstanceRecord {
            workspace_root: temp.path().to_path_buf(),
            artifact_id: agentstow_core::ArtifactId::new_unchecked("hello"),
            profile: agentstow_core::ProfileName::new_unchecked("base"),
            target_path: temp.child("proj/hello.txt").path().to_path_buf(),
            method: agentstow_core::InstallMethod::Copy,
            rendered_path: Some(temp.child("cache/hello.txt").path().to_path_buf()),
            blake3: Some("deadbeef".to_string()),
            updated_at: time::OffsetDateTime::UNIX_EPOCH,
        })
        .unwrap();

        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let server = test_server(&temp, temp.child("missing-dist").path().to_path_buf());
            let resp = server.get("/api/links").await;

            resp.assert_status_ok();
            let body: serde_json::Value = resp.json();
            assert_eq!(body.as_array().unwrap().len(), 1);
            assert_eq!(body[0]["artifact_id"], serde_json::json!("hello"));
            assert_eq!(body[0]["method"], serde_json::json!("copy"));
            assert_eq!(body[0]["blake3"], serde_json::json!("deadbeef"));
            assert_eq!(
                body[0]["updated_at"],
                serde_json::json!("1970-01-01T00:00:00Z")
            );
        });
    });
}

#[test]
#[serial]
fn api_link_status_should_report_copy_dir_as_healthy_when_tree_matches() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("artifacts/skills").create_dir_all().unwrap();
    temp.child("artifacts/skills/rule.md")
        .write_str("hello")
        .unwrap();
    temp.child("proj/.agents/skills").create_dir_all().unwrap();
    temp.child("proj/.agents/skills/rule.md")
        .write_str("hello")
        .unwrap();
    temp.child("home").create_dir_all().unwrap();
    temp.child("agentstow.toml")
        .write_str(
            r#"
[profiles.base]
vars = { name = "demo" }

[artifacts.skills]
kind = "dir"
source = "artifacts/skills"

[targets.skills]
artifact = "skills"
profile = "base"
target_path = "proj/.agents/skills"
method = "copy"
"#,
        )
        .unwrap();

    temp_env::with_var("AGENTSTOW_HOME", Some(temp.child("home").path()), || {
        let dirs = agentstow_core::AgentStowDirs::from_env().unwrap();
        let db = agentstow_state::StateDb::open(&dirs).unwrap();
        db.upsert_link_instance(&agentstow_state::LinkInstanceRecord {
            workspace_root: temp.path().to_path_buf(),
            artifact_id: agentstow_core::ArtifactId::new_unchecked("skills"),
            profile: agentstow_core::ProfileName::new_unchecked("base"),
            target_path: temp.child("proj/.agents/skills").path().to_path_buf(),
            method: agentstow_core::InstallMethod::Copy,
            rendered_path: Some(temp.child("artifacts/skills").path().to_path_buf()),
            blake3: None,
            updated_at: time::OffsetDateTime::now_utc(),
        })
        .unwrap();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let server = test_server(&temp, temp.child("missing-dist").path().to_path_buf());
            let resp = server.get("/api/link-status").await;

            resp.assert_status_ok();
            let body: serde_json::Value = resp.json();
            assert_eq!(body.as_array().unwrap().len(), 1);
            assert_eq!(body[0]["ok"], serde_json::json!(true));
            assert_eq!(body[0]["message"], serde_json::json!("healthy"));
            assert_eq!(body[0]["method"], serde_json::json!("copy"));
        });
    });
}
