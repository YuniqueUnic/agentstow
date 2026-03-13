use assert_fs::prelude::*;
use axum_test::TestServer;
use pretty_assertions::assert_eq;
use serial_test::serial;

use super::build_app;

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

fn write_web_dist(temp: &assert_fs::TempDir) {
    temp.child("dist/assets").create_dir_all().unwrap();
    temp.child("dist/index.html")
        .write_str("<!doctype html><html><body><div id=\"app\">AgentStow UI</div></body></html>")
        .unwrap();
    temp.child("dist/assets/app.js")
        .write_str("console.log('agentstow-ui');")
        .unwrap();
}

#[tokio::test]
async fn api_health_should_return_ok() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_minimal_workspace(&temp);

    let server = TestServer::new(build_app(temp.path().to_path_buf()));
    let resp = server.get("/api/health").await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body, serde_json::json!({ "ok": true }));
}

#[tokio::test]
async fn api_manifest_should_list_workspace_entities() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_minimal_workspace(&temp);

    let server = TestServer::new(build_app(temp.path().to_path_buf()));
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
    let server = TestServer::new(build_app(temp.path().to_path_buf()));

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

    let server = TestServer::new(build_app(temp.path().to_path_buf()));
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

    let server = TestServer::new(build_app(temp.path().to_path_buf()));
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

    let server = TestServer::new(build_app(temp.path().to_path_buf()));
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
            let server = TestServer::new(build_app(temp.path().to_path_buf()));
            let links_resp = server.get("/api/links").await;
            links_resp.assert_status_ok();
            let links_body: serde_json::Value = links_resp.json();
            assert_eq!(links_body[0]["method"], serde_json::json!("copy"));

            let resp = server.get("/api/link-status").await;

            resp.assert_status_ok();
            let body: serde_json::Value = resp.json();
            assert_eq!(body.as_array().unwrap().len(), 1);
            assert_eq!(body[0]["ok"], serde_json::json!(true));
            assert_eq!(body[0]["method"], serde_json::json!("copy"));
            assert_eq!(body[0]["message"], serde_json::json!("healthy"));
        });
    });
}

#[test]
#[serial]
fn ui_should_return_service_unavailable_when_dist_is_missing() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_minimal_workspace(&temp);
    let missing_dist = temp.child("missing-dist");

    temp_env::with_var("AGENTSTOW_WEB_DIST", Some(missing_dist.path()), || {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let server = TestServer::new(build_app(temp.path().to_path_buf()));
            let resp = server.get("/").await;

            resp.assert_status_service_unavailable();
            let body = resp.text();
            assert!(body.contains("AgentStow Web UI 未构建"));
            assert!(body.contains("web/dist"));
        });
    });
}

#[test]
#[serial]
fn ui_should_serve_spa_index_for_client_routes_and_assets_from_dist() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_minimal_workspace(&temp);
    write_web_dist(&temp);

    temp_env::with_var("AGENTSTOW_WEB_DIST", Some(temp.child("dist").path()), || {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let server = TestServer::new(build_app(temp.path().to_path_buf()));

            let root = server.get("/").await;
            root.assert_status_ok();
            assert!(root.text().contains("AgentStow UI"));

            let route = server.get("/artifacts/hello").await;
            route.assert_status_ok();
            assert!(route.text().contains("AgentStow UI"));

            let asset = server.get("/assets/app.js").await;
            asset.assert_status_ok();
            assert_eq!(asset.text(), "console.log('agentstow-ui');");

            let missing_asset = server.get("/assets/missing.js").await;
            missing_asset.assert_status_not_found();
        });
    });
}
