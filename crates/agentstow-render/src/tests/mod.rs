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
      "transport": {
        "kind": "stdio",
        "command": "npx",
        "args": ["-y", "@modelcontextprotocol/server-filesystem", "."]
      },
      "env": [
        { "key": "NODE_ENV", "binding": { "kind": "literal", "value": "production" } }
      ],
      "options": {
        "startup_timeout_sec": 20,
        "tool_timeout_sec": 45,
        "enabled_tools": ["read", "write"],
        "timeout": 30000,
        "trust": true,
        "include_tools": ["read"],
        "oauth": {
          "client_id": "claude-client",
          "callback_port": 4317,
          "auth_server_metadata_url": "https://auth.example.com/.well-known/openid-configuration",
          "scopes": ["https://www.googleapis.com/auth/cloud-platform"]
        }
      }
    }
  }
}"#,
        )
        .unwrap();
    temp.child("artifacts/hello.txt.tera")
        .write_str(
            "owner={{ env.OWNER }}\ndirect={{ env.DIRECT_ENV }}\nduplicate={{ env.DUPLICATE_ENV }}\nref={{ file.reference }}\njson={{ mcp_servers.filesystem | trim }}\ntoml={{ mcp_servers.filesystem | trim | toml }}\nyaml={{ mcp_servers.filesystem | trim | yaml }}\ncodex_json={{ mcp_servers.remote | trim | codex | json }}\ncodex_toml={{ mcp_servers.remote | trim | codex | toml }}\nclaude_json={{ mcp_servers.remote | trim | claude | json }}\ngemini_toml={{ mcp_servers.remote | trim | gemini | toml }}\n",
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

[mcp_servers.remote]
transport = { kind = "http", url = "https://example.com/mcp", headers = { Accept = "application/json" } }
env = [
  { key = "X-Workspace-Token", binding = { kind = "env", var = "WORKSPACE_TOKEN" } }
]

[mcp_servers.remote.options]
startup_timeout_sec = 20
tool_timeout_sec = 45
timeout = 30000
trust = true
include_tools = ["read"]

[mcp_servers.remote.options.oauth]
client_id = "claude-client"
callback_port = 4317
auth_server_metadata_url = "https://auth.example.com/.well-known/openid-configuration"
scopes = ["https://www.googleapis.com/auth/cloud-platform"]
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
    assert!(text.contains("\"transport\": {\n        \"kind\": \"stdio\""));
    assert!(text.contains("\"options\": {\n        \"startup_timeout_sec\": 20"));
    assert!(text.contains("toml=[mcp_servers.filesystem]"));
    assert!(text.contains("[mcp_servers.filesystem.options]"));
    assert!(text.contains("[mcp_servers.filesystem.transport]"));
    assert!(text.contains("yaml=mcpServers:"));
    assert!(text.contains(
        "codex_json={\n  \"mcpServers\": {\n    \"remote\": {\n      \"url\": \"https://example.com/mcp\""
    ));
    assert!(text.contains("codex_toml=[mcp_servers.remote]"));
    assert!(text.contains("startup_timeout_sec = 20"));
    assert!(text.contains(
        "claude_json={\n  \"mcpServers\": {\n    \"remote\": {\n      \"type\": \"http\""
    ));
    assert!(text.contains("\"clientId\": \"claude-client\""));
    assert!(text.contains("gemini_toml=[mcp_servers.remote]"));
    assert!(text.contains("timeout = 30000"));
    assert!(text.contains("trust = true"));
    assert!(!text.contains("env_vars = []"));
}
