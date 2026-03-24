use std::collections::BTreeMap;

use agentstow_core::{AgentStowError, Result};
use agentstow_manifest::{McpServerDef, McpTransport};
use tracing::instrument;

use crate::Mcp;
use crate::adapter::{
    render_codex_server, split_http_env_bindings, split_stdio_env_bindings, validate_generic_server,
};
use crate::parse::{infer_snippet_format, parse_rendered_server};
use crate::snippet::{render_codex_server_payload, render_generic_server_payload};
use crate::types::{
    CodexMcpJsonFile, CodexMcpServer, McpDryRunCheck, McpDryRunCheckStatus, McpSnippetFormat,
    McpTargetAdapter,
};

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
