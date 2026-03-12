use std::collections::BTreeMap;

use pretty_assertions::assert_eq;

use super::*;

#[test]
fn render_mcp_json_should_use_env_placeholder() {
    let mut servers = BTreeMap::new();
    servers.insert(
        "demo".to_string(),
        agentstow_manifest::McpServerDef {
            transport: agentstow_manifest::McpTransport::Stdio {
                command: "echo".to_string(),
                args: vec!["ok".to_string()],
            },
            env: vec![agentstow_manifest::EnvVarDef {
                key: "TOKEN".to_string(),
                binding: agentstow_core::SecretBinding::Env {
                    var: "TOKEN".to_string(),
                },
            }],
        },
    );

    let json = Mcp::render_mcp_json(&servers).unwrap();
    assert!(json.contains("\"TOKEN\": \"${TOKEN}\""));
}
