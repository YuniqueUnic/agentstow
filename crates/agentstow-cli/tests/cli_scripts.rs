use assert_cmd::Command;
use assert_fs::prelude::*;
use predicates::prelude::*;

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
