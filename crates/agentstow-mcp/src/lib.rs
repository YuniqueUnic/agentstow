use std::collections::{BTreeMap, BTreeSet};

use agentstow_core::{AgentStowError, Result, SecretBinding};
use agentstow_manifest::{EnvVarDef, McpServerDef, McpTransport};
use serde::{Deserialize, Serialize};
use tracing::instrument;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GenericMcpJsonFile {
    #[serde(rename = "mcpServers")]
    mcp_servers: BTreeMap<String, McpServerDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GenericMcpTomlFile {
    #[serde(rename = "mcp_servers")]
    mcp_servers: BTreeMap<String, McpServerDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexMcpJsonFile {
    #[serde(rename = "mcpServers")]
    pub mcp_servers: BTreeMap<String, CodexMcpServer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CodexMcpServer {
    Stdio {
        command: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        args: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        env_vars: Vec<String>,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        env: BTreeMap<String, String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cwd: Option<String>,
    },
    Http {
        url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bearer_token_env_var: Option<String>,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        http_headers: BTreeMap<String, String>,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        env_http_headers: BTreeMap<String, String>,
    },
}

#[derive(Debug, Deserialize)]
struct CodexMcpTomlFile {
    #[serde(rename = "mcp_servers", default)]
    mcp_servers: BTreeMap<String, CodexMcpTomlServer>,
}

#[derive(Debug, Deserialize)]
struct CodexMcpTomlServer {
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    env_vars: Vec<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    bearer_token_env_var: Option<String>,
    #[serde(default)]
    http_headers: BTreeMap<String, String>,
    #[serde(default)]
    env_http_headers: BTreeMap<String, String>,
    #[serde(default)]
    env: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClaudeMcpJsonFile {
    #[serde(rename = "mcpServers")]
    mcp_servers: BTreeMap<String, ClaudeMcpServer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClaudeMcpTomlFile {
    #[serde(rename = "mcp_servers", default)]
    mcp_servers: BTreeMap<String, ClaudeMcpServer>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ClaudeMcpOAuthConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    client_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    callback_port: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    auth_server_metadata_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ClaudeMcpServer {
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    transport_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    command: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    args: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    env: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    headers: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    oauth: Option<ClaudeMcpOAuthConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiMcpJsonFile {
    #[serde(rename = "mcpServers")]
    mcp_servers: BTreeMap<String, GeminiMcpServer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GeminiMcpTomlFile {
    #[serde(rename = "mcp_servers", default)]
    mcp_servers: BTreeMap<String, GeminiMcpServer>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct GeminiMcpOAuthConfig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct GeminiMcpServer {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    command: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    args: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    env: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cwd: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    http_url: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    headers: BTreeMap<String, String>,
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    transport_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    timeout: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    trust: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    include_tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    exclude_tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    oauth: Option<GeminiMcpOAuthConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    auth_provider_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    target_audience: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    target_service_account: Option<String>,
}

#[derive(Debug, Clone)]
struct StdioEnvRender {
    env: BTreeMap<String, String>,
    env_vars: Vec<String>,
}

#[derive(Debug, Clone)]
struct HttpBindingRender {
    bearer_token_env_var: Option<String>,
    http_headers: BTreeMap<String, String>,
    env_http_headers: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
enum ParsedServerSnippet {
    Generic {
        name: String,
        server: McpServerDef,
    },
    Codex {
        name: String,
        server: CodexMcpServer,
    },
    Claude {
        name: String,
        server: ClaudeMcpServer,
    },
    Gemini {
        name: String,
        server: GeminiMcpServer,
    },
}

impl ParsedServerSnippet {
    fn format(&self, format: McpSnippetFormat) -> Result<String> {
        match self {
            Self::Generic { name, server } => render_generic_server_payload(name, server, format),
            Self::Codex { name, server } => render_codex_server_payload(name, server, format),
            Self::Claude { name, server } => render_claude_server_payload(name, server, format),
            Self::Gemini { name, server } => render_gemini_server_payload(name, server, format),
        }
    }

    fn adapt(self, target: McpTargetAdapter) -> Result<Self> {
        match (self, target) {
            (snippet @ Self::Generic { .. }, McpTargetAdapter::Generic) => Ok(snippet),
            (snippet @ Self::Codex { .. }, McpTargetAdapter::Codex) => Ok(snippet),
            (snippet @ Self::Claude { .. }, McpTargetAdapter::Claude) => Ok(snippet),
            (snippet @ Self::Gemini { .. }, McpTargetAdapter::Gemini) => Ok(snippet),
            (Self::Generic { name, server }, McpTargetAdapter::Codex) => Ok(Self::Codex {
                name: name.clone(),
                server: render_codex_server(name.as_str(), &server)?,
            }),
            (Self::Generic { name, server }, McpTargetAdapter::Claude) => Ok(Self::Claude {
                name: name.clone(),
                server: render_claude_server(name.as_str(), &server)?,
            }),
            (Self::Generic { name, server }, McpTargetAdapter::Gemini) => Ok(Self::Gemini {
                name: name.clone(),
                server: render_gemini_server(name.as_str(), &server)?,
            }),
            (Self::Codex { name, server }, McpTargetAdapter::Generic) => Ok(Self::Generic {
                name,
                server: codex_server_to_generic_def(server),
            }),
            (snippet, target) => Err(AgentStowError::Mcp {
                message: format!(
                    "无法把 {} MCP 片段直接适配为 {}；请改用通用 mcp_servers.<name> 片段作为输入",
                    snippet.kind_name(),
                    target.as_str()
                )
                .into(),
            }),
        }
    }

    fn kind_name(&self) -> &'static str {
        match self {
            Self::Generic { .. } => "通用",
            Self::Codex { .. } => "Codex",
            Self::Claude { .. } => "Claude",
            Self::Gemini { .. } => "Gemini",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpSnippetFormat {
    Json,
    Toml,
    Yaml,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpTargetAdapter {
    Generic,
    Codex,
    Claude,
    Gemini,
}

impl McpTargetAdapter {
    fn as_str(self) -> &'static str {
        match self {
            Self::Generic => "通用",
            Self::Codex => "Codex",
            Self::Claude => "Claude",
            Self::Gemini => "Gemini",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpDryRunCheckStatus {
    Ok,
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpDryRunCheck {
    pub code: String,
    pub status: McpDryRunCheckStatus,
    pub message: String,
    pub detail: Option<String>,
}

pub struct Mcp;

impl Mcp {
    #[instrument(skip_all)]
    pub fn validate_server(name: &str, server: &McpServerDef) -> Result<()> {
        validate_generic_server(name, server)?;
        match &server.transport {
            McpTransport::Stdio { .. } => {
                split_stdio_env_bindings(name, &server.env)?;
            }
            McpTransport::Http { headers, .. } => {
                split_http_env_bindings(name, headers, &server.env)?;
            }
        }
        Ok(())
    }

    pub fn render_mcp_json(servers: &BTreeMap<String, McpServerDef>) -> Result<String> {
        let mut out = BTreeMap::new();
        for (name, server) in servers {
            Self::validate_server(name, server)?;
            out.insert(name.clone(), render_codex_server(name, server)?);
        }

        let file = CodexMcpJsonFile { mcp_servers: out };
        serde_json::to_string_pretty(&file).map_err(|e| AgentStowError::Mcp {
            message: format!("序列化 MCP JSON 失败：{e}").into(),
        })
    }

    pub fn render_server_json(name: &str, server: &McpServerDef) -> Result<String> {
        Self::validate_server(name, server)?;
        render_codex_server_payload(
            name,
            &render_codex_server(name, server)?,
            McpSnippetFormat::Json,
        )
    }

    pub fn render_server_toml(name: &str, server: &McpServerDef) -> Result<String> {
        Self::validate_server(name, server)?;
        render_codex_server_payload(
            name,
            &render_codex_server(name, server)?,
            McpSnippetFormat::Toml,
        )
    }

    pub fn render_server_snippet(
        name: &str,
        server: &McpServerDef,
        format: McpSnippetFormat,
    ) -> Result<String> {
        Self::validate_server(name, server)?;
        render_codex_server_payload(name, &render_codex_server(name, server)?, format)
    }

    pub fn render_generic_server_snippet(
        name: &str,
        server: &McpServerDef,
        format: McpSnippetFormat,
    ) -> Result<String> {
        validate_generic_server(name, server)?;
        render_generic_server_payload(name, server, format)
    }

    pub fn convert_server_snippet(rendered: &str, format: McpSnippetFormat) -> Result<String> {
        parse_rendered_server(rendered)?.format(format)
    }

    pub fn adapt_server_snippet(
        rendered: &str,
        adapter: McpTargetAdapter,
        format: Option<McpSnippetFormat>,
    ) -> Result<String> {
        let parsed = parse_rendered_server(rendered)?;
        let target_format = format.unwrap_or_else(|| infer_snippet_format(rendered));
        parsed.adapt(adapter)?.format(target_format)
    }

    pub fn codex_http_header_preview(
        name: &str,
        server: &McpServerDef,
    ) -> Result<Vec<(String, String)>> {
        let CodexMcpServer::Http {
            bearer_token_env_var,
            http_headers,
            env_http_headers,
            ..
        } = render_codex_server(name, server)?
        else {
            return Ok(Vec::new());
        };

        let mut out = Vec::new();
        if let Some(var) = bearer_token_env_var {
            out.push(("Authorization".to_string(), format!("Bearer ${{{var}}}")));
        }
        out.extend(http_headers);
        out.extend(
            env_http_headers
                .into_iter()
                .map(|(key, var)| (key, format!("${{{var}}}"))),
        );
        Ok(out)
    }

    pub fn launcher_preview(name: &str, server: &McpServerDef) -> String {
        match &server.transport {
            McpTransport::Stdio { command, args, .. } => std::iter::once(command.as_str())
                .chain(args.iter().map(String::as_str))
                .map(quote_shell_arg)
                .collect::<Vec<_>>()
                .join(" "),
            McpTransport::Http { url, headers } => {
                let header_lines = Self::codex_http_header_preview(name, server)
                    .unwrap_or_else(|_| {
                        headers
                            .iter()
                            .map(|(key, value)| (key.clone(), value.clone()))
                            .collect()
                    })
                    .into_iter()
                    .map(|(key, value)| format!("{key}: {value}"))
                    .collect::<Vec<_>>()
                    .join("\n");
                [format!("GET {url}"), header_lines]
                    .into_iter()
                    .filter(|part| !part.is_empty())
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        }
    }

    pub fn test_server_dry_run(name: &str, server: &McpServerDef) -> Vec<McpDryRunCheck> {
        let mut checks = Vec::new();

        match Self::validate_server(name, server) {
            Ok(()) => checks.push(McpDryRunCheck {
                code: "validate".to_string(),
                status: McpDryRunCheckStatus::Ok,
                message: "Codex transport 校验通过".to_string(),
                detail: None,
            }),
            Err(error) => {
                checks.push(McpDryRunCheck {
                    code: "validate".to_string(),
                    status: McpDryRunCheckStatus::Error,
                    message: "Codex transport 校验失败".to_string(),
                    detail: Some(error.to_string()),
                });
                return checks;
            }
        }

        match &server.transport {
            McpTransport::Stdio { command, args, .. } => {
                checks.push(McpDryRunCheck {
                    code: "launcher".to_string(),
                    status: McpDryRunCheckStatus::Ok,
                    message: "stdio launcher 已解析".to_string(),
                    detail: Some(
                        std::iter::once(command.as_str())
                            .chain(args.iter().map(String::as_str))
                            .collect::<Vec<_>>()
                            .join(" "),
                    ),
                });
            }
            McpTransport::Http { url, .. } => {
                let header_count = Self::codex_http_header_preview(name, server)
                    .map(|headers| headers.len())
                    .unwrap_or_default();
                checks.push(McpDryRunCheck {
                    code: "endpoint".to_string(),
                    status: McpDryRunCheckStatus::Ok,
                    message: "HTTP endpoint 已解析".to_string(),
                    detail: Some(url.clone()),
                });
                checks.push(McpDryRunCheck {
                    code: "headers".to_string(),
                    status: if header_count == 0 {
                        McpDryRunCheckStatus::Warn
                    } else {
                        McpDryRunCheckStatus::Ok
                    },
                    message: if header_count == 0 {
                        "未声明 HTTP headers".to_string()
                    } else {
                        format!("已声明 {} 个 HTTP headers", header_count)
                    },
                    detail: None,
                });
            }
        }

        checks.push(McpDryRunCheck {
            code: "render".to_string(),
            status: match Self::render_server_json(name, server) {
                Ok(_) => McpDryRunCheckStatus::Ok,
                Err(_) => McpDryRunCheckStatus::Error,
            },
            message: "Codex 单 server 配置可渲染".to_string(),
            detail: None,
        });

        checks
    }
}

fn validate_generic_server(name: &str, server: &McpServerDef) -> Result<()> {
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

fn render_codex_server(name: &str, server: &McpServerDef) -> Result<CodexMcpServer> {
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
            })
        }
        McpTransport::Http { url, headers } => {
            let http = split_http_env_bindings(name, headers, &server.env)?;
            Ok(CodexMcpServer::Http {
                url: url.clone(),
                bearer_token_env_var: http.bearer_token_env_var,
                http_headers: http.http_headers,
                env_http_headers: http.env_http_headers,
            })
        }
    }
}

fn codex_server_to_generic_def(server: CodexMcpServer) -> McpServerDef {
    match server {
        CodexMcpServer::Stdio {
            command,
            args,
            env_vars,
            env,
            cwd,
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
        },
        CodexMcpServer::Http {
            url,
            bearer_token_env_var,
            http_headers,
            env_http_headers,
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
        },
    }
}

fn render_claude_server(name: &str, server: &McpServerDef) -> Result<ClaudeMcpServer> {
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
                ..Default::default()
            })
        }
    }
}

fn render_gemini_server(name: &str, server: &McpServerDef) -> Result<GeminiMcpServer> {
    match &server.transport {
        McpTransport::Stdio { command, args, cwd } => Ok(GeminiMcpServer {
            command: Some(command.clone()),
            args: args.clone(),
            env: render_bound_env_map(name, &server.env, EnvReferenceStyle::Gemini)?,
            cwd: cwd
                .as_ref()
                .map(|value| value.as_os_str().to_string_lossy().to_string()),
            trust: Some(false),
            ..Default::default()
        }),
        McpTransport::Http { url, headers } => {
            let http = split_http_env_bindings(name, headers, &server.env)?;
            Ok(GeminiMcpServer {
                url: Some(url.clone()),
                transport_type: Some("http".to_string()),
                headers: materialize_http_headers(&http, EnvReferenceStyle::Gemini),
                trust: Some(false),
                ..Default::default()
            })
        }
    }
}

fn render_generic_server_payload(
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

fn render_generic_server_toml_payload(name: &str, server: &McpServerDef) -> Result<String> {
    let rendered_name = render_server_name(name);
    let mut lines = vec![format!("[mcp_servers.{rendered_name}]")];

    if !server.env.is_empty() {
        lines.push(format!("env = {}", encode_toml_env_defs(&server.env)?));
    }

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

fn render_codex_server_payload(
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
            command, args, env, ..
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
            append_env_block(&mut lines, &rendered_name, env.clone())?;
        }
        CodexMcpServer::Http {
            url,
            bearer_token_env_var,
            http_headers,
            env_http_headers,
        } => {
            lines.push(format!("url = {}", encode_toml_string(url)?));
            if let Some(env_var) = bearer_token_env_var {
                lines.push(format!(
                    "bearer_token_env_var = {}",
                    encode_toml_string(env_var)?
                ));
            }
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

fn render_claude_server_payload(
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

fn render_gemini_server_payload(
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

fn parse_rendered_server(rendered: &str) -> Result<ParsedServerSnippet> {
    if let Ok(file) = serde_json::from_str::<GenericMcpJsonFile>(rendered) {
        return extract_single_generic_server(file.mcp_servers);
    }
    if looks_like_gemini_snippet(rendered)
        && let Ok(file) = serde_json::from_str::<GeminiMcpJsonFile>(rendered)
    {
        return extract_single_gemini_server(file.mcp_servers);
    }
    if looks_like_claude_snippet(rendered)
        && let Ok(file) = serde_json::from_str::<ClaudeMcpJsonFile>(rendered)
    {
        return extract_single_claude_server(file.mcp_servers);
    }
    if let Ok(file) = serde_json::from_str::<CodexMcpJsonFile>(rendered) {
        return extract_single_codex_server(file.mcp_servers);
    }
    if let Ok(file) = serde_json::from_str::<ClaudeMcpJsonFile>(rendered) {
        return extract_single_claude_server(file.mcp_servers);
    }
    if let Ok(file) = serde_yaml::from_str::<GenericMcpJsonFile>(rendered) {
        return extract_single_generic_server(file.mcp_servers);
    }
    if looks_like_gemini_snippet(rendered)
        && let Ok(file) = serde_yaml::from_str::<GeminiMcpJsonFile>(rendered)
    {
        return extract_single_gemini_server(file.mcp_servers);
    }
    if looks_like_claude_snippet(rendered)
        && let Ok(file) = serde_yaml::from_str::<ClaudeMcpJsonFile>(rendered)
    {
        return extract_single_claude_server(file.mcp_servers);
    }
    if let Ok(file) = serde_yaml::from_str::<CodexMcpJsonFile>(rendered) {
        return extract_single_codex_server(file.mcp_servers);
    }
    if let Ok(file) = serde_yaml::from_str::<ClaudeMcpJsonFile>(rendered) {
        return extract_single_claude_server(file.mcp_servers);
    }
    if let Ok(file) = toml::from_str::<GenericMcpTomlFile>(rendered) {
        return extract_single_generic_server(file.mcp_servers);
    }
    if looks_like_gemini_snippet(rendered)
        && let Ok(file) = toml::from_str::<GeminiMcpTomlFile>(rendered)
    {
        return extract_single_gemini_server(file.mcp_servers);
    }
    if looks_like_claude_snippet(rendered)
        && let Ok(file) = toml::from_str::<ClaudeMcpTomlFile>(rendered)
    {
        return extract_single_claude_server(file.mcp_servers);
    }
    if let Ok(file) = toml::from_str::<CodexMcpTomlFile>(rendered) {
        let servers = file
            .mcp_servers
            .into_iter()
            .map(|(name, server)| Ok((name, server.into_codex_server()?)))
            .collect::<Result<BTreeMap<_, _>>>()?;
        return extract_single_codex_server(servers);
    }
    if let Ok(file) = toml::from_str::<ClaudeMcpTomlFile>(rendered) {
        return extract_single_claude_server(file.mcp_servers);
    }

    Err(AgentStowError::Mcp {
        message: "无法解析 MCP 片段：既不是通用片段，也不是 Codex/Claude/Gemini 片段".into(),
    })
}

fn extract_single_generic_server(
    servers: BTreeMap<String, McpServerDef>,
) -> Result<ParsedServerSnippet> {
    let mut iter = servers.into_iter();
    let (name, server) = iter.next().ok_or_else(|| AgentStowError::Mcp {
        message: "MCP 片段里没有 server".into(),
    })?;
    if iter.next().is_some() {
        return Err(AgentStowError::Mcp {
            message: "MCP 片段一次只能转换一个 server".into(),
        });
    }
    Ok(ParsedServerSnippet::Generic { name, server })
}

fn extract_single_codex_server(
    servers: BTreeMap<String, CodexMcpServer>,
) -> Result<ParsedServerSnippet> {
    let mut iter = servers.into_iter();
    let (name, server) = iter.next().ok_or_else(|| AgentStowError::Mcp {
        message: "MCP 片段里没有 server".into(),
    })?;
    if iter.next().is_some() {
        return Err(AgentStowError::Mcp {
            message: "MCP 片段一次只能转换一个 server".into(),
        });
    }
    Ok(ParsedServerSnippet::Codex { name, server })
}

fn extract_single_claude_server(
    servers: BTreeMap<String, ClaudeMcpServer>,
) -> Result<ParsedServerSnippet> {
    let mut iter = servers.into_iter();
    let (name, server) = iter.next().ok_or_else(|| AgentStowError::Mcp {
        message: "MCP 片段里没有 server".into(),
    })?;
    if iter.next().is_some() {
        return Err(AgentStowError::Mcp {
            message: "MCP 片段一次只能转换一个 server".into(),
        });
    }
    Ok(ParsedServerSnippet::Claude { name, server })
}

fn extract_single_gemini_server(
    servers: BTreeMap<String, GeminiMcpServer>,
) -> Result<ParsedServerSnippet> {
    let mut iter = servers.into_iter();
    let (name, server) = iter.next().ok_or_else(|| AgentStowError::Mcp {
        message: "MCP 片段里没有 server".into(),
    })?;
    if iter.next().is_some() {
        return Err(AgentStowError::Mcp {
            message: "MCP 片段一次只能转换一个 server".into(),
        });
    }
    Ok(ParsedServerSnippet::Gemini { name, server })
}

fn infer_snippet_format(rendered: &str) -> McpSnippetFormat {
    let trimmed = rendered.trim_start();
    if trimmed.starts_with('{') {
        McpSnippetFormat::Json
    } else if trimmed.starts_with("[mcp_servers.") {
        McpSnippetFormat::Toml
    } else {
        McpSnippetFormat::Yaml
    }
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
    if !oauth.scopes.is_empty() {
        lines.push(format!(
            "scopes = {}",
            encode_toml_string_array(&oauth.scopes)?
        ));
    }
    Ok(())
}

fn split_stdio_env_bindings(name: &str, envs: &[EnvVarDef]) -> Result<StdioEnvRender> {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EnvReferenceStyle {
    Claude,
    Gemini,
}

fn render_bound_env_map(
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

fn materialize_http_headers(
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

fn format_env_reference(var: &str, style: EnvReferenceStyle) -> String {
    match style {
        EnvReferenceStyle::Claude => format!("${{{var}}}"),
        EnvReferenceStyle::Gemini => format!("${var}"),
    }
}

fn split_http_env_bindings(
    name: &str,
    headers: &std::collections::HashMap<String, String>,
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

fn looks_like_bearer_env_var(var: &str) -> bool {
    let upper = var.to_ascii_uppercase();
    upper.contains("TOKEN")
        || upper.contains("API_KEY")
        || upper.contains("BEARER")
        || upper.contains("AUTH")
}

fn looks_like_gemini_snippet(rendered: &str) -> bool {
    [
        "httpUrl",
        "http_url",
        "includeTools",
        "include_tools",
        "excludeTools",
        "exclude_tools",
        "authProviderType",
        "auth_provider_type",
        "targetAudience",
        "target_audience",
        "targetServiceAccount",
        "target_service_account",
        "trust",
        "timeout",
        "cwd",
    ]
    .iter()
    .any(|marker| rendered.contains(marker))
}

fn looks_like_claude_snippet(rendered: &str) -> bool {
    [
        "\"type\"",
        "\ntype = ",
        "\ntype:",
        "clientId",
        "callbackPort",
        "authServerMetadataUrl",
    ]
    .iter()
    .any(|marker| rendered.contains(marker))
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
    fn into_codex_server(self) -> Result<CodexMcpServer> {
        if let Some(command) = self.command {
            return Ok(CodexMcpServer::Stdio {
                command,
                args: self.args,
                env_vars: self.env_vars,
                env: self.env,
                cwd: self.cwd,
            });
        }
        if let Some(url) = self.url {
            return Ok(CodexMcpServer::Http {
                url,
                bearer_token_env_var: self.bearer_token_env_var,
                http_headers: self.http_headers,
                env_http_headers: self.env_http_headers,
            });
        }
        Err(AgentStowError::Mcp {
            message: "TOML MCP 片段缺少 command/url，无法判断 transport".into(),
        })
    }
}

fn imported_env_map_to_defs(env: BTreeMap<String, String>) -> Vec<EnvVarDef> {
    env.into_iter()
        .map(|(key, value)| EnvVarDef {
            key,
            binding: SecretBinding::Literal { value },
        })
        .collect()
}

fn quote_shell_arg(part: &str) -> String {
    if part
        .chars()
        .any(|ch| ch.is_whitespace() || matches!(ch, '"' | '\'' | '\\'))
    {
        serde_json::to_string(part).unwrap_or_else(|_| format!("\"{part}\""))
    } else {
        part.to_string()
    }
}

#[cfg(test)]
mod tests;
