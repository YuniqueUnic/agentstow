use std::collections::{BTreeMap, HashMap};

use super::*;

#[test]
fn render_mcp_json_should_use_env_vars_for_forwarded_stdio_env() {
    let mut servers = BTreeMap::new();
    servers.insert(
        "demo".to_string(),
        agentstow_manifest::McpServerDef {
            transport: agentstow_manifest::McpTransport::Stdio {
                command: "echo".to_string(),
                args: vec!["ok".to_string()],
                cwd: None,
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
    assert!(json.contains("\"env_vars\": ["));
    assert!(json.contains("\"TOKEN\""));
}

#[test]
fn render_generic_server_snippet_should_preserve_transport_shape() {
    let server = agentstow_manifest::McpServerDef {
        transport: agentstow_manifest::McpTransport::Stdio {
            command: "npx".to_string(),
            args: vec!["demo-mcp".to_string(), "--stdio".to_string()],
            cwd: None,
        },
        env: vec![agentstow_manifest::EnvVarDef {
            key: "NODE_ENV".to_string(),
            binding: agentstow_core::SecretBinding::Literal {
                value: "production".to_string(),
            },
        }],
    };

    let json = Mcp::render_generic_server_snippet("demo", &server, McpSnippetFormat::Json).unwrap();
    assert!(json.contains("\"transport\": {"));
    assert!(json.contains("\"kind\": \"stdio\""));
    assert!(json.contains("\"NODE_ENV\""));
}

#[test]
fn render_server_json_should_only_include_selected_server() {
    let server = agentstow_manifest::McpServerDef {
        transport: agentstow_manifest::McpTransport::Stdio {
            command: "npx".to_string(),
            args: vec!["demo-mcp".to_string(), "--stdio".to_string()],
            cwd: None,
        },
        env: vec![],
    };

    let json = Mcp::render_server_json("demo", &server).unwrap();
    assert!(json.contains("\"demo\""));
    assert!(json.contains("\"command\": \"npx\""));
    assert!(!json.contains("\"other\""));
}

#[test]
fn render_server_toml_should_emit_full_embeddable_block() {
    let server = agentstow_manifest::McpServerDef {
        transport: agentstow_manifest::McpTransport::Stdio {
            command: "npx".to_string(),
            args: vec!["demo-mcp".to_string(), "--stdio".to_string()],
            cwd: None,
        },
        env: vec![agentstow_manifest::EnvVarDef {
            key: "TOKEN".to_string(),
            binding: agentstow_core::SecretBinding::Env {
                var: "TOKEN".to_string(),
            },
        }],
    };

    let rendered = Mcp::render_server_toml("demo_server", &server).unwrap();
    assert!(rendered.contains("[mcp_servers.demo-server]"));
    assert!(rendered.contains("args = [\"demo-mcp\", \"--stdio\"]"));
    assert!(rendered.contains("command = \"npx\""));
    assert!(rendered.contains("env_vars = [\"TOKEN\"]"));
}

#[test]
fn adapt_server_snippet_to_codex_should_map_http_env_to_bearer_token() {
    let server = agentstow_manifest::McpServerDef {
        transport: agentstow_manifest::McpTransport::Http {
            url: "https://example.com/mcp".to_string(),
            headers: HashMap::from([("Accept".to_string(), "application/json".to_string())]),
        },
        env: vec![agentstow_manifest::EnvVarDef {
            key: "TAVILY_API_KEY".to_string(),
            binding: agentstow_core::SecretBinding::Env {
                var: "TAVILY_API_KEY".to_string(),
            },
        }],
    };

    let generic_json =
        Mcp::render_generic_server_snippet("demo", &server, McpSnippetFormat::Json).unwrap();
    let codex_yaml = Mcp::adapt_server_snippet(
        &generic_json,
        McpTargetAdapter::Codex,
        Some(McpSnippetFormat::Yaml),
    )
    .unwrap();

    assert!(codex_yaml.contains("url: https://example.com/mcp"));
    assert!(codex_yaml.contains("bearer_token_env_var: TAVILY_API_KEY"));
    assert!(codex_yaml.contains("http_headers:"));
}

#[test]
fn adapt_server_snippet_to_codex_should_map_renamed_http_env_to_env_http_headers() {
    let server = agentstow_manifest::McpServerDef {
        transport: agentstow_manifest::McpTransport::Http {
            url: "https://example.com/mcp".to_string(),
            headers: HashMap::new(),
        },
        env: vec![agentstow_manifest::EnvVarDef {
            key: "X-Workspace-Token".to_string(),
            binding: agentstow_core::SecretBinding::Env {
                var: "WORKSPACE_TOKEN".to_string(),
            },
        }],
    };

    let generic_json =
        Mcp::render_generic_server_snippet("demo", &server, McpSnippetFormat::Json).unwrap();
    let codex_json =
        Mcp::adapt_server_snippet(&generic_json, McpTargetAdapter::Codex, None).unwrap();
    assert!(codex_json.contains("\"env_http_headers\": {"));
    assert!(codex_json.contains("\"X-Workspace-Token\": \"WORKSPACE_TOKEN\""));
}

#[test]
fn validate_server_should_reject_http_literal_env() {
    let server = agentstow_manifest::McpServerDef {
        transport: agentstow_manifest::McpTransport::Http {
            url: "https://example.com/mcp".to_string(),
            headers: HashMap::new(),
        },
        env: vec![agentstow_manifest::EnvVarDef {
            key: "OPENAI_API_KEY".to_string(),
            binding: agentstow_core::SecretBinding::Literal {
                value: "secret".to_string(),
            },
        }],
    };

    let err = Mcp::validate_server("demo", &server).unwrap_err();
    assert!(err.to_string().contains("HTTP env 不支持 literal"));
}

#[test]
fn launcher_preview_should_render_stdio_command_line() {
    let server = agentstow_manifest::McpServerDef {
        transport: agentstow_manifest::McpTransport::Stdio {
            command: "npx".to_string(),
            args: vec!["demo mcp".to_string(), "--stdio".to_string()],
            cwd: None,
        },
        env: vec![],
    };

    let preview = Mcp::launcher_preview("demo", &server);
    assert_eq!(preview, "npx \"demo mcp\" --stdio");
}

#[test]
fn test_server_dry_run_should_include_render_check() {
    let server = agentstow_manifest::McpServerDef {
        transport: agentstow_manifest::McpTransport::Http {
            url: "https://example.com/mcp".to_string(),
            headers: HashMap::from([("X-Workspace".to_string(), "agentstow".to_string())]),
        },
        env: vec![agentstow_manifest::EnvVarDef {
            key: "OPENAI_API_KEY".to_string(),
            binding: agentstow_core::SecretBinding::Env {
                var: "OPENAI_API_KEY".to_string(),
            },
        }],
    };

    let checks = Mcp::test_server_dry_run("demo", &server);
    assert!(checks.iter().any(|check| check.code == "validate"));
    assert!(checks.iter().any(|check| check.code == "render"));
    assert!(checks.iter().any(|check| check.code == "headers"));
}
