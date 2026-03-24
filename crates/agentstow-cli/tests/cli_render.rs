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
            "owner={{ env.OWNER }}\ndirect={{ env.DIRECT_ENV }}\nduplicate={{ env.DUPLICATE_ENV }}\nref={{ file.reference }}\njson={{ mcp_servers.filesystem | trim }}\ntoml={{ mcp_servers.filesystem | trim | toml }}\nyaml={{ mcp_servers.filesystem | trim | yaml }}\ncodex_json={{ mcp_servers.filesystem | trim | codex | json }}\n",
        )
        .unwrap();
    temp.child(".env")
        .write_str("OWNER=platform-team\nDUPLICATE_ENV=from-dotenv\n")
        .unwrap();
    temp.child("reference.md")
        .write_str("reference-fragment")
        .unwrap();
    temp.child("mcps.json")
        .write_str(
            r#"{
  "mcpServers": {
    "filesystem": {
      "transport": {
        "kind": "stdio",
        "command": "npx",
        "args": ["-y", "@modelcontextprotocol/server-filesystem", "."]
      },
      "env": [
        { "key": "NODE_ENV", "binding": { "kind": "literal", "value": "production" } }
      ]
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

[env]
DIRECT_ENV = "from-inline"
DUPLICATE_ENV = "from-manifest"

[env.files]
paths = [".env"]

[file.reference]
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
        "owner=platform-team\ndirect=from-inline\nduplicate=from-manifest\nref=reference-fragment\njson={\n  \"mcpServers\": {\n    \"filesystem\": {\n      \"transport\": {\n        \"kind\": \"stdio\"",
    ));
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
