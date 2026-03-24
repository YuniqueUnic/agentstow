use assert_cmd::Command;
use assert_fs::prelude::*;
use predicates::prelude::*;

#[test]
fn env_emit_without_set_should_export_merged_env_context() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child(".env")
        .write_str("OWNER=platform-team\nDUPLICATE_ENV=from-dotenv\n")
        .unwrap();
    temp.child("agentstow.toml")
        .write_str(
            r#"
[env]
DIRECT_ENV = "from-inline"
DUPLICATE_ENV = "from-manifest"

[env.files]
paths = [".env"]
"#,
        )
        .unwrap();

    let mut cmd = Command::cargo_bin("agentstow").unwrap();
    cmd.arg("--cwd")
        .arg(temp.path())
        .arg("env")
        .arg("emit")
        .arg("--shell")
        .arg("bash")
        .arg("--stdout");

    cmd.assert().success().stdout(predicate::str::contains(
        "export DUPLICATE_ENV='from-manifest'",
    ));
}

#[test]
fn env_emit_with_set_should_export_named_env_emit_set() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("agentstow.toml")
        .write_str(
            r#"
[env.emit.default]
vars = [
  { key = "OPENAI_API_KEY", binding = { kind = "env", var = "OPENAI_API_KEY" } }
]
"#,
        )
        .unwrap();

    let mut cmd = Command::cargo_bin("agentstow").unwrap();
    cmd.arg("--cwd")
        .arg(temp.path())
        .env("OPENAI_API_KEY", "token123")
        .arg("env")
        .arg("emit")
        .arg("--set")
        .arg("default")
        .arg("--shell")
        .arg("bash")
        .arg("--stdout");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("export OPENAI_API_KEY='token123'"));
}
