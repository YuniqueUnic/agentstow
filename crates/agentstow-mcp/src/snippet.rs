use std::collections::BTreeMap;

use agentstow_core::{AgentStowError, Result, SecretBinding};
use agentstow_manifest::{EnvVarDef, McpServerDef, McpServerOptions, McpTransport};

use crate::types::{
    ClaudeMcpJsonFile, ClaudeMcpOAuthConfig, ClaudeMcpServer, CodexMcpJsonFile, CodexMcpServer,
    CodexMcpServerOptions, CodexMcpTomlServer, GeminiMcpJsonFile, GeminiMcpOAuthConfig,
    GeminiMcpServer, GenericMcpJsonFile, McpSnippetFormat,
};

pub(crate) fn render_generic_server_payload(
    name: &str,
    server: &McpServerDef,
    format: McpSnippetFormat,
) -> Result<String> {
    match format {
        McpSnippetFormat::Json => {
            let file = GenericMcpJsonFile {
                mcp_servers: BTreeMap::from([(name.to_string(), server.clone())]),
            };
            serde_json::to_string_pretty(&file).map_err(|e| AgentStowError::Mcp {
                message: format!("序列化通用 MCP JSON 失败：{e}").into(),
            })
        }
        McpSnippetFormat::Toml => render_generic_server_toml_payload(name, server),
        McpSnippetFormat::Yaml => {
            let file = GenericMcpJsonFile {
                mcp_servers: BTreeMap::from([(name.to_string(), server.clone())]),
            };
            serde_yaml::to_string(&file).map_err(|e| AgentStowError::Mcp {
                message: format!("序列化通用 MCP YAML 失败：{e}").into(),
            })
        }
    }
}

pub(crate) fn render_codex_server_payload(
    name: &str,
    server: &CodexMcpServer,
    format: McpSnippetFormat,
) -> Result<String> {
    match format {
        McpSnippetFormat::Json => render_codex_server_json_payload(name, server),
        McpSnippetFormat::Toml => render_codex_server_toml_payload(name, server),
        McpSnippetFormat::Yaml => render_codex_server_yaml_payload(name, server),
    }
}

pub(crate) fn render_claude_server_payload(
    name: &str,
    server: &ClaudeMcpServer,
    format: McpSnippetFormat,
) -> Result<String> {
    match format {
        McpSnippetFormat::Json => render_claude_server_json_payload(name, server),
        McpSnippetFormat::Toml => render_claude_server_toml_payload(name, server),
        McpSnippetFormat::Yaml => render_claude_server_yaml_payload(name, server),
    }
}

pub(crate) fn render_gemini_server_payload(
    name: &str,
    server: &GeminiMcpServer,
    format: McpSnippetFormat,
) -> Result<String> {
    match format {
        McpSnippetFormat::Json => render_gemini_server_json_payload(name, server),
        McpSnippetFormat::Toml => render_gemini_server_toml_payload(name, server),
        McpSnippetFormat::Yaml => render_gemini_server_yaml_payload(name, server),
    }
}

fn render_generic_server_toml_payload(name: &str, server: &McpServerDef) -> Result<String> {
    let rendered_name = render_server_name(name);
    let mut lines = vec![format!("[mcp_servers.{rendered_name}]")];

    if !server.env.is_empty() {
        lines.push(format!("env = {}", encode_toml_env_defs(&server.env)?));
    }
    append_generic_options_block(&mut lines, &rendered_name, &server.options)?;

    lines.push(String::new());
    lines.push(format!("[mcp_servers.{rendered_name}.transport]"));
    match &server.transport {
        McpTransport::Stdio { command, args, cwd } => {
            lines.push("kind = \"stdio\"".to_string());
            lines.push(format!("command = {}", encode_toml_string(command)?));
            if !args.is_empty() {
                lines.push(format!("args = {}", encode_toml_string_array(args)?));
            }
            if let Some(cwd) = cwd {
                lines.push(format!(
                    "cwd = {}",
                    encode_toml_string(&cwd.display().to_string())?
                ));
            }
        }
        McpTransport::Http { url, headers } => {
            lines.push("kind = \"http\"".to_string());
            lines.push(format!("url = {}", encode_toml_string(url)?));
            if !headers.is_empty() {
                lines.push(String::new());
                lines.push(format!("[mcp_servers.{rendered_name}.transport.headers]"));
                for (key, value) in headers {
                    lines.push(format!("{key} = {}", encode_toml_string(value)?));
                }
            }
        }
    }

    Ok(lines.join("\n") + "\n")
}

fn render_codex_server_json_payload(name: &str, server: &CodexMcpServer) -> Result<String> {
    let file = CodexMcpJsonFile {
        mcp_servers: BTreeMap::from([(name.to_string(), server.clone())]),
    };
    serde_json::to_string_pretty(&file).map_err(|e| AgentStowError::Mcp {
        message: format!("序列化 MCP JSON 失败：{e}").into(),
    })
}

fn render_codex_server_toml_payload(name: &str, server: &CodexMcpServer) -> Result<String> {
    let rendered_name = render_server_name(name);
    let mut lines = vec![format!("[mcp_servers.{rendered_name}]")];
    match server {
        CodexMcpServer::Stdio {
            command,
            args,
            env,
            options,
            ..
        } => {
            if !args.is_empty() {
                lines.push(format!("args = {}", encode_toml_string_array(args)?));
            }
            lines.push(format!("command = {}", encode_toml_string(command)?));
            if let CodexMcpServer::Stdio { env_vars, cwd, .. } = server {
                if !env_vars.is_empty() {
                    lines.push(format!(
                        "env_vars = {}",
                        encode_toml_string_array(env_vars)?
                    ));
                }
                if let Some(cwd) = cwd {
                    lines.push(format!("cwd = {}", encode_toml_string(cwd)?));
                }
            }
            append_codex_options_block(&mut lines, options)?;
            append_env_block(&mut lines, &rendered_name, env.clone())?;
        }
        CodexMcpServer::Http {
            url,
            bearer_token_env_var,
            http_headers,
            env_http_headers,
            options,
        } => {
            lines.push(format!("url = {}", encode_toml_string(url)?));
            if let Some(env_var) = bearer_token_env_var {
                lines.push(format!(
                    "bearer_token_env_var = {}",
                    encode_toml_string(env_var)?
                ));
            }
            append_codex_options_block(&mut lines, options)?;
            if !http_headers.is_empty() {
                lines.push(String::new());
                lines.push(format!("[mcp_servers.{rendered_name}.http_headers]"));
                for (key, value) in http_headers {
                    lines.push(format!("{key} = {}", encode_toml_string(value)?));
                }
            }
            if !env_http_headers.is_empty() {
                lines.push(String::new());
                lines.push(format!("[mcp_servers.{rendered_name}.env_http_headers]"));
                for (key, value) in env_http_headers {
                    lines.push(format!("{key} = {}", encode_toml_string(value)?));
                }
            }
        }
    }

    Ok(lines.join("\n") + "\n")
}

fn render_codex_server_yaml_payload(name: &str, server: &CodexMcpServer) -> Result<String> {
    let file = CodexMcpJsonFile {
        mcp_servers: BTreeMap::from([(name.to_string(), server.clone())]),
    };
    serde_yaml::to_string(&file).map_err(|e| AgentStowError::Mcp {
        message: format!("序列化 MCP YAML 失败：{e}").into(),
    })
}

fn render_claude_server_json_payload(name: &str, server: &ClaudeMcpServer) -> Result<String> {
    let file = ClaudeMcpJsonFile {
        mcp_servers: BTreeMap::from([(name.to_string(), server.clone())]),
    };
    serde_json::to_string_pretty(&file).map_err(|e| AgentStowError::Mcp {
        message: format!("序列化 Claude MCP JSON 失败：{e}").into(),
    })
}

fn render_claude_server_toml_payload(name: &str, server: &ClaudeMcpServer) -> Result<String> {
    let rendered_name = render_server_name(name);
    let mut lines = vec![format!("[mcp_servers.{rendered_name}]")];

    if let Some(transport_type) = &server.transport_type {
        lines.push(format!("type = {}", encode_toml_string(transport_type)?));
    }
    if let Some(command) = &server.command {
        lines.push(format!("command = {}", encode_toml_string(command)?));
    }
    if !server.args.is_empty() {
        lines.push(format!(
            "args = {}",
            encode_toml_string_array(&server.args)?
        ));
    }
    if let Some(url) = &server.url {
        lines.push(format!("url = {}", encode_toml_string(url)?));
    }
    append_env_block(&mut lines, &rendered_name, server.env.clone())?;
    append_string_map_table(&mut lines, &rendered_name, "headers", &server.headers)?;
    if let Some(oauth) = &server.oauth {
        append_claude_oauth_block(&mut lines, &rendered_name, oauth)?;
    }

    Ok(lines.join("\n") + "\n")
}

fn render_claude_server_yaml_payload(name: &str, server: &ClaudeMcpServer) -> Result<String> {
    let file = ClaudeMcpJsonFile {
        mcp_servers: BTreeMap::from([(name.to_string(), server.clone())]),
    };
    serde_yaml::to_string(&file).map_err(|e| AgentStowError::Mcp {
        message: format!("序列化 Claude MCP YAML 失败：{e}").into(),
    })
}

fn render_gemini_server_json_payload(name: &str, server: &GeminiMcpServer) -> Result<String> {
    let file = GeminiMcpJsonFile {
        mcp_servers: BTreeMap::from([(name.to_string(), server.clone())]),
    };
    serde_json::to_string_pretty(&file).map_err(|e| AgentStowError::Mcp {
        message: format!("序列化 Gemini MCP JSON 失败：{e}").into(),
    })
}

fn render_gemini_server_toml_payload(name: &str, server: &GeminiMcpServer) -> Result<String> {
    let rendered_name = render_server_name(name);
    let mut lines = vec![format!("[mcp_servers.{rendered_name}]")];

    if let Some(command) = &server.command {
        lines.push(format!("command = {}", encode_toml_string(command)?));
    }
    if !server.args.is_empty() {
        lines.push(format!(
            "args = {}",
            encode_toml_string_array(&server.args)?
        ));
    }
    if let Some(cwd) = &server.cwd {
        lines.push(format!("cwd = {}", encode_toml_string(cwd)?));
    }
    if let Some(url) = &server.url {
        lines.push(format!("url = {}", encode_toml_string(url)?));
    }
    if let Some(http_url) = &server.http_url {
        lines.push(format!("httpUrl = {}", encode_toml_string(http_url)?));
    }
    if let Some(transport_type) = &server.transport_type {
        lines.push(format!("type = {}", encode_toml_string(transport_type)?));
    }
    if let Some(timeout) = server.timeout {
        lines.push(format!("timeout = {timeout}"));
    }
    if let Some(trust) = server.trust {
        lines.push(format!("trust = {trust}"));
    }
    if let Some(description) = &server.description {
        lines.push(format!(
            "description = {}",
            encode_toml_string(description)?
        ));
    }
    if !server.include_tools.is_empty() {
        lines.push(format!(
            "includeTools = {}",
            encode_toml_string_array(&server.include_tools)?
        ));
    }
    if !server.exclude_tools.is_empty() {
        lines.push(format!(
            "excludeTools = {}",
            encode_toml_string_array(&server.exclude_tools)?
        ));
    }
    if let Some(auth_provider_type) = &server.auth_provider_type {
        lines.push(format!(
            "authProviderType = {}",
            encode_toml_string(auth_provider_type)?
        ));
    }
    if let Some(target_audience) = &server.target_audience {
        lines.push(format!(
            "targetAudience = {}",
            encode_toml_string(target_audience)?
        ));
    }
    if let Some(target_service_account) = &server.target_service_account {
        lines.push(format!(
            "targetServiceAccount = {}",
            encode_toml_string(target_service_account)?
        ));
    }

    append_env_block(&mut lines, &rendered_name, server.env.clone())?;
    append_string_map_table(&mut lines, &rendered_name, "headers", &server.headers)?;
    if let Some(oauth) = &server.oauth {
        append_gemini_oauth_block(&mut lines, &rendered_name, oauth)?;
    }

    Ok(lines.join("\n") + "\n")
}

fn render_gemini_server_yaml_payload(name: &str, server: &GeminiMcpServer) -> Result<String> {
    let file = GeminiMcpJsonFile {
        mcp_servers: BTreeMap::from([(name.to_string(), server.clone())]),
    };
    serde_yaml::to_string(&file).map_err(|e| AgentStowError::Mcp {
        message: format!("序列化 Gemini MCP YAML 失败：{e}").into(),
    })
}

fn append_generic_options_block(
    lines: &mut Vec<String>,
    rendered_name: &str,
    options: &McpServerOptions,
) -> Result<()> {
    if options.is_empty() {
        return Ok(());
    }

    lines.push(String::new());
    lines.push(format!("[mcp_servers.{rendered_name}.options]"));
    append_optional_u64(lines, "startup_timeout_sec", options.startup_timeout_sec);
    append_optional_u64(lines, "tool_timeout_sec", options.tool_timeout_sec);
    append_optional_bool(lines, "enabled", options.enabled);
    append_optional_bool(lines, "required", options.required);
    append_optional_string_array(lines, "enabled_tools", &options.enabled_tools)?;
    append_optional_string_array(lines, "disabled_tools", &options.disabled_tools)?;
    append_optional_u64(lines, "timeout", options.timeout);
    append_optional_bool(lines, "trust", options.trust);
    append_optional_string(lines, "description", options.description.as_deref())?;
    append_optional_string_array(lines, "include_tools", &options.include_tools)?;
    append_optional_string_array(lines, "exclude_tools", &options.exclude_tools)?;
    append_optional_string(
        lines,
        "auth_provider_type",
        options.auth_provider_type.as_deref(),
    )?;
    append_optional_string(lines, "target_audience", options.target_audience.as_deref())?;
    append_optional_string(
        lines,
        "target_service_account",
        options.target_service_account.as_deref(),
    )?;

    if let Some(oauth) = &options.oauth
        && !oauth.is_empty()
    {
        lines.push(String::new());
        lines.push(format!("[mcp_servers.{rendered_name}.options.oauth]"));
        append_optional_string(lines, "client_id", oauth.client_id.as_deref())?;
        if let Some(callback_port) = oauth.callback_port {
            lines.push(format!("callback_port = {callback_port}"));
        }
        append_optional_bool(lines, "enabled", oauth.enabled);
        append_optional_string(lines, "client_secret", oauth.client_secret.as_deref())?;
        append_optional_string(
            lines,
            "authorization_url",
            oauth.authorization_url.as_deref(),
        )?;
        append_optional_string(lines, "token_url", oauth.token_url.as_deref())?;
        append_optional_string(
            lines,
            "auth_server_metadata_url",
            oauth.auth_server_metadata_url.as_deref(),
        )?;
        append_optional_string_array(lines, "scopes", &oauth.scopes)?;
        append_optional_string(lines, "redirect_uri", oauth.redirect_uri.as_deref())?;
        append_optional_string(lines, "token_param_name", oauth.token_param_name.as_deref())?;
        append_optional_string_array(lines, "audiences", &oauth.audiences)?;
    }

    Ok(())
}

fn append_codex_options_block(
    lines: &mut Vec<String>,
    options: &CodexMcpServerOptions,
) -> Result<()> {
    append_optional_u64(lines, "startup_timeout_sec", options.startup_timeout_sec);
    append_optional_u64(lines, "tool_timeout_sec", options.tool_timeout_sec);
    append_optional_bool(lines, "enabled", options.enabled);
    append_optional_bool(lines, "required", options.required);
    append_optional_string_array(lines, "enabled_tools", &options.enabled_tools)?;
    append_optional_string_array(lines, "disabled_tools", &options.disabled_tools)?;
    Ok(())
}

fn append_env_block(
    lines: &mut Vec<String>,
    rendered_name: &str,
    env: BTreeMap<String, String>,
) -> Result<()> {
    if env.is_empty() {
        return Ok(());
    }

    lines.push(String::new());
    lines.push(format!("[mcp_servers.{rendered_name}.env]"));
    for (key, value) in env {
        lines.push(format!("{key} = {}", encode_toml_string(&value)?));
    }
    Ok(())
}

fn append_string_map_table(
    lines: &mut Vec<String>,
    rendered_name: &str,
    table_name: &str,
    values: &BTreeMap<String, String>,
) -> Result<()> {
    if values.is_empty() {
        return Ok(());
    }

    lines.push(String::new());
    lines.push(format!("[mcp_servers.{rendered_name}.{table_name}]"));
    for (key, value) in values {
        lines.push(format!("{key} = {}", encode_toml_string(value)?));
    }
    Ok(())
}

fn append_optional_string(lines: &mut Vec<String>, key: &str, value: Option<&str>) -> Result<()> {
    if let Some(value) = value {
        lines.push(format!("{key} = {}", encode_toml_string(value)?));
    }
    Ok(())
}

fn append_optional_string_array(
    lines: &mut Vec<String>,
    key: &str,
    values: &[String],
) -> Result<()> {
    if !values.is_empty() {
        lines.push(format!("{key} = {}", encode_toml_string_array(values)?));
    }
    Ok(())
}

fn append_optional_bool(lines: &mut Vec<String>, key: &str, value: Option<bool>) {
    if let Some(value) = value {
        lines.push(format!("{key} = {value}"));
    }
}

fn append_optional_u64(lines: &mut Vec<String>, key: &str, value: Option<u64>) {
    if let Some(value) = value {
        lines.push(format!("{key} = {value}"));
    }
}

fn append_claude_oauth_block(
    lines: &mut Vec<String>,
    rendered_name: &str,
    oauth: &ClaudeMcpOAuthConfig,
) -> Result<()> {
    lines.push(String::new());
    lines.push(format!("[mcp_servers.{rendered_name}.oauth]"));
    if let Some(client_id) = &oauth.client_id {
        lines.push(format!("clientId = {}", encode_toml_string(client_id)?));
    }
    if let Some(callback_port) = oauth.callback_port {
        lines.push(format!("callbackPort = {callback_port}"));
    }
    if let Some(url) = &oauth.auth_server_metadata_url {
        lines.push(format!(
            "authServerMetadataUrl = {}",
            encode_toml_string(url)?
        ));
    }
    Ok(())
}

fn append_gemini_oauth_block(
    lines: &mut Vec<String>,
    rendered_name: &str,
    oauth: &GeminiMcpOAuthConfig,
) -> Result<()> {
    lines.push(String::new());
    lines.push(format!("[mcp_servers.{rendered_name}.oauth]"));
    append_optional_bool(lines, "enabled", oauth.enabled);
    append_optional_string(lines, "clientId", oauth.client_id.as_deref())?;
    append_optional_string(lines, "clientSecret", oauth.client_secret.as_deref())?;
    append_optional_string(
        lines,
        "authorizationUrl",
        oauth.authorization_url.as_deref(),
    )?;
    append_optional_string(lines, "tokenUrl", oauth.token_url.as_deref())?;
    append_optional_string_array(lines, "scopes", &oauth.scopes)?;
    append_optional_string(lines, "redirectUri", oauth.redirect_uri.as_deref())?;
    append_optional_string(lines, "tokenParamName", oauth.token_param_name.as_deref())?;
    append_optional_string_array(lines, "audiences", &oauth.audiences)?;
    Ok(())
}

fn render_server_name(name: &str) -> String {
    name.replace('_', "-")
}

fn encode_toml_string(value: &str) -> Result<String> {
    serde_json::to_string(value).map_err(|e| AgentStowError::Mcp {
        message: format!("序列化 MCP TOML 字符串失败：{e}").into(),
    })
}

fn encode_toml_string_array(values: &[String]) -> Result<String> {
    let encoded = values
        .iter()
        .map(|value| encode_toml_string(value))
        .collect::<Result<Vec<_>>>()?;
    Ok(format!("[{}]", encoded.join(", ")))
}

fn encode_toml_env_defs(values: &[EnvVarDef]) -> Result<String> {
    let encoded = values
        .iter()
        .map(encode_toml_env_def)
        .collect::<Result<Vec<_>>>()?;
    Ok(format!("[{}]", encoded.join(", ")))
}

fn encode_toml_env_def(value: &EnvVarDef) -> Result<String> {
    let binding = match &value.binding {
        SecretBinding::Literal { value } => format!(
            "{{ kind = \"literal\", value = {} }}",
            encode_toml_string(value)?
        ),
        SecretBinding::Env { var } => {
            format!("{{ kind = \"env\", var = {} }}", encode_toml_string(var)?)
        }
    };
    Ok(format!(
        "{{ key = {}, binding = {} }}",
        encode_toml_string(&value.key)?,
        binding
    ))
}

impl CodexMcpTomlServer {
    pub(crate) fn into_codex_server(self) -> Result<CodexMcpServer> {
        if let Some(command) = self.command {
            return Ok(CodexMcpServer::Stdio {
                command,
                args: self.args,
                env_vars: self.env_vars,
                env: self.env,
                cwd: self.cwd,
                options: self.options,
            });
        }
        if let Some(url) = self.url {
            return Ok(CodexMcpServer::Http {
                url,
                bearer_token_env_var: self.bearer_token_env_var,
                http_headers: self.http_headers,
                env_http_headers: self.env_http_headers,
                options: self.options,
            });
        }
        Err(AgentStowError::Mcp {
            message: "TOML MCP 片段缺少 command/url，无法判断 transport".into(),
        })
    }
}
