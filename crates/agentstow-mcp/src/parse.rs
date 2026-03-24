use std::collections::BTreeMap;

use agentstow_core::{AgentStowError, Result};
use agentstow_manifest::McpServerDef;

use crate::adapter::{
    codex_server_to_generic_def, render_claude_server, render_codex_server, render_gemini_server,
};
use crate::snippet::{
    render_claude_server_payload, render_codex_server_payload, render_gemini_server_payload,
    render_generic_server_payload,
};
use crate::types::{
    ClaudeMcpJsonFile, ClaudeMcpServer, ClaudeMcpTomlFile, CodexMcpJsonFile, CodexMcpServer,
    CodexMcpTomlFile, GeminiMcpJsonFile, GeminiMcpServer, GeminiMcpTomlFile, GenericMcpJsonFile,
    GenericMcpTomlFile, McpSnippetFormat, McpTargetAdapter,
};

#[derive(Debug, Clone)]
pub(crate) enum ParsedServerSnippet {
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
    pub(crate) fn format(&self, format: McpSnippetFormat) -> Result<String> {
        match self {
            Self::Generic { name, server } => render_generic_server_payload(name, server, format),
            Self::Codex { name, server } => render_codex_server_payload(name, server, format),
            Self::Claude { name, server } => render_claude_server_payload(name, server, format),
            Self::Gemini { name, server } => render_gemini_server_payload(name, server, format),
        }
    }

    pub(crate) fn adapt(self, target: McpTargetAdapter) -> Result<Self> {
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

pub(crate) fn parse_rendered_server(rendered: &str) -> Result<ParsedServerSnippet> {
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

pub(crate) fn infer_snippet_format(rendered: &str) -> McpSnippetFormat {
    let trimmed = rendered.trim_start();
    if trimmed.starts_with('{') {
        McpSnippetFormat::Json
    } else if trimmed.starts_with("[mcp_servers.") {
        McpSnippetFormat::Toml
    } else {
        McpSnippetFormat::Yaml
    }
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
