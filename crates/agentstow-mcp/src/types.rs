use std::collections::BTreeMap;

use agentstow_manifest::McpServerDef;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GenericMcpJsonFile {
    #[serde(rename = "mcpServers")]
    pub(crate) mcp_servers: BTreeMap<String, McpServerDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GenericMcpTomlFile {
    #[serde(rename = "mcp_servers")]
    pub(crate) mcp_servers: BTreeMap<String, McpServerDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CodexMcpJsonFile {
    #[serde(rename = "mcpServers")]
    pub(crate) mcp_servers: BTreeMap<String, CodexMcpServer>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct CodexMcpServerOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) startup_timeout_sec: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) tool_timeout_sec: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) required: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) enabled_tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) disabled_tools: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub(crate) enum CodexMcpServer {
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
        #[serde(flatten)]
        options: CodexMcpServerOptions,
    },
    Http {
        url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bearer_token_env_var: Option<String>,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        http_headers: BTreeMap<String, String>,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        env_http_headers: BTreeMap<String, String>,
        #[serde(flatten)]
        options: CodexMcpServerOptions,
    },
}

#[derive(Debug, Deserialize)]
pub(crate) struct CodexMcpTomlFile {
    #[serde(rename = "mcp_servers", default)]
    pub(crate) mcp_servers: BTreeMap<String, CodexMcpTomlServer>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CodexMcpTomlServer {
    #[serde(default)]
    pub(crate) command: Option<String>,
    #[serde(default)]
    pub(crate) args: Vec<String>,
    #[serde(default)]
    pub(crate) env_vars: Vec<String>,
    #[serde(default)]
    pub(crate) url: Option<String>,
    #[serde(default)]
    pub(crate) cwd: Option<String>,
    #[serde(default)]
    pub(crate) bearer_token_env_var: Option<String>,
    #[serde(default)]
    pub(crate) http_headers: BTreeMap<String, String>,
    #[serde(default)]
    pub(crate) env_http_headers: BTreeMap<String, String>,
    #[serde(default)]
    pub(crate) env: BTreeMap<String, String>,
    #[serde(flatten)]
    pub(crate) options: CodexMcpServerOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ClaudeMcpJsonFile {
    #[serde(rename = "mcpServers")]
    pub(crate) mcp_servers: BTreeMap<String, ClaudeMcpServer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ClaudeMcpTomlFile {
    #[serde(rename = "mcp_servers", default)]
    pub(crate) mcp_servers: BTreeMap<String, ClaudeMcpServer>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClaudeMcpOAuthConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) client_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) callback_port: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) auth_server_metadata_url: Option<String>,
}

impl ClaudeMcpOAuthConfig {
    pub(crate) fn is_empty(&self) -> bool {
        self.client_id.is_none()
            && self.callback_port.is_none()
            && self.auth_server_metadata_url.is_none()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClaudeMcpServer {
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub(crate) transport_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) command: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) args: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) env: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) url: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) headers: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) oauth: Option<ClaudeMcpOAuthConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GeminiMcpJsonFile {
    #[serde(rename = "mcpServers")]
    pub(crate) mcp_servers: BTreeMap<String, GeminiMcpServer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GeminiMcpTomlFile {
    #[serde(rename = "mcp_servers", default)]
    pub(crate) mcp_servers: BTreeMap<String, GeminiMcpServer>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GeminiMcpOAuthConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) client_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) client_secret: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) authorization_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) token_url: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) scopes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) redirect_uri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) token_param_name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) audiences: Vec<String>,
}

impl GeminiMcpOAuthConfig {
    pub(crate) fn is_empty(&self) -> bool {
        self.enabled.is_none()
            && self.client_id.is_none()
            && self.client_secret.is_none()
            && self.authorization_url.is_none()
            && self.token_url.is_none()
            && self.scopes.is_empty()
            && self.redirect_uri.is_none()
            && self.token_param_name.is_none()
            && self.audiences.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GeminiMcpServer {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) command: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) args: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) env: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) cwd: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) http_url: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) headers: BTreeMap<String, String>,
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub(crate) transport_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) timeout: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) trust: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) include_tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) exclude_tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) oauth: Option<GeminiMcpOAuthConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) auth_provider_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) target_audience: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) target_service_account: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct StdioEnvRender {
    pub(crate) env: BTreeMap<String, String>,
    pub(crate) env_vars: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct HttpBindingRender {
    pub(crate) bearer_token_env_var: Option<String>,
    pub(crate) http_headers: BTreeMap<String, String>,
    pub(crate) env_http_headers: BTreeMap<String, String>,
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
    pub(crate) fn as_str(self) -> &'static str {
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
