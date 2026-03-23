use assert_cmd::Command;
use assert_fs::prelude::*;
use predicates::prelude::*;
use pretty_assertions::assert_eq;

#[test]
fn render_dry_run_should_print_rendered_text() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("artifacts").create_dir_all().unwrap();
    temp.child("artifacts/hello.txt.tera")
        .write_str("Hello {{ name }}!")
        .unwrap();

    temp.child("agentstow.toml")
        .write_str(
            r#"
[profiles.base]
vars = { name = "CLI" }

[artifacts.hello]
kind = "file"
source = "artifacts/hello.txt.tera"
template = true
validate_as = "none"
"#,
        )
        .unwrap();

    let mut cmd = Command::cargo_bin("agentstow").unwrap();
    cmd.arg("--cwd")
        .arg(temp.path())
        .arg("--profile")
        .arg("base")
        .arg("render")
        .arg("--artifact")
        .arg("hello")
        .arg("--dry-run");

    cmd.assert().success().stdout("Hello CLI!");
}

#[test]
fn render_dir_out_should_materialize_rendered_directory() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("templates/agents/skills")
        .create_dir_all()
        .unwrap();
    temp.child("templates/agents/worker.toml.tera")
        .write_str("name = \"{{ workspace }}\"")
        .unwrap();
    temp.child("templates/agents/skills/rule.md")
        .write_str("Keep builds reproducible.")
        .unwrap();

    temp.child("agentstow.toml")
        .write_str(
            r#"
[profiles.base]
workspace = "codex-lab"

[artifacts.agents_dir]
kind = "dir"
source = "templates/agents"
template = true
"#,
        )
        .unwrap();

    let mut cmd = Command::cargo_bin("agentstow").unwrap();
    cmd.arg("--cwd")
        .arg(temp.path())
        .arg("--profile")
        .arg("base")
        .arg("render")
        .arg("--artifact")
        .arg("agents_dir")
        .arg("--out")
        .arg(temp.child("rendered/.agents").path());

    cmd.assert().success();
    assert_eq!(
        std::fs::read_to_string(temp.child("rendered/.agents/worker.toml").path()).unwrap(),
        "name = \"codex-lab\""
    );
    assert_eq!(
        std::fs::read_to_string(temp.child("rendered/.agents/skills/rule.md").path()).unwrap(),
        "Keep builds reproducible."
    );
}

#[test]
fn render_should_support_real_example_style_contexts() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("artifacts").create_dir_all().unwrap();
    temp.child("artifacts/hello.txt.tera")
        .write_str(
            "owner={{ env_files.shared.OWNER }}\nref={{ files.reference }}\ncmd={{ mcp_servers.filesystem.mcpServers.filesystem.command }}\n",
        )
        .unwrap();
    temp.child(".env")
        .write_str("OWNER=platform-team\n")
        .unwrap();
    temp.child("reference.md")
        .write_str("reference-fragment")
        .unwrap();
    temp.child("mcps.json")
        .write_str(
            r#"{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "."]
    }
  }
}"#,
        )
        .unwrap();

    temp.child("agentstow.toml")
        .write_str(
            r#"
[profiles.base]
vars = { name = "CLI" }

[env.files]
paths = [".env"]

[files.reference]
path = "reference.md"

[mcp_servers.file]
path = "mcps.json"

[artifacts.hello]
kind = "file"
source = "artifacts/hello.txt.tera"
template = true
validate_as = "none"
"#,
        )
        .unwrap();

    let mut cmd = Command::cargo_bin("agentstow").unwrap();
    cmd.arg("--cwd")
        .arg(temp.path())
        .arg("--profile")
        .arg("base")
        .arg("render")
        .arg("--artifact")
        .arg("hello")
        .arg("--dry-run");

    cmd.assert().success().stdout(predicate::str::contains(
        "owner=platform-team\nref=reference-fragment\ncmd=npx",
    ));
}

#[test]
fn link_plan_json_should_be_machine_readable() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("artifacts").create_dir_all().unwrap();
    temp.child("artifacts/hello.txt.tera")
        .write_str("Hello {{ name }}!")
        .unwrap();
    temp.child("proj").create_dir_all().unwrap();

    temp.child("agentstow.toml")
        .write_str(
            r#"
[profiles.base]
vars = { name = "Link" }

[artifacts.hello]
kind = "file"
source = "artifacts/hello.txt.tera"
template = true
validate_as = "none"

[targets.out]
artifact = "hello"
profile = "base"
target_path = "proj/out.txt"
method = "copy"
"#,
        )
        .unwrap();

    let mut cmd = Command::cargo_bin("agentstow").unwrap();
    cmd.arg("--cwd")
        .arg(temp.path())
        .arg("--json")
        .arg("link")
        .arg("--plan");

    let assert = cmd.assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v.as_array().unwrap().len(), 1);
}

#[test]
fn link_apply_copy_should_write_target_and_record_state() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("artifacts").create_dir_all().unwrap();
    temp.child("artifacts/hello.txt.tera")
        .write_str("Hello {{ name }}!")
        .unwrap();
    temp.child("proj").create_dir_all().unwrap();
    temp.child("home").create_dir_all().unwrap();

    temp.child("agentstow.toml")
        .write_str(
            r#"
[profiles.base]
vars = { name = "State" }

[artifacts.hello]
kind = "file"
source = "artifacts/hello.txt.tera"
template = true
validate_as = "none"

[targets.out]
artifact = "hello"
profile = "base"
target_path = "proj/out.txt"
method = "copy"
"#,
        )
        .unwrap();

    let mut cmd = Command::cargo_bin("agentstow").unwrap();
    cmd.arg("--cwd")
        .arg(temp.path())
        .env("AGENTSTOW_HOME", temp.child("home").path())
        .arg("link");
    cmd.assert().success();

    let text = std::fs::read_to_string(temp.child("proj/out.txt").path()).unwrap();
    assert_eq!(text, "Hello State!");

    assert!(temp.child("home/data/agentstow.db").path().exists());
}

#[test]
fn link_should_preflight_all_targets_before_mutation() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("artifacts").create_dir_all().unwrap();
    temp.child("artifacts/ok.txt.tera")
        .write_str("ok-{{ name }}")
        .unwrap();
    temp.child("artifacts/conflict.txt.tera")
        .write_str("conflict-{{ name }}")
        .unwrap();
    temp.child("proj").create_dir_all().unwrap();
    temp.child("proj/conflict.txt")
        .write_str("user-owned")
        .unwrap();

    temp.child("agentstow.toml")
        .write_str(
            r#"
[profiles.base]
vars = { name = "demo" }

[artifacts.ok]
kind = "file"
source = "artifacts/ok.txt.tera"
template = true
validate_as = "none"

[artifacts.conflict]
kind = "file"
source = "artifacts/conflict.txt.tera"
template = true
validate_as = "none"

[targets.ok]
artifact = "ok"
profile = "base"
target_path = "proj/ok.txt"
method = "copy"

[targets.conflict]
artifact = "conflict"
profile = "base"
target_path = "proj/conflict.txt"
method = "copy"
"#,
        )
        .unwrap();

    let mut cmd = Command::cargo_bin("agentstow").unwrap();
    cmd.arg("--cwd").arg(temp.path()).arg("link");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("target 已存在且内容不同"));
    assert!(!temp.child("proj/ok.txt").path().exists());
    assert_eq!(
        std::fs::read_to_string(temp.child("proj/conflict.txt").path()).unwrap(),
        "user-owned"
    );
}

#[test]
fn render_with_json_should_emit_structured_error_for_invalid_args() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("agentstow.toml")
        .write_str(
            r#"
[profiles.base]
vars = { name = "CLI" }

[artifacts.hello]
kind = "file"
source = "artifacts/hello.txt.tera"
template = false
validate_as = "none"
"#,
        )
        .unwrap();
    temp.child("artifacts").create_dir_all().unwrap();
    temp.child("artifacts/hello.txt.tera")
        .write_str("hello")
        .unwrap();

    let mut cmd = Command::cargo_bin("agentstow").unwrap();
    cmd.arg("--cwd")
        .arg(temp.path())
        .arg("--json")
        .arg("render")
        .arg("--artifact")
        .arg("bad/name");

    cmd.assert().failure().code(2).stdout(
        predicate::str::contains("\"error\"").and(predicate::str::contains("\"exit_code\": 2")),
    );
}

#[test]
fn scripts_run_should_honor_global_timeout_override() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("agentstow.toml")
        .write_str(
            r#"
[scripts.sleepy]
kind = "shell"
entry = "bash"
args = ["-lc", "sleep 1"]
cwd_policy = "current"
stdin_mode = "none"
stdout_mode = "capture"
stderr_mode = "capture"
timeout_ms = 5000
expected_exit_codes = [0]
"#,
        )
        .unwrap();

    let mut cmd = Command::cargo_bin("agentstow").unwrap();
    cmd.arg("--cwd")
        .arg(temp.path())
        .arg("--timeout")
        .arg("50")
        .arg("scripts")
        .arg("run")
        .arg("--id")
        .arg("sleepy");

    cmd.assert()
        .failure()
        .code(7)
        .stderr(predicate::str::contains("脚本超时（50ms）"));
}

#[test]
fn link_status_should_report_copy_dir_as_healthy() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("artifacts/skills").create_dir_all().unwrap();
    temp.child("artifacts/skills/rule.md")
        .write_str("hello")
        .unwrap();
    temp.child("proj").create_dir_all().unwrap();
    temp.child("home").create_dir_all().unwrap();

    temp.child("agentstow.toml")
        .write_str(
            r#"
[profiles.base]
vars = { name = "Dir" }

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

    let mut link = Command::cargo_bin("agentstow").unwrap();
    link.arg("--cwd")
        .arg(temp.path())
        .env("AGENTSTOW_HOME", temp.child("home").path())
        .arg("link");
    link.assert().success();

    let mut status = Command::cargo_bin("agentstow").unwrap();
    status
        .arg("--cwd")
        .arg(temp.path())
        .env("AGENTSTOW_HOME", temp.child("home").path())
        .arg("link")
        .arg("status");

    status
        .assert()
        .success()
        .stdout(predicate::str::contains("[ok]").and(predicate::str::contains(".agents/skills")));
}

#[test]
fn link_status_should_report_symlink_targets_as_healthy() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("artifacts").create_dir_all().unwrap();
    temp.child("artifacts/hello.txt.tera")
        .write_str("Hello {{ name }}!")
        .unwrap();
    temp.child("proj").create_dir_all().unwrap();
    temp.child("home").create_dir_all().unwrap();

    temp.child("agentstow.toml")
        .write_str(
            r#"
[profiles.base]
vars = { name = "Symlink" }

[artifacts.hello]
kind = "file"
source = "artifacts/hello.txt.tera"
template = true
validate_as = "none"

[targets.out]
artifact = "hello"
profile = "base"
target_path = "proj/out.txt"
method = "symlink"
"#,
        )
        .unwrap();

    let mut link = Command::cargo_bin("agentstow").unwrap();
    link.arg("--cwd")
        .arg(temp.path())
        .env("AGENTSTOW_HOME", temp.child("home").path())
        .arg("link");
    link.assert().success();

    let mut status = Command::cargo_bin("agentstow").unwrap();
    status
        .arg("--cwd")
        .arg(temp.path())
        .env("AGENTSTOW_HOME", temp.child("home").path())
        .arg("link")
        .arg("status");

    status
        .assert()
        .success()
        .stdout(predicate::str::contains("[ok]").and(predicate::str::contains("proj/out.txt")));
}

#[test]
fn link_should_support_rendered_dir_artifact_as_first_class_symlink() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("templates/agents/skills")
        .create_dir_all()
        .unwrap();
    temp.child("templates/agents/worker.toml.tera")
        .write_str("name = \"{{ workspace }}\"")
        .unwrap();
    temp.child("templates/agents/skills/rule.md")
        .write_str("Keep builds reproducible.")
        .unwrap();
    temp.child("proj").create_dir_all().unwrap();
    temp.child("home").create_dir_all().unwrap();

    temp.child("agentstow.toml")
        .write_str(
            r#"
[profiles.base]
workspace = "codex-lab"

[artifacts.agents_dir]
kind = "dir"
source = "templates/agents"
template = true

[targets.agents]
artifact = "agents_dir"
profile = "base"
target_path = "proj/.agents"
method = "symlink"
"#,
        )
        .unwrap();

    let mut link = Command::cargo_bin("agentstow").unwrap();
    link.arg("--cwd")
        .arg(temp.path())
        .env("AGENTSTOW_HOME", temp.child("home").path())
        .arg("link");
    link.assert().success();

    let metadata = std::fs::symlink_metadata(temp.child("proj/.agents").path()).unwrap();
    assert!(metadata.file_type().is_symlink());
    assert_eq!(
        std::fs::read_to_string(temp.child("proj/.agents/worker.toml").path()).unwrap(),
        "name = \"codex-lab\""
    );
    assert_eq!(
        std::fs::read_to_string(temp.child("proj/.agents/skills/rule.md").path()).unwrap(),
        "Keep builds reproducible."
    );
}

#[test]
fn workspace_init_should_create_manifest_and_sample_artifact() {
    let temp = assert_fs::TempDir::new().unwrap();
    let ws = temp.child("ws");

    let mut cmd = Command::cargo_bin("agentstow").unwrap();
    cmd.arg("--cwd")
        .arg(temp.path())
        .arg("--workspace")
        .arg(ws.path())
        .arg("workspace")
        .arg("init");

    cmd.assert().success();

    assert!(ws.child("agentstow.toml").path().exists());
    assert!(ws.child("artifacts/hello.txt.tera").path().exists());
}
