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
            options: Default::default(),
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
        options: Default::default(),
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
        options: Default::default(),
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
        options: Default::default(),
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
        options: Default::default(),
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
        options: Default::default(),
    };

    let generic_json =
        Mcp::render_generic_server_snippet("demo", &server, McpSnippetFormat::Json).unwrap();
    let codex_json =
        Mcp::adapt_server_snippet(&generic_json, McpTargetAdapter::Codex, None).unwrap();
    assert!(codex_json.contains("\"env_http_headers\": {"));
    assert!(codex_json.contains("\"X-Workspace-Token\": \"WORKSPACE_TOKEN\""));
}

#[test]
fn adapt_server_snippet_to_codex_should_render_provider_options() {
    let server = agentstow_manifest::McpServerDef {
        transport: agentstow_manifest::McpTransport::Stdio {
            command: "npx".to_string(),
            args: vec!["demo-mcp".to_string()],
            cwd: None,
        },
        env: vec![],
        options: agentstow_manifest::McpServerOptions {
            startup_timeout_sec: Some(20),
            tool_timeout_sec: Some(45),
            enabled: Some(true),
            required: Some(true),
            enabled_tools: vec!["forecast".to_string()],
            disabled_tools: vec!["screenshot".to_string()],
            ..Default::default()
        },
    };

    let generic_json =
        Mcp::render_generic_server_snippet("demo", &server, McpSnippetFormat::Json).unwrap();
    let codex_toml = Mcp::adapt_server_snippet(
        &generic_json,
        McpTargetAdapter::Codex,
        Some(McpSnippetFormat::Toml),
    )
    .unwrap();

    assert!(codex_toml.contains("startup_timeout_sec = 20"));
    assert!(codex_toml.contains("tool_timeout_sec = 45"));
    assert!(codex_toml.contains("enabled = true"));
    assert!(codex_toml.contains("required = true"));
    assert!(codex_toml.contains("enabled_tools = [\"forecast\"]"));
    assert!(codex_toml.contains("disabled_tools = [\"screenshot\"]"));
}

#[test]
fn adapt_server_snippet_to_claude_should_materialize_http_headers() {
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
        options: Default::default(),
    };

    let generic_json =
        Mcp::render_generic_server_snippet("demo", &server, McpSnippetFormat::Json).unwrap();
    let claude_json =
        Mcp::adapt_server_snippet(&generic_json, McpTargetAdapter::Claude, None).unwrap();
    let claude_toml = Mcp::convert_server_snippet(&claude_json, McpSnippetFormat::Toml).unwrap();

    assert!(claude_json.contains("\"type\": \"http\""));
    assert!(claude_json.contains("\"Authorization\": \"Bearer ${TAVILY_API_KEY}\""));
    assert!(claude_json.contains("\"Accept\": \"application/json\""));
    assert!(claude_toml.contains("type = \"http\""));
    assert!(claude_toml.contains("Authorization = \"Bearer ${TAVILY_API_KEY}\""));
}

#[test]
fn adapt_server_snippet_to_claude_should_reject_stdio_cwd() {
    let server = agentstow_manifest::McpServerDef {
        transport: agentstow_manifest::McpTransport::Stdio {
            command: "uvx".to_string(),
            args: vec!["demo-server".to_string()],
            cwd: Some("/tmp/demo".into()),
        },
        env: vec![],
        options: Default::default(),
    };

    let generic_json =
        Mcp::render_generic_server_snippet("demo", &server, McpSnippetFormat::Json).unwrap();
    let err = Mcp::adapt_server_snippet(&generic_json, McpTargetAdapter::Claude, None).unwrap_err();

    assert!(err.to_string().contains("Claude stdio 当前不支持 cwd"));
}

#[test]
fn adapt_server_snippet_to_claude_should_render_oauth_options() {
    let server = agentstow_manifest::McpServerDef {
        transport: agentstow_manifest::McpTransport::Http {
            url: "https://example.com/mcp".to_string(),
            headers: HashMap::new(),
        },
        env: vec![],
        options: agentstow_manifest::McpServerOptions {
            oauth: Some(agentstow_manifest::McpOauthDef {
                client_id: Some("claude-client".to_string()),
                callback_port: Some(4317),
                auth_server_metadata_url: Some(
                    "https://auth.example.com/.well-known/openid-configuration".to_string(),
                ),
                scopes: vec!["ignored:for-claude".to_string()],
                ..Default::default()
            }),
            ..Default::default()
        },
    };

    let generic_json =
        Mcp::render_generic_server_snippet("demo", &server, McpSnippetFormat::Json).unwrap();
    let claude_json =
        Mcp::adapt_server_snippet(&generic_json, McpTargetAdapter::Claude, None).unwrap();

    assert!(claude_json.contains("\"oauth\": {"));
    assert!(claude_json.contains("\"clientId\": \"claude-client\""));
    assert!(claude_json.contains("\"callbackPort\": 4317"));
    assert!(claude_json.contains(
        "\"authServerMetadataUrl\": \"https://auth.example.com/.well-known/openid-configuration\""
    ));
    assert!(!claude_json.contains("ignored:for-claude"));
}

#[test]
fn adapt_server_snippet_to_gemini_should_render_env_maps_and_default_trust() {
    let server = agentstow_manifest::McpServerDef {
        transport: agentstow_manifest::McpTransport::Stdio {
            command: "uvx".to_string(),
            args: vec!["demo-server".to_string()],
            cwd: Some("/tmp/demo".into()),
        },
        env: vec![agentstow_manifest::EnvVarDef {
            key: "API_KEY".to_string(),
            binding: agentstow_core::SecretBinding::Env {
                var: "SHARED_KEY".to_string(),
            },
        }],
        options: Default::default(),
    };

    let generic_json =
        Mcp::render_generic_server_snippet("demo", &server, McpSnippetFormat::Json).unwrap();
    let gemini_json =
        Mcp::adapt_server_snippet(&generic_json, McpTargetAdapter::Gemini, None).unwrap();
    let gemini_toml = Mcp::convert_server_snippet(&gemini_json, McpSnippetFormat::Toml).unwrap();

    assert!(gemini_json.contains("\"command\": \"uvx\""));
    assert!(gemini_json.contains("\"cwd\": \"/tmp/demo\""));
    assert!(gemini_json.contains("\"trust\": false"));
    assert!(gemini_json.contains("\"API_KEY\": \"$SHARED_KEY\""));
    assert!(gemini_toml.contains("cwd = \"/tmp/demo\""));
    assert!(gemini_toml.contains("trust = false"));
    assert!(gemini_toml.contains("API_KEY = \"$SHARED_KEY\""));
}

#[test]
fn adapt_server_snippet_to_gemini_should_materialize_http_headers() {
    let server = agentstow_manifest::McpServerDef {
        transport: agentstow_manifest::McpTransport::Http {
            url: "https://example.com/mcp".to_string(),
            headers: HashMap::from([("Accept".to_string(), "application/json".to_string())]),
        },
        env: vec![agentstow_manifest::EnvVarDef {
            key: "X-Workspace-Token".to_string(),
            binding: agentstow_core::SecretBinding::Env {
                var: "WORKSPACE_TOKEN".to_string(),
            },
        }],
        options: Default::default(),
    };

    let generic_json =
        Mcp::render_generic_server_snippet("demo", &server, McpSnippetFormat::Json).unwrap();
    let gemini_json =
        Mcp::adapt_server_snippet(&generic_json, McpTargetAdapter::Gemini, None).unwrap();

    assert!(gemini_json.contains("\"httpUrl\": \"https://example.com/mcp\""));
    assert!(gemini_json.contains("\"trust\": false"));
    assert!(gemini_json.contains("\"X-Workspace-Token\": \"$WORKSPACE_TOKEN\""));
}

#[test]
fn adapt_server_snippet_to_gemini_should_render_provider_only_options() {
    let server = agentstow_manifest::McpServerDef {
        transport: agentstow_manifest::McpTransport::Http {
            url: "https://example.com/mcp".to_string(),
            headers: HashMap::new(),
        },
        env: vec![],
        options: agentstow_manifest::McpServerOptions {
            timeout: Some(30_000),
            trust: Some(true),
            description: Some("Remote toolbox".to_string()),
            include_tools: vec!["search".to_string(), "fetch".to_string()],
            exclude_tools: vec!["delete".to_string()],
            oauth: Some(agentstow_manifest::McpOauthDef {
                scopes: vec!["https://www.googleapis.com/auth/cloud-platform".to_string()],
                ..Default::default()
            }),
            auth_provider_type: Some("google_credentials".to_string()),
            target_audience: Some("https://mcp.example.com".to_string()),
            target_service_account: Some("svc@example.iam.gserviceaccount.com".to_string()),
            ..Default::default()
        },
    };

    let generic_json =
        Mcp::render_generic_server_snippet("demo", &server, McpSnippetFormat::Json).unwrap();
    let gemini_json =
        Mcp::adapt_server_snippet(&generic_json, McpTargetAdapter::Gemini, None).unwrap();

    assert!(gemini_json.contains("\"timeout\": 30000"));
    assert!(gemini_json.contains("\"trust\": true"));
    assert!(gemini_json.contains("\"description\": \"Remote toolbox\""));
    assert!(gemini_json.contains("\"includeTools\": ["));
    assert!(gemini_json.contains("\"excludeTools\": ["));
    assert!(gemini_json.contains("\"oauth\": {"));
    assert!(gemini_json.contains("\"scopes\": ["));
    assert!(gemini_json.contains("\"httpUrl\": \"https://example.com/mcp\""));
    assert!(gemini_json.contains("\"authProviderType\": \"google_credentials\""));
    assert!(gemini_json.contains("\"targetAudience\": \"https://mcp.example.com\""));
    assert!(
        gemini_json.contains("\"targetServiceAccount\": \"svc@example.iam.gserviceaccount.com\"")
    );
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
        options: Default::default(),
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
        options: Default::default(),
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
        options: Default::default(),
    };

    let checks = Mcp::test_server_dry_run("demo", &server);
    assert!(checks.iter().any(|check| check.code == "validate"));
    assert!(checks.iter().any(|check| check.code == "render"));
    assert!(checks.iter().any(|check| check.code == "headers"));
}
