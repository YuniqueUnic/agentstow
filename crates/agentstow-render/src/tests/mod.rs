use assert_fs::prelude::*;
use pretty_assertions::assert_eq;

use super::*;

#[test]
fn render_tera_template_should_work() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("artifacts").create_dir_all().unwrap();
    temp.child("artifacts/hello.txt.tera")
        .write_str("Hello {{ name }}!")
        .unwrap();

    temp.child("agentstow.toml")
        .write_str(
            r#"
[profiles.base]
vars = { name = "AgentStow" }

[artifacts.hello]
kind = "file"
source = "artifacts/hello.txt.tera"
template = true
validate_as = "none"
"#,
        )
        .unwrap();

    let manifest = Manifest::load_from_path(temp.child("agentstow.toml").path()).unwrap();
    let out = Renderer::render_file(
        &manifest,
        &ArtifactId::new_unchecked("hello"),
        &ProfileName::new_unchecked("base"),
    )
    .unwrap();

    assert_eq!(String::from_utf8(out.bytes).unwrap(), "Hello AgentStow!");
}

#[test]
fn render_should_fail_when_template_variable_is_missing() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("artifacts").create_dir_all().unwrap();
    temp.child("artifacts/hello.txt.tera")
        .write_str("Hello {{ missing_name }}!")
        .unwrap();

    temp.child("agentstow.toml")
        .write_str(
            r#"
[profiles.base]
vars = { name = "AgentStow" }

[artifacts.hello]
kind = "file"
source = "artifacts/hello.txt.tera"
template = true
validate_as = "none"
"#,
        )
        .unwrap();

    let manifest = Manifest::load_from_path(temp.child("agentstow.toml").path()).unwrap();
    let err = Renderer::render_file(
        &manifest,
        &ArtifactId::new_unchecked("hello"),
        &ProfileName::new_unchecked("base"),
    )
    .unwrap_err();

    assert_eq!(err.exit_code(), agentstow_core::ExitCode::InvalidConfig);
    assert!(err.to_string().contains("missing_name"));
}

#[test]
fn render_should_include_env_file_inline_env_file_contexts_and_mcp_contexts() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("artifacts").create_dir_all().unwrap();
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
    temp.child("artifacts/hello.txt.tera")
        .write_str(
            "owner={{ env.OWNER }}\ndirect={{ env.DIRECT_ENV }}\nduplicate={{ env.DUPLICATE_ENV }}\nref={{ file.reference }}\ncmd={{ mcp_servers.filesystem.mcpServers.filesystem.command }}\n",
        )
        .unwrap();
    temp.child(".env")
        .write_str("OWNER=platform-team\nDUPLICATE_ENV=from-dotenv\n")
        .unwrap();
    temp.child("reference.md")
        .write_str("reference-fragment")
        .unwrap();

    temp.child("agentstow.toml")
        .write_str(
            r#"
[profiles.base]
vars = { name = "AgentStow" }

[env]
DIRECT_ENV = "from-inline"
DUPLICATE_ENV = "from-manifest"

[artifacts.hello]
kind = "file"
source = "artifacts/hello.txt.tera"
template = true
validate_as = "none"

[env.files]
paths = [".env"]

[file.reference]
path = "reference.md"

[mcp_servers.file]
path = "mcps.json"
"#,
        )
        .unwrap();

    let manifest = Manifest::load_from_path(temp.child("agentstow.toml").path()).unwrap();
    let out = Renderer::render_file(
        &manifest,
        &ArtifactId::new_unchecked("hello"),
        &ProfileName::new_unchecked("base"),
    )
    .unwrap();

    let text = String::from_utf8(out.bytes).unwrap();
    assert!(text.contains("owner=platform-team"));
    assert!(text.contains("direct=from-inline"));
    assert!(text.contains("duplicate=from-manifest"));
    assert!(text.contains("ref=reference-fragment"));
    assert!(text.contains("cmd=npx"));
}
