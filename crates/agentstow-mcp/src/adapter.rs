use std::collections::{BTreeMap, BTreeSet, HashMap};

use agentstow_core::{AgentStowError, Result, SecretBinding};
use agentstow_manifest::{EnvVarDef, McpServerDef, McpServerOptions, McpTransport};

use crate::types::{
    ClaudeMcpOAuthConfig, ClaudeMcpServer, CodexMcpServer, CodexMcpServerOptions,
    GeminiMcpOAuthConfig, GeminiMcpServer, HttpBindingRender, StdioEnvRender,
};

pub(crate) fn validate_generic_server(name: &str, server: &McpServerDef) -> Result<()> {
    if name.trim().is_empty() {
        return Err(AgentStowError::Mcp {
            message: "mcp server name 不能为空".into(),
        });
    }

    match &server.transport {
        McpTransport::Stdio { command, .. } => {
            if command.trim().is_empty() {
                return Err(AgentStowError::Mcp {
                    message: format!("mcp[{name}] command 不能为空").into(),
                });
            }
        }
        McpTransport::Http { url, .. } => {
            if !url.starts_with("http://") && !url.starts_with("https://") {
                return Err(AgentStowError::Mcp {
                    message: format!("mcp[{name}] url 必须以 http(s):// 开头").into(),
                });
            }
        }
    }

    Ok(())
}

pub(crate) fn render_codex_server(name: &str, server: &McpServerDef) -> Result<CodexMcpServer> {
    let options = codex_options_from_generic(&server.options);
    match &server.transport {
        McpTransport::Stdio { command, args, cwd } => {
            let env = split_stdio_env_bindings(name, &server.env)?;
            Ok(CodexMcpServer::Stdio {
                command: command.clone(),
                args: args.clone(),
                env_vars: env.env_vars,
                env: env.env,
                cwd: cwd
                    .as_ref()
                    .map(|cwd| cwd.as_os_str().to_string_lossy().to_string()),
                options,
            })
        }
        McpTransport::Http { url, headers } => {
            let http = split_http_env_bindings(name, headers, &server.env)?;
            Ok(CodexMcpServer::Http {
                url: url.clone(),
                bearer_token_env_var: http.bearer_token_env_var,
                http_headers: http.http_headers,
                env_http_headers: http.env_http_headers,
                options,
            })
        }
    }
}

pub(crate) fn codex_server_to_generic_def(server: CodexMcpServer) -> McpServerDef {
    match server {
        CodexMcpServer::Stdio {
            command,
            args,
            env_vars,
            env,
            cwd,
            options,
        } => McpServerDef {
            transport: McpTransport::Stdio {
                command,
                args,
                cwd: cwd.map(Into::into),
            },
            env: imported_env_map_to_defs(env)
                .into_iter()
                .chain(env_vars.into_iter().map(|var| EnvVarDef {
                    key: var.clone(),
                    binding: SecretBinding::Env { var },
                }))
                .collect(),
            options: generic_options_from_codex(options),
        },
        CodexMcpServer::Http {
            url,
            bearer_token_env_var,
            http_headers,
            env_http_headers,
            options,
        } => McpServerDef {
            transport: McpTransport::Http {
                url,
                headers: http_headers.into_iter().collect(),
            },
            env: bearer_token_env_var
                .into_iter()
                .map(|var| EnvVarDef {
                    key: var.clone(),
                    binding: SecretBinding::Env { var },
                })
                .chain(env_http_headers.into_iter().map(|(key, var)| EnvVarDef {
                    key,
                    binding: SecretBinding::Env { var },
                }))
                .collect(),
            options: generic_options_from_codex(options),
        },
    }
}

pub(crate) fn render_claude_server(name: &str, server: &McpServerDef) -> Result<ClaudeMcpServer> {
    let oauth = claude_oauth_from_generic(&server.options);
    match &server.transport {
        McpTransport::Stdio { command, args, cwd } => {
            if cwd.is_some() {
                return Err(AgentStowError::Mcp {
                    message: format!(
                        "mcp[{name}] Claude stdio 当前不支持 cwd；Anthropic 官方 MCP 配置文档未声明该字段"
                    )
                    .into(),
                });
            }
            if oauth.is_some() {
                return Err(AgentStowError::Mcp {
                    message: format!(
                        "mcp[{name}] Claude oauth 仅适用于 HTTP/SSE transport，stdio 不能声明 options.oauth"
                    )
                    .into(),
                });
            }

            Ok(ClaudeMcpServer {
                transport_type: Some("stdio".to_string()),
                command: Some(command.clone()),
                args: args.clone(),
                env: render_bound_env_map(name, &server.env, EnvReferenceStyle::Claude)?,
                ..Default::default()
            })
        }
        McpTransport::Http { url, headers } => {
            let http = split_http_env_bindings(name, headers, &server.env)?;
            Ok(ClaudeMcpServer {
                transport_type: Some("http".to_string()),
                url: Some(url.clone()),
                headers: materialize_http_headers(&http, EnvReferenceStyle::Claude),
                oauth,
                ..Default::default()
            })
        }
    }
}

pub(crate) fn render_gemini_server(name: &str, server: &McpServerDef) -> Result<GeminiMcpServer> {
    validate_gemini_provider_options(name, &server.options)?;
    let trust = Some(server.options.trust.unwrap_or(false));
    let oauth = gemini_oauth_from_generic(&server.options);

    match &server.transport {
        McpTransport::Stdio { command, args, cwd } => Ok(GeminiMcpServer {
            command: Some(command.clone()),
            args: args.clone(),
            env: render_bound_env_map(name, &server.env, EnvReferenceStyle::Gemini)?,
            cwd: cwd
                .as_ref()
                .map(|value| value.as_os_str().to_string_lossy().to_string()),
            timeout: server.options.timeout,
            trust,
            description: server.options.description.clone(),
            include_tools: server.options.include_tools.clone(),
            exclude_tools: server.options.exclude_tools.clone(),
            oauth,
            auth_provider_type: server.options.auth_provider_type.clone(),
            target_audience: server.options.target_audience.clone(),
            target_service_account: server.options.target_service_account.clone(),
            ..Default::default()
        }),
        McpTransport::Http { url, headers } => {
            let http = split_http_env_bindings(name, headers, &server.env)?;
            Ok(GeminiMcpServer {
                http_url: Some(url.clone()),
                headers: materialize_http_headers(&http, EnvReferenceStyle::Gemini),
                timeout: server.options.timeout,
                trust,
                description: server.options.description.clone(),
                include_tools: server.options.include_tools.clone(),
                exclude_tools: server.options.exclude_tools.clone(),
                oauth,
                auth_provider_type: server.options.auth_provider_type.clone(),
                target_audience: server.options.target_audience.clone(),
                target_service_account: server.options.target_service_account.clone(),
                ..Default::default()
            })
        }
    }
}

pub(crate) fn split_stdio_env_bindings(name: &str, envs: &[EnvVarDef]) -> Result<StdioEnvRender> {
    let mut env = BTreeMap::new();
    let mut env_vars = BTreeSet::new();

    for EnvVarDef { key, binding } in envs {
        match binding {
            SecretBinding::Literal { value } => {
                if env.contains_key(key) || env_vars.contains(key) {
                    return Err(AgentStowError::Mcp {
                        message: format!("mcp[{name}] stdio env 重复声明：{key}").into(),
                    });
                }
                env.insert(key.clone(), value.clone());
            }
            SecretBinding::Env { var } if key == var => {
                if env.contains_key(key) || !env_vars.insert(var.clone()) {
                    return Err(AgentStowError::Mcp {
                        message: format!("mcp[{name}] stdio env 重复声明：{key}").into(),
                    });
                }
            }
            SecretBinding::Env { var } => {
                return Err(AgentStowError::Mcp {
                    message: format!(
                        "mcp[{name}] stdio env 绑定不支持重命名：key={key}, var={var}；Codex `env_vars` 只支持同名透传"
                    )
                    .into(),
                });
            }
        }
    }

    Ok(StdioEnvRender {
        env,
        env_vars: env_vars.into_iter().collect(),
    })
}

pub(crate) fn split_http_env_bindings(
    name: &str,
    headers: &HashMap<String, String>,
    envs: &[EnvVarDef],
) -> Result<HttpBindingRender> {
    let http_headers = headers
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut bearer_token_env_var = None;
    let mut env_http_headers = BTreeMap::new();

    for EnvVarDef { key, binding } in envs {
        match binding {
            SecretBinding::Literal { .. } => {
                return Err(AgentStowError::Mcp {
                    message: format!(
                        "mcp[{name}] HTTP env 不支持 literal；请改用 transport.headers 或 env 绑定"
                    )
                    .into(),
                });
            }
            SecretBinding::Env { var } if key == var => {
                if !looks_like_bearer_env_var(var) {
                    return Err(AgentStowError::Mcp {
                        message: format!(
                            "mcp[{name}] HTTP env `{key}` 无法自动映射到 Codex；同名 env 绑定当前仅支持 bearer token 类变量（如 *_TOKEN / *_API_KEY）"
                        )
                        .into(),
                    });
                }
                if let Some(existing) = &bearer_token_env_var {
                    return Err(AgentStowError::Mcp {
                        message: format!(
                            "mcp[{name}] HTTP bearer token env 重复声明：已有 {existing}，又收到 {var}"
                        )
                        .into(),
                    });
                }
                bearer_token_env_var = Some(var.clone());
            }
            SecretBinding::Env { var } => {
                if env_http_headers.insert(key.clone(), var.clone()).is_some() {
                    return Err(AgentStowError::Mcp {
                        message: format!("mcp[{name}] HTTP env header 重复声明：{key}").into(),
                    });
                }
            }
        }
    }

    if let Some(var) = &bearer_token_env_var {
        if http_headers.contains_key("Authorization") {
            return Err(AgentStowError::Mcp {
                message: format!(
                    "mcp[{name}] 同时声明了 bearer token env `{var}` 与静态 Authorization header；请只保留一种来源"
                )
                .into(),
            });
        }
        if env_http_headers.contains_key("Authorization") {
            return Err(AgentStowError::Mcp {
                message: format!(
                    "mcp[{name}] 同时声明了 bearer token env `{var}` 与 Authorization env header；请只保留一种来源"
                )
                .into(),
            });
        }
    }

    if let Some(duplicate) = http_headers
        .keys()
        .find(|key| env_http_headers.contains_key(*key))
        .cloned()
    {
        return Err(AgentStowError::Mcp {
            message: format!(
                "mcp[{name}] HTTP header `{duplicate}` 同时出现在 transport.headers 和 env 绑定里；请只保留一种来源"
            )
            .into(),
        });
    }

    Ok(HttpBindingRender {
        bearer_token_env_var,
        http_headers,
        env_http_headers,
    })
}

pub(crate) fn render_bound_env_map(
    name: &str,
    envs: &[EnvVarDef],
    style: EnvReferenceStyle,
) -> Result<BTreeMap<String, String>> {
    let mut rendered = BTreeMap::new();
    for EnvVarDef { key, binding } in envs {
        let value = match binding {
            SecretBinding::Literal { value } => value.clone(),
            SecretBinding::Env { var } => format_env_reference(var, style),
        };
        if rendered.insert(key.clone(), value).is_some() {
            return Err(AgentStowError::Mcp {
                message: format!("mcp[{name}] env 重复声明：{key}").into(),
            });
        }
    }
    Ok(rendered)
}

pub(crate) fn materialize_http_headers(
    render: &HttpBindingRender,
    style: EnvReferenceStyle,
) -> BTreeMap<String, String> {
    let mut headers = render.http_headers.clone();
    if let Some(var) = &render.bearer_token_env_var {
        headers.insert(
            "Authorization".to_string(),
            format!("Bearer {}", format_env_reference(var, style)),
        );
    }
    headers.extend(
        render
            .env_http_headers
            .iter()
            .map(|(key, var)| (key.clone(), format_env_reference(var, style))),
    );
    headers
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EnvReferenceStyle {
    Claude,
    Gemini,
}

fn codex_options_from_generic(options: &McpServerOptions) -> CodexMcpServerOptions {
    CodexMcpServerOptions {
        startup_timeout_sec: options.startup_timeout_sec,
        tool_timeout_sec: options.tool_timeout_sec,
        enabled: options.enabled,
        required: options.required,
        enabled_tools: options.enabled_tools.clone(),
        disabled_tools: options.disabled_tools.clone(),
    }
}

fn generic_options_from_codex(options: CodexMcpServerOptions) -> McpServerOptions {
    McpServerOptions {
        startup_timeout_sec: options.startup_timeout_sec,
        tool_timeout_sec: options.tool_timeout_sec,
        enabled: options.enabled,
        required: options.required,
        enabled_tools: options.enabled_tools,
        disabled_tools: options.disabled_tools,
        ..Default::default()
    }
}

fn claude_oauth_from_generic(options: &McpServerOptions) -> Option<ClaudeMcpOAuthConfig> {
    let oauth = options.oauth.as_ref()?;
    let config = ClaudeMcpOAuthConfig {
        client_id: oauth.client_id.clone(),
        callback_port: oauth.callback_port,
        auth_server_metadata_url: oauth.auth_server_metadata_url.clone(),
    };
    (!config.is_empty()).then_some(config)
}

fn gemini_oauth_from_generic(options: &McpServerOptions) -> Option<GeminiMcpOAuthConfig> {
    let oauth = options.oauth.as_ref()?;
    let config = GeminiMcpOAuthConfig {
        enabled: oauth.enabled,
        client_id: oauth.client_id.clone(),
        client_secret: oauth.client_secret.clone(),
        authorization_url: oauth.authorization_url.clone(),
        token_url: oauth.token_url.clone(),
        scopes: oauth.scopes.clone(),
        redirect_uri: oauth.redirect_uri.clone(),
        token_param_name: oauth.token_param_name.clone(),
        audiences: oauth.audiences.clone(),
    };
    (!config.is_empty()).then_some(config)
}

fn validate_gemini_provider_options(name: &str, options: &McpServerOptions) -> Result<()> {
    match options.auth_provider_type.as_deref() {
        Some("google_credentials") => {
            let has_scopes = options
                .oauth
                .as_ref()
                .is_some_and(|oauth| !oauth.scopes.is_empty());
            if !has_scopes {
                return Err(AgentStowError::Mcp {
                    message: format!(
                        "mcp[{name}] Gemini auth_provider_type=google_credentials 需要 options.oauth.scopes"
                    )
                    .into(),
                });
            }
        }
        Some("service_account_impersonation") => {
            if options.target_audience.is_none() || options.target_service_account.is_none() {
                return Err(AgentStowError::Mcp {
                    message: format!(
                        "mcp[{name}] Gemini auth_provider_type=service_account_impersonation 需要 target_audience 和 target_service_account"
                    )
                    .into(),
                });
            }
        }
        _ => {}
    }
    Ok(())
}

fn format_env_reference(var: &str, style: EnvReferenceStyle) -> String {
    match style {
        EnvReferenceStyle::Claude => format!("${{{var}}}"),
        EnvReferenceStyle::Gemini => format!("${var}"),
    }
}

fn looks_like_bearer_env_var(var: &str) -> bool {
    let upper = var.to_ascii_uppercase();
    upper.contains("TOKEN")
        || upper.contains("API_KEY")
        || upper.contains("BEARER")
        || upper.contains("AUTH")
}

fn imported_env_map_to_defs(env: BTreeMap<String, String>) -> Vec<EnvVarDef> {
    env.into_iter()
        .map(|(key, value)| EnvVarDef {
            key,
            binding: SecretBinding::Literal { value },
        })
        .collect()
}
