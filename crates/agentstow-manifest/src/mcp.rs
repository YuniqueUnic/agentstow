use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use agentstow_core::{AgentStowError, Result, SecretBinding, absolutize, normalize_for_display};
use serde::{Deserialize, Serialize};

use crate::EnvVarDef;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerDef {
    pub transport: McpTransport,
    #[serde(default)]
    pub env: Vec<EnvVarDef>,
    #[serde(default, skip_serializing_if = "McpServerOptions::is_empty")]
    pub options: McpServerOptions,
}

impl McpServerDef {
    pub fn env_binding_defs(&self) -> Vec<EnvVarDef> {
        self.env.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct McpServerOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub startup_timeout_sec: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_timeout_sec: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enabled_tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disabled_tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trust: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub include_tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude_tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oauth: Option<McpOauthDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_provider_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_audience: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_service_account: Option<String>,
}

impl McpServerOptions {
    pub fn is_empty(&self) -> bool {
        self.startup_timeout_sec.is_none()
            && self.tool_timeout_sec.is_none()
            && self.enabled.is_none()
            && self.required.is_none()
            && self.enabled_tools.is_empty()
            && self.disabled_tools.is_empty()
            && self.timeout.is_none()
            && self.trust.is_none()
            && self.description.is_none()
            && self.include_tools.is_empty()
            && self.exclude_tools.is_empty()
            && self
                .oauth
                .as_ref()
                .map(McpOauthDef::is_empty)
                .unwrap_or(true)
            && self.auth_provider_type.is_none()
            && self.target_audience.is_none()
            && self.target_service_account.is_none()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct McpOauthDef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub callback_port: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_server_metadata_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authorization_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_url: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scopes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub redirect_uri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_param_name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub audiences: Vec<String>,
}

impl McpOauthDef {
    pub fn is_empty(&self) -> bool {
        self.client_id.is_none()
            && self.callback_port.is_none()
            && self.auth_server_metadata_url.is_none()
            && self.enabled.is_none()
            && self.client_secret.is_none()
            && self.authorization_url.is_none()
            && self.token_url.is_none()
            && self.scopes.is_empty()
            && self.redirect_uri.is_none()
            && self.token_param_name.is_none()
            && self.audiences.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum McpTransport {
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        cwd: Option<PathBuf>,
    },
    Http {
        url: String,
        #[serde(default)]
        headers: HashMap<String, String>,
    },
}

#[derive(Debug, Deserialize)]
struct McpServerImportDef {
    path: PathBuf,
}

#[derive(Debug, Deserialize)]
struct ImportedGenericMcpJsonFile {
    #[serde(rename = "mcpServers", default)]
    mcp_servers: BTreeMap<String, McpServerDef>,
}

#[derive(Debug, Deserialize)]
struct ImportedMcpJsonFile {
    #[serde(rename = "mcpServers", default)]
    mcp_servers: BTreeMap<String, ImportedMcpJsonServer>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ImportedMcpJsonServer {
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env_vars: Vec<String>,
        #[serde(default)]
        env: BTreeMap<String, String>,
        #[serde(default)]
        cwd: Option<PathBuf>,
        #[serde(flatten)]
        options: ImportedMcpServerOptions,
    },
    Http {
        url: String,
        #[serde(default)]
        bearer_token_env_var: Option<String>,
        #[serde(default)]
        http_headers: HashMap<String, String>,
        #[serde(default)]
        env_http_headers: HashMap<String, String>,
        #[serde(flatten)]
        options: ImportedMcpServerOptions,
    },
}

#[derive(Debug, Deserialize, Default)]
struct ImportedMcpServerOptions {
    #[serde(default)]
    startup_timeout_sec: Option<u64>,
    #[serde(default)]
    tool_timeout_sec: Option<u64>,
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    required: Option<bool>,
    #[serde(default)]
    enabled_tools: Vec<String>,
    #[serde(default)]
    disabled_tools: Vec<String>,
}

pub(crate) fn resolve_mcp_servers(
    raw: &toml::Table,
    workspace_root: &Path,
) -> Result<BTreeMap<String, McpServerDef>> {
    let mut servers = BTreeMap::new();

    for (name, value) in raw {
        if name == "file" {
            let import = value
                .clone()
                .try_into::<McpServerImportDef>()
                .map_err(|error| AgentStowError::Manifest {
                    message: format!("解析 mcp_servers.file 失败：{error}").into(),
                })?;
            import_mcp_servers_from_file(&mut servers, workspace_root, &import.path)?;
            continue;
        }

        let server =
            value
                .clone()
                .try_into::<McpServerDef>()
                .map_err(|error| AgentStowError::Manifest {
                    message: format!("解析 mcp_servers.{name} 失败：{error}").into(),
                })?;
        if servers.insert(name.clone(), server).is_some() {
            return Err(AgentStowError::Manifest {
                message: format!("重复的 mcp server 名称：{name}").into(),
            });
        }
    }

    Ok(servers)
}

fn import_mcp_servers_from_file(
    servers: &mut BTreeMap<String, McpServerDef>,
    workspace_root: &Path,
    path: &Path,
) -> Result<()> {
    let absolute_path = absolutize(workspace_root, path);
    let content = std::fs::read_to_string(&absolute_path).map_err(AgentStowError::from)?;
    if let Ok(imported) = serde_json::from_str::<ImportedGenericMcpJsonFile>(&content) {
        for (name, server) in imported.mcp_servers {
            if servers.insert(name.clone(), server).is_some() {
                return Err(AgentStowError::Manifest {
                    message: format!("导入的 mcp server 与现有名称冲突：{name}").into(),
                });
            }
        }
        return Ok(());
    }

    let imported: ImportedMcpJsonFile =
        serde_json::from_str(&content).map_err(|error| AgentStowError::Manifest {
            message: format!(
                "解析 mcp server 导入文件失败：path={}, {error}",
                normalize_for_display(&absolute_path),
            )
            .into(),
        })?;

    for (name, server) in imported.mcp_servers {
        if servers.contains_key(&name) {
            return Err(AgentStowError::Manifest {
                message: format!("导入的 mcp server 与现有名称冲突：{name}").into(),
            });
        }
        servers.insert(name, imported_mcp_server_to_def(server));
    }

    Ok(())
}

fn imported_mcp_server_to_def(server: ImportedMcpJsonServer) -> McpServerDef {
    match server {
        ImportedMcpJsonServer::Stdio {
            command,
            args,
            env_vars,
            env,
            cwd,
            options,
        } => McpServerDef {
            transport: McpTransport::Stdio { command, args, cwd },
            env: imported_env_map_to_defs(env)
                .into_iter()
                .chain(env_vars.into_iter().map(|var| EnvVarDef {
                    key: var.clone(),
                    binding: SecretBinding::Env { var },
                }))
                .collect(),
            options: imported_mcp_server_options_to_def(options),
        },
        ImportedMcpJsonServer::Http {
            url,
            bearer_token_env_var,
            http_headers,
            env_http_headers,
            options,
        } => McpServerDef {
            transport: McpTransport::Http {
                url,
                headers: http_headers,
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
            options: imported_mcp_server_options_to_def(options),
        },
    }
}

fn imported_mcp_server_options_to_def(options: ImportedMcpServerOptions) -> McpServerOptions {
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

fn imported_env_map_to_defs(env: BTreeMap<String, String>) -> Vec<EnvVarDef> {
    env.into_iter()
        .map(|(key, value)| EnvVarDef {
            key,
            binding: SecretBinding::Literal { value },
        })
        .collect()
}
