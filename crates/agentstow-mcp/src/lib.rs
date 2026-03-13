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
            let env = bindings_to_env_map(&server.env);
            let rendered = match &server.transport {
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
            };
            out.insert(name.clone(), rendered);
        }

        let file = McpJsonFile { mcp_servers: out };
        serde_json::to_string_pretty(&file).map_err(|e| AgentStowError::Mcp {
            message: format!("序列化 MCP JSON 失败：{e}").into(),
        })
    }
}

fn bindings_to_env_map(envs: &[EnvVarDef]) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for EnvVarDef { key, binding } in envs {
        out.insert(key.clone(), binding.render_for_config());
    }
    out
}

#[cfg(test)]
mod tests;
