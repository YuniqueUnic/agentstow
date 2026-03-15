use std::collections::BTreeMap;

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

#[test]
fn render_server_json_should_only_include_selected_server() {
    let server = agentstow_manifest::McpServerDef {
        transport: agentstow_manifest::McpTransport::Stdio {
            command: "npx".to_string(),
            args: vec!["demo-mcp".to_string(), "--stdio".to_string()],
        },
        env: vec![],
    };

    let json = Mcp::render_server_json("demo", &server).unwrap();
    assert!(json.contains("\"demo\""));
    assert!(json.contains("\"command\": \"npx\""));
    assert!(!json.contains("\"other\""));
}

#[test]
fn launcher_preview_should_render_stdio_command_line() {
    let server = agentstow_manifest::McpServerDef {
        transport: agentstow_manifest::McpTransport::Stdio {
            command: "npx".to_string(),
            args: vec!["demo mcp".to_string(), "--stdio".to_string()],
        },
        env: vec![],
    };

    let preview = Mcp::launcher_preview(&server);
    assert_eq!(preview, "npx \"demo mcp\" --stdio");
}

#[test]
fn test_server_dry_run_should_include_render_check() {
    let server = agentstow_manifest::McpServerDef {
        transport: agentstow_manifest::McpTransport::Http {
            url: "https://example.com/mcp".to_string(),
            headers: std::collections::HashMap::new(),
        },
        env: vec![],
    };

    let checks = Mcp::test_server_dry_run("demo", &server);
    assert!(checks.iter().any(|check| check.code == "validate"));
    assert!(checks.iter().any(|check| check.code == "render"));
    assert!(checks.iter().any(|check| check.code == "headers"));
}
