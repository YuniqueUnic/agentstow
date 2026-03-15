use std::collections::BTreeMap;

use agentstow_core::{AgentStowError, Result};
use agentstow_manifest::{EnvVarDef, McpServerDef, McpTransport};
use serde::Serialize;
use tracing::instrument;

#[derive(Debug, Serialize)]
pub struct McpJsonFile {
    #[serde(rename = "mcpServers")]
    pub mcp_servers: BTreeMap<String, McpJsonServer>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum McpJsonServer {
    Stdio {
        command: String,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        args: Vec<String>,
        #[serde(skip_serializing_if = "BTreeMap::is_empty")]
        env: BTreeMap<String, String>,
    },
    Http {
        r#type: String,
        url: String,
        #[serde(skip_serializing_if = "BTreeMap::is_empty")]
        headers: BTreeMap<String, String>,
        #[serde(skip_serializing_if = "BTreeMap::is_empty")]
        env: BTreeMap<String, String>,
    },
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

    pub fn render_mcp_json(servers: &BTreeMap<String, McpServerDef>) -> Result<String> {
        let mut out = BTreeMap::new();
        for (name, server) in servers {
            Self::validate_server(name, server)?;
            let rendered = render_server_payload(server);
            out.insert(name.clone(), rendered);
        }

        let file = McpJsonFile { mcp_servers: out };
        serde_json::to_string_pretty(&file).map_err(|e| AgentStowError::Mcp {
            message: format!("序列化 MCP JSON 失败：{e}").into(),
        })
    }

    pub fn render_server_json(name: &str, server: &McpServerDef) -> Result<String> {
        Self::validate_server(name, server)?;
        let file = McpJsonFile {
            mcp_servers: BTreeMap::from([(name.to_string(), render_server_payload(server))]),
        };
        serde_json::to_string_pretty(&file).map_err(|e| AgentStowError::Mcp {
            message: format!("序列化 MCP JSON 失败：{e}").into(),
        })
    }

    pub fn launcher_preview(server: &McpServerDef) -> String {
        match &server.transport {
            McpTransport::Stdio { command, args } => std::iter::once(command.as_str())
                .chain(args.iter().map(String::as_str))
                .map(quote_shell_arg)
                .collect::<Vec<_>>()
                .join(" "),
            McpTransport::Http { url, headers } => {
                let header_lines = headers
                    .iter()
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
                message: "基础 transport 校验通过".to_string(),
                detail: None,
            }),
            Err(error) => {
                checks.push(McpDryRunCheck {
                    code: "validate".to_string(),
                    status: McpDryRunCheckStatus::Error,
                    message: "基础 transport 校验失败".to_string(),
                    detail: Some(error.to_string()),
                });
                return checks;
            }
        }

        match &server.transport {
            McpTransport::Stdio { command, args } => {
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
            McpTransport::Http { url, headers } => {
                checks.push(McpDryRunCheck {
                    code: "endpoint".to_string(),
                    status: McpDryRunCheckStatus::Ok,
                    message: "HTTP endpoint 已解析".to_string(),
                    detail: Some(url.clone()),
                });
                checks.push(McpDryRunCheck {
                    code: "headers".to_string(),
                    status: if headers.is_empty() {
                        McpDryRunCheckStatus::Warn
                    } else {
                        McpDryRunCheckStatus::Ok
                    },
                    message: if headers.is_empty() {
                        "未声明 HTTP headers".to_string()
                    } else {
                        format!("已声明 {} 个 HTTP headers", headers.len())
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
            message: "单 server 配置可渲染".to_string(),
            detail: None,
        });

        checks
    }
}

fn bindings_to_env_map(envs: &[EnvVarDef]) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for EnvVarDef { key, binding } in envs {
        out.insert(key.clone(), binding.render_for_config());
    }
    out
}

fn render_server_payload(server: &McpServerDef) -> McpJsonServer {
    let env = bindings_to_env_map(&server.env);
    match &server.transport {
        McpTransport::Stdio { command, args } => McpJsonServer::Stdio {
            command: command.clone(),
            args: args.clone(),
            env,
        },
        McpTransport::Http { url, headers } => McpJsonServer::Http {
            r#type: "http".to_string(),
            url: url.clone(),
            headers: headers
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            env,
        },
    }
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
