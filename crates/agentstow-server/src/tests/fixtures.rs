use std::future::Future;

use assert_fs::prelude::*;
use axum_test::TestServer;

use crate::{WatchStatusHandle, WatchStatusSnapshot, build_app_with_ui_dist_and_watch};

pub(super) fn write_minimal_workspace(temp: &assert_fs::TempDir) {
    temp.child("artifacts").create_dir_all().unwrap();
    temp.child("artifacts/hello.txt.tera")
        .write_str("Hello {{ name }}!")
        .unwrap();
    temp.child("agentstow.toml")
        .write_str(
            r#"
[profiles.base]
name = "Server"

[artifacts.hello]
kind = "file"
source = "artifacts/hello.txt.tera"
template = true
validate_as = "none"
"#,
        )
        .unwrap();
}

pub(super) fn write_prd_workspace(temp: &assert_fs::TempDir) {
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
name = "AgentStow"
region = "global"

[profiles.derived]
extends = ["base"]
region = "cn"

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

[env.emit.default]
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

pub(super) fn write_script_workspace(temp: &assert_fs::TempDir) {
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

pub(super) fn write_http_mcp_workspace(temp: &assert_fs::TempDir) {
    temp.child("agentstow.toml")
        .write_str(
            r#"
[mcp_servers.remote]
transport = { kind = "http", url = "https://example.com/mcp", headers = { X-Workspace = "agentstow" } }
env = [
  { key = "OPENAI_API_KEY", binding = { kind = "env", var = "OPENAI_API_KEY" } }
]
"#,
        )
        .unwrap();
}

pub(super) fn init_git_repo(temp: &assert_fs::TempDir) {
    git_stdout(temp, &["init"]);
    git_stdout(temp, &["config", "user.name", "AgentStow Tests"]);
    git_stdout(
        temp,
        &["config", "user.email", "agentstow-tests@example.com"],
    );
    git_stdout(temp, &["add", "."]);
    git_stdout(temp, &["commit", "-m", "initial workspace"]);
}

pub(super) fn git_stdout(temp: &assert_fs::TempDir, args: &[&str]) -> String {
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

pub(super) fn with_test_home(temp: &assert_fs::TempDir, f: impl FnOnce()) {
    temp.child("home").create_dir_all().unwrap();
    temp_env::with_var("AGENTSTOW_HOME", Some(temp.child("home").path()), f);
}

pub(super) fn write_ui_dist(temp: &assert_fs::TempDir) -> std::path::PathBuf {
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

pub(super) fn default_watch_status(temp: &assert_fs::TempDir) -> WatchStatusSnapshot {
    WatchStatusSnapshot::manual(vec![temp.path().display().to_string()], None)
}

pub(super) fn test_server(
    temp: &assert_fs::TempDir,
    ui_dist_dir: std::path::PathBuf,
) -> TestServer {
    test_server_with_watch(temp, ui_dist_dir, default_watch_status(temp))
}

pub(super) fn test_server_with_watch(
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

pub(super) fn test_server_unconfigured(ui_dist_dir: std::path::PathBuf) -> TestServer {
    TestServer::new(build_app_with_ui_dist_and_watch(
        None,
        ui_dist_dir,
        WatchStatusHandle::manual(Vec::new(), Some("workspace 未选择".to_string())),
    ))
}

pub(super) fn block_on<F>(future: F)
where
    F: Future<Output = ()>,
{
    tokio::runtime::Runtime::new().unwrap().block_on(future);
}

pub(super) fn upsert_link_instance(record: &agentstow_state::LinkInstanceRecord) {
    let dirs = agentstow_core::AgentStowDirs::from_env().unwrap();
    let db = agentstow_state::StateDb::open(&dirs).unwrap();
    db.upsert_link_instance(record).unwrap();
}
