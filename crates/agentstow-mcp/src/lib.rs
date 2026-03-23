use std::collections::BTreeMap;

use agentstow_core::{AgentStowError, Result};
use agentstow_manifest::{EnvVarDef, McpServerDef, McpTransport};
use serde::{Deserialize, Serialize};
use tracing::instrument;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpJsonFile {
    #[serde(rename = "mcpServers")]
    pub mcp_servers: BTreeMap<String, McpJsonServer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum McpJsonServer {
    Stdio {
        command: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        args: Vec<String>,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        env: BTreeMap<String, String>,
    },
    Http {
        r#type: String,
        url: String,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        headers: BTreeMap<String, String>,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        env: BTreeMap<String, String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpSnippetFormat {
    Json,
    Toml,
    Yaml,
}

#[derive(Debug, Deserialize)]
struct McpTomlFile {
    #[serde(rename = "mcp_servers", default)]
    mcp_servers: BTreeMap<String, McpTomlServer>,
}

#[derive(Debug, Deserialize)]
struct McpTomlServer {
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    args: Vec<String>,
    #[serde(rename = "type", default)]
    type_name: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    headers: BTreeMap<String, String>,
    #[serde(default)]
    env: BTreeMap<String, String>,
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

    pub fn render_server_toml(name: &str, server: &McpServerDef) -> Result<String> {
        Self::validate_server(name, server)?;
        render_server_toml_payload(name, &render_server_payload(server))
    }

    pub fn render_server_snippet(
        name: &str,
        server: &McpServerDef,
        format: McpSnippetFormat,
    ) -> Result<String> {
        match format {
            McpSnippetFormat::Json => Self::render_server_json(name, server),
            McpSnippetFormat::Toml => Self::render_server_toml(name, server),
            McpSnippetFormat::Yaml => {
                render_server_yaml_payload(name, &render_server_payload(server))
            }
        }
    }

    pub fn convert_server_snippet(rendered: &str, format: McpSnippetFormat) -> Result<String> {
        let (name, server) = parse_rendered_server(rendered)?;
        match format {
            McpSnippetFormat::Json => render_server_json_payload(&name, &server),
            McpSnippetFormat::Toml => render_server_toml_payload(&name, &server),
            McpSnippetFormat::Yaml => render_server_yaml_payload(&name, &server),
        }
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

fn render_server_json_payload(name: &str, server: &McpJsonServer) -> Result<String> {
    let file = McpJsonFile {
        mcp_servers: BTreeMap::from([(name.to_string(), server.clone())]),
    };
    serde_json::to_string_pretty(&file).map_err(|e| AgentStowError::Mcp {
        message: format!("序列化 MCP JSON 失败：{e}").into(),
    })
}

fn render_server_toml_payload(name: &str, server: &McpJsonServer) -> Result<String> {
    let rendered_name = render_server_name(name);
    let mut lines = vec![format!("[mcp_servers.{rendered_name}]")];
    match server {
        McpJsonServer::Stdio { command, args, env } => {
            if !args.is_empty() {
                lines.push(format!("args = {}", encode_toml_string_array(args)?));
            }
            lines.push(format!("command = {}", encode_toml_string(command)?));
            append_env_block(&mut lines, &rendered_name, env.clone())?;
        }
        McpJsonServer::Http {
            r#type,
            url,
            headers,
            env,
        } => {
            lines.push(format!("type = {}", encode_toml_string(r#type)?));
            lines.push(format!("url = {}", encode_toml_string(url)?));
            if !headers.is_empty() {
                lines.push(String::new());
                lines.push(format!("[mcp_servers.{rendered_name}.headers]"));
                for (key, value) in headers {
                    lines.push(format!("{key} = {}", encode_toml_string(value)?));
                }
            }
            append_env_block(&mut lines, &rendered_name, env.clone())?;
        }
    }

    Ok(lines.join("\n") + "\n")
}

fn render_server_yaml_payload(name: &str, server: &McpJsonServer) -> Result<String> {
    let file = McpJsonFile {
        mcp_servers: BTreeMap::from([(name.to_string(), server.clone())]),
    };
    serde_yaml::to_string(&file).map_err(|e| AgentStowError::Mcp {
        message: format!("序列化 MCP YAML 失败：{e}").into(),
    })
}

fn parse_rendered_server(rendered: &str) -> Result<(String, McpJsonServer)> {
    if let Ok(file) = serde_json::from_str::<McpJsonFile>(rendered) {
        return extract_single_server(file.mcp_servers);
    }
    if let Ok(file) = serde_yaml::from_str::<McpJsonFile>(rendered) {
        return extract_single_server(file.mcp_servers);
    }
    if let Ok(file) = toml::from_str::<McpTomlFile>(rendered) {
        let servers = file
            .mcp_servers
            .into_iter()
            .map(|(name, server)| Ok((name, server.into_json_server()?)))
            .collect::<Result<BTreeMap<_, _>>>()?;
        return extract_single_server(servers);
    }

    Err(AgentStowError::Mcp {
        message: "无法解析 MCP 片段：既不是单 server JSON，也不是单 server TOML".into(),
    })
}

fn extract_single_server(
    servers: BTreeMap<String, McpJsonServer>,
) -> Result<(String, McpJsonServer)> {
    let mut iter = servers.into_iter();
    let first = iter.next().ok_or_else(|| AgentStowError::Mcp {
        message: "MCP 片段里没有 server".into(),
    })?;
    if iter.next().is_some() {
        return Err(AgentStowError::Mcp {
            message: "MCP 片段一次只能转换一个 server".into(),
        });
    }
    Ok(first)
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

impl McpTomlServer {
    fn into_json_server(self) -> Result<McpJsonServer> {
        if let Some(command) = self.command {
            return Ok(McpJsonServer::Stdio {
                command,
                args: self.args,
                env: self.env,
            });
        }
        if let Some(url) = self.url {
            return Ok(McpJsonServer::Http {
                r#type: self.type_name.unwrap_or_else(|| "http".to_string()),
                url,
                headers: self.headers,
                env: self.env,
            });
        }
        Err(AgentStowError::Mcp {
            message: "TOML MCP 片段缺少 command/url，无法判断 transport".into(),
        })
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
