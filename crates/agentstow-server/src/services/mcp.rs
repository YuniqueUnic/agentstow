use agentstow_core::{AgentStowError, Result};
use agentstow_manifest::McpTransport;
use agentstow_mcp::Mcp;
use agentstow_web_types::{
    McpCheckResponse, McpCheckStatusResponse, McpRenderResponse, McpTestResponse,
    McpTransportKindResponse, McpValidateResponse,
};

use super::WorkspaceQueryService;
use super::common::mcp_check_status_response;
use super::env::{build_env_usage_index, build_env_var_summaries};
use super::issues::issue;

impl WorkspaceQueryService {
    pub(crate) fn mcp_validate(&self, server_id: &str) -> Result<McpValidateResponse> {
        let manifest = self.load_manifest()?;
        let env_usage = build_env_usage_index(&manifest);
        let server =
            manifest
                .mcp_servers
                .get(server_id)
                .ok_or_else(|| AgentStowError::Manifest {
                    message: format!("mcp server 不存在：{server_id}").into(),
                })?;
        let env_bindings = build_env_var_summaries(&server.env, &env_usage);
        let mut issues = Vec::new();

        if let Err(error) = Mcp::validate_server(server_id, server) {
            issues.push(issue(
                "error",
                "mcp_server",
                server_id,
                "mcp_invalid",
                error.to_string(),
            ));
        }

        for binding in &env_bindings {
            if let Some(message) = &binding.diagnostic {
                issues.push(issue(
                    "warning",
                    "mcp_server",
                    server_id,
                    "mcp_env_unavailable",
                    format!("{}: {}", binding.key, message),
                ));
            }
        }

        Ok(McpValidateResponse {
            server_id: server_id.to_string(),
            ok: !issues.iter().any(|item| item.severity == "error"),
            issues,
        })
    }

    pub(crate) fn mcp_render(&self, server_id: &str) -> Result<McpRenderResponse> {
        let manifest = self.load_manifest()?;
        let env_usage = build_env_usage_index(&manifest);
        let server =
            manifest
                .mcp_servers
                .get(server_id)
                .ok_or_else(|| AgentStowError::Manifest {
                    message: format!("mcp server 不存在：{server_id}").into(),
                })?;
        let env_bindings = build_env_var_summaries(&server.env, &env_usage);

        Ok(McpRenderResponse {
            server_id: server_id.to_string(),
            transport_kind: match server.transport {
                McpTransport::Stdio { .. } => McpTransportKindResponse::Stdio,
                McpTransport::Http { .. } => McpTransportKindResponse::Http,
            },
            launcher_preview: Mcp::launcher_preview(server),
            config_json: Mcp::render_server_json(server_id, server)?,
            env_bindings,
        })
    }

    pub(crate) fn mcp_test(&self, server_id: &str) -> Result<McpTestResponse> {
        let manifest = self.load_manifest()?;
        let env_usage = build_env_usage_index(&manifest);
        let server =
            manifest
                .mcp_servers
                .get(server_id)
                .ok_or_else(|| AgentStowError::Manifest {
                    message: format!("mcp server 不存在：{server_id}").into(),
                })?;
        let env_bindings = build_env_var_summaries(&server.env, &env_usage);
        let mut checks: Vec<McpCheckResponse> = Mcp::test_server_dry_run(server_id, server)
            .into_iter()
            .map(|check| McpCheckResponse {
                code: check.code,
                status: mcp_check_status_response(check.status),
                message: check.message,
                detail: check.detail,
            })
            .collect();

        for binding in &env_bindings {
            checks.push(McpCheckResponse {
                code: format!("env:{}", binding.key),
                status: if binding.available {
                    McpCheckStatusResponse::Ok
                } else {
                    McpCheckStatusResponse::Error
                },
                message: if binding.available {
                    format!("环境变量 `{}` 可用", binding.key)
                } else {
                    format!("环境变量 `{}` 缺失", binding.key)
                },
                detail: binding.diagnostic.clone(),
            });
        }

        Ok(McpTestResponse {
            server_id: server_id.to_string(),
            ok: !checks
                .iter()
                .any(|check| matches!(check.status, McpCheckStatusResponse::Error)),
            checks,
        })
    }
}
