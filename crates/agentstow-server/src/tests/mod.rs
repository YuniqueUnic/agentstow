mod watch;

use assert_fs::prelude::*;
use axum::http::StatusCode;
use axum_test::TestServer;
use pretty_assertions::assert_eq;
use serial_test::serial;

use super::{
    WatchMode, WatchStatusHandle, WatchStatusSnapshot, build_app_with_ui_dist_and_watch,
    resolve_ui_dist_dir_for_test, ui_dist_missing_page,
};

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

fn write_prd_workspace(temp: &assert_fs::TempDir) {
    temp.child("artifacts/skills").create_dir_all().unwrap();
    temp.child("artifacts/hello.txt.tera")
        .write_str("Hello {{ name }} from {{ region }}!")
        .unwrap();
    temp.child("artifacts/skills/rule.md")
        .write_str("Use the shared rule.")
        .unwrap();
    temp.child("agentstow.toml")
        .write_str(
            r#"
[profiles.base]
vars = { name = "AgentStow", region = "global" }

[profiles.derived]
extends = ["base"]
vars = { region = "cn" }

[artifacts.hello]
kind = "file"
source = "artifacts/hello.txt.tera"
template = true
validate_as = "none"

[artifacts.skills]
kind = "dir"
source = "artifacts/skills"

[targets.hello]
artifact = "hello"
profile = "derived"
target_path = "proj/AGENTS.md"
method = "copy"

[targets.shared_skills]
artifact = "skills"
profile = "base"
target_path = "proj/.agents/skills"
method = "copy"

[targets.ad_hoc]
artifact = "hello"
target_path = "proj/adhoc.md"
method = "copy"

[env_sets.default]
vars = [
  { key = "OPENAI_API_KEY", binding = { kind = "env", var = "OPENAI_API_KEY" } }
]

[scripts.sync]
kind = "shell"
entry = "echo"
args = ["sync"]
env = [
  { key = "OPENAI_API_KEY", binding = { kind = "env", var = "OPENAI_API_KEY" } }
]

[mcp_servers.local]
transport = { kind = "stdio", command = "npx", args = ["-y", "@modelcontextprotocol/server-filesystem", "."] }
env = [
  { key = "OPENAI_API_KEY", binding = { kind = "env", var = "OPENAI_API_KEY" } }
]
"#,
        )
        .unwrap();
}

fn write_script_workspace(temp: &assert_fs::TempDir) {
    temp.child("agentstow.toml")
        .write_str(if cfg!(windows) {
            r#"
[scripts.hello]
kind = "shell"
entry = "cmd"
args = ["/C", "echo hello"]
cwd_policy = "current"
stdin_mode = "none"
stdout_mode = "capture"
stderr_mode = "capture"
timeout_ms = 5000
expected_exit_codes = [0]
"#
        } else {
            r#"
[scripts.hello]
kind = "shell"
entry = "bash"
args = ["-lc", "echo hello"]
cwd_policy = "current"
stdin_mode = "none"
stdout_mode = "capture"
stderr_mode = "capture"
timeout_ms = 5000
expected_exit_codes = [0]
"#
        })
        .unwrap();
}

fn with_test_home(temp: &assert_fs::TempDir, f: impl FnOnce()) {
    temp.child("home").create_dir_all().unwrap();
    temp_env::with_var("AGENTSTOW_HOME", Some(temp.child("home").path()), f);
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

fn default_watch_status(temp: &assert_fs::TempDir) -> WatchStatusSnapshot {
    WatchStatusSnapshot::manual(vec![temp.path().display().to_string()], None)
}

fn test_server(temp: &assert_fs::TempDir, ui_dist_dir: std::path::PathBuf) -> TestServer {
    test_server_with_watch(temp, ui_dist_dir, default_watch_status(temp))
}

fn test_server_with_watch(
    temp: &assert_fs::TempDir,
    ui_dist_dir: std::path::PathBuf,
    watch_status: WatchStatusSnapshot,
) -> TestServer {
    TestServer::new(build_app_with_ui_dist_and_watch(
        Some(temp.path().to_path_buf()),
        ui_dist_dir,
        WatchStatusHandle::from_snapshot(watch_status),
    ))
}

fn test_server_unconfigured(ui_dist_dir: std::path::PathBuf) -> TestServer {
    TestServer::new(build_app_with_ui_dist_and_watch(
        None,
        ui_dist_dir,
        WatchStatusHandle::manual(Vec::new(), Some("workspace 未选择".to_string())),
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

#[test]
fn ui_dist_resolver_should_honor_env_override() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("override-dist/index.html")
        .write_str("<!doctype html>")
        .unwrap();
    let override_dir = temp.child("override-dist").path().to_path_buf();

    let resolved = resolve_ui_dist_dir_for_test(
        Some(override_dir.clone()),
        Some(
            temp.child("repo/target/debug/agentstow")
                .path()
                .to_path_buf(),
        ),
        temp.child("repo").path().to_path_buf(),
    );

    assert_eq!(resolved, override_dir);
}

#[test]
fn ui_dist_resolver_should_find_web_dist_from_current_exe_ancestors() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("repo/web/dist/index.html")
        .write_str("<!doctype html>")
        .unwrap();

    let exe_path = temp
        .child("repo/target/debug/agentstow")
        .path()
        .to_path_buf();

    let resolved = resolve_ui_dist_dir_for_test(
        None,
        Some(exe_path),
        temp.child("repo").path().to_path_buf(),
    );

    assert_eq!(resolved, temp.child("repo/web/dist").path().to_path_buf());
}

#[test]
fn ui_dist_resolver_should_fallback_to_repo_root_web_dist() {
    let temp = assert_fs::TempDir::new().unwrap();
    let repo_root = temp.child("repo");
    repo_root.create_dir_all().unwrap();

    let resolved = resolve_ui_dist_dir_for_test(
        None,
        Some(temp.child("bin/agentstow").path().to_path_buf()),
        repo_root.path().to_path_buf(),
    );

    assert_eq!(resolved, repo_root.child("web/dist").path().to_path_buf());
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
async fn api_artifact_source_should_read_and_update_file_artifact() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_minimal_workspace(&temp);

    let server = test_server(&temp, temp.child("missing-dist").path().to_path_buf());

    let resp = server.get("/api/artifacts/hello/source").await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["artifact_id"], serde_json::json!("hello"));
    assert_eq!(body["content"], serde_json::json!("Hello {{ name }}!"));

    let resp = server
        .put("/api/artifacts/hello/source")
        .json(&serde_json::json!({ "content": "Hi {{ name }}!" }))
        .await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["content"], serde_json::json!("Hi {{ name }}!"));

    let updated = std::fs::read_to_string(temp.child("artifacts/hello.txt.tera").path()).unwrap();
    assert_eq!(updated, "Hi {{ name }}!");
}

#[test]
#[serial]
fn api_env_emit_should_generate_shell_script_for_env_set() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_prd_workspace(&temp);

    with_test_home(&temp, || {
        temp_env::with_var("OPENAI_API_KEY", Some("token123"), || {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let server = test_server(&temp, temp.child("missing-dist").path().to_path_buf());

                let resp = server
                    .post("/api/env/emit")
                    .json(&serde_json::json!({
                        "env_set_id": "default",
                        "shell": "bash",
                    }))
                    .await;

                resp.assert_status_ok();
                let body: serde_json::Value = resp.json();
                let text = body["text"].as_str().unwrap();
                assert!(text.contains("Generated by agentstow"));
                assert!(text.contains("export OPENAI_API_KEY='token123'"));
            });
        });
    });
}

#[tokio::test]
async fn api_scripts_run_should_execute_manifest_script_and_return_output() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_script_workspace(&temp);

    let server = test_server(&temp, temp.child("missing-dist").path().to_path_buf());
    let resp = server
        .post("/api/scripts/hello/run")
        .json(&serde_json::json!({ "stdin": null }))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["exit_code"], serde_json::json!(0));
    let stdout = body["stdout"].as_str().unwrap();
    assert_eq!(stdout.trim(), "hello");
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
fn api_links_plan_apply_and_repair_should_work() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_prd_workspace(&temp);

    with_test_home(&temp, || {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let server = test_server(&temp, temp.child("missing-dist").path().to_path_buf());

            let resp = server
                .post("/api/links/plan")
                .json(&serde_json::json!({
                    "targets": [],
                    "default_profile": "base",
                }))
                .await;
            resp.assert_status_ok();
            let planned: serde_json::Value = resp.json();
            assert_eq!(planned["items"].as_array().unwrap().len(), 3);
            assert_eq!(planned["items"][0]["action"], serde_json::json!("planned"));

            let resp = server
                .post("/api/links/apply")
                .json(&serde_json::json!({
                    "targets": [],
                    "default_profile": "base",
                    "force": false,
                }))
                .await;
            resp.assert_status_ok();
            let applied: serde_json::Value = resp.json();
            assert_eq!(applied["items"].as_array().unwrap().len(), 3);
            assert!(temp.child("proj/AGENTS.md").path().is_file());
            assert!(temp.child("proj/adhoc.md").path().is_file());
            assert!(temp.child("proj/.agents/skills/rule.md").path().is_file());

            let resp = server
                .post("/api/links/repair")
                .json(&serde_json::json!({
                    "targets": [],
                    "default_profile": "base",
                    "force": false,
                }))
                .await;
            resp.assert_status_ok();
            let repaired: serde_json::Value = resp.json();
            assert_eq!(repaired["items"].as_array().unwrap().len(), 3);
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

#[test]
#[serial]
fn api_workspace_summary_should_expose_prd_read_model() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_prd_workspace(&temp);

    with_test_home(&temp, || {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let server = test_server(&temp, temp.child("missing-dist").path().to_path_buf());
            let resp = server.get("/api/workspace-summary").await;

            resp.assert_status_ok();
            let body: serde_json::Value = resp.json();
            assert_eq!(body["counts"]["profile_count"], serde_json::json!(2));
            assert_eq!(body["counts"]["artifact_count"], serde_json::json!(2));
            assert_eq!(body["counts"]["target_count"], serde_json::json!(3));
            assert_eq!(body["counts"]["env_set_count"], serde_json::json!(1));
            assert_eq!(body["counts"]["script_count"], serde_json::json!(1));
            assert_eq!(body["counts"]["mcp_server_count"], serde_json::json!(1));
            assert_eq!(
                body["issues"][0]["code"],
                serde_json::json!("target_profile_missing")
            );
        });
    });
}

#[test]
#[serial]
fn api_artifact_detail_should_include_targets_profiles_and_issues() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_prd_workspace(&temp);
    temp.child("proj").create_dir_all().unwrap();
    temp.child("proj/AGENTS.md")
        .write_str("Hello AgentStow from cn!")
        .unwrap();

    with_test_home(&temp, || {
        let dirs = agentstow_core::AgentStowDirs::from_env().unwrap();
        let db = agentstow_state::StateDb::open(&dirs).unwrap();
        db.upsert_link_instance(&agentstow_state::LinkInstanceRecord {
            workspace_root: temp.path().to_path_buf(),
            artifact_id: agentstow_core::ArtifactId::new_unchecked("hello"),
            profile: agentstow_core::ProfileName::new_unchecked("derived"),
            target_path: temp.child("proj/AGENTS.md").path().to_path_buf(),
            method: agentstow_core::InstallMethod::Copy,
            rendered_path: None,
            blake3: Some("abc123".to_string()),
            updated_at: time::OffsetDateTime::UNIX_EPOCH,
        })
        .unwrap();

        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let server = test_server(&temp, temp.child("missing-dist").path().to_path_buf());
            let resp = server.get("/api/artifacts/hello").await;

            resp.assert_status_ok();
            let body: serde_json::Value = resp.json();
            assert_eq!(body["artifact"]["id"], serde_json::json!("hello"));
            assert_eq!(body["targets"].as_array().unwrap().len(), 2);
            assert_eq!(body["profiles"].as_array().unwrap().len(), 1);
            assert_eq!(
                body["issues"][0]["code"],
                serde_json::json!("target_profile_missing")
            );
        });
    });
}

#[test]
#[serial]
fn api_profile_detail_should_include_merged_vars_and_artifacts() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_prd_workspace(&temp);

    with_test_home(&temp, || {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let server = test_server(&temp, temp.child("missing-dist").path().to_path_buf());
            let resp = server.get("/api/profiles/derived").await;

            resp.assert_status_ok();
            let body: serde_json::Value = resp.json();
            assert_eq!(body["profile"]["id"], serde_json::json!("derived"));
            assert_eq!(body["targets"].as_array().unwrap().len(), 1);
            assert_eq!(body["artifacts"][0]["id"], serde_json::json!("hello"));
            let merged_keys: Vec<_> = body["merged_vars"]
                .as_array()
                .unwrap()
                .iter()
                .map(|item| item["key"].as_str().unwrap())
                .collect();
            assert!(merged_keys.contains(&"name"));
            assert!(merged_keys.contains(&"region"));
        });
    });
}

#[test]
#[serial]
fn api_impact_should_filter_by_artifact_and_include_link_status() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_prd_workspace(&temp);
    temp.child("proj").create_dir_all().unwrap();
    temp.child("proj/AGENTS.md")
        .write_str("Hello AgentStow from cn!")
        .unwrap();

    with_test_home(&temp, || {
        let dirs = agentstow_core::AgentStowDirs::from_env().unwrap();
        let db = agentstow_state::StateDb::open(&dirs).unwrap();
        db.upsert_link_instance(&agentstow_state::LinkInstanceRecord {
            workspace_root: temp.path().to_path_buf(),
            artifact_id: agentstow_core::ArtifactId::new_unchecked("hello"),
            profile: agentstow_core::ProfileName::new_unchecked("derived"),
            target_path: temp.child("proj/AGENTS.md").path().to_path_buf(),
            method: agentstow_core::InstallMethod::Copy,
            rendered_path: None,
            blake3: Some("abc123".to_string()),
            updated_at: time::OffsetDateTime::UNIX_EPOCH,
        })
        .unwrap();

        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let server = test_server(&temp, temp.child("missing-dist").path().to_path_buf());
            let resp = server
                .get("/api/impact")
                .add_query_param("artifact", "hello")
                .await;

            resp.assert_status_ok();
            let body: serde_json::Value = resp.json();
            assert_eq!(body["subject_kind"], serde_json::json!("artifact"));
            assert_eq!(body["affected_targets"].as_array().unwrap().len(), 2);
            assert_eq!(body["affected_artifacts"].as_array().unwrap().len(), 1);
            assert_eq!(body["link_status"].as_array().unwrap().len(), 1);
        });
    });
}
