use std::collections::BTreeMap;

use agentstow_core::{AgentStowError, Result, SecretBinding};
use agentstow_manifest::{EnvVarDef, Manifest};
use agentstow_web_types::{
    EnvEmitResponse, EnvUsageOwnerKindResponse, EnvUsageRefResponse, EnvVarSummaryResponse,
    SecretBindingKindResponse, ShellKindResponse,
};

use super::WorkspaceQueryService;
use super::common::shell_kind;

impl WorkspaceQueryService {
    pub(crate) fn env_emit(
        &self,
        set: Option<&str>,
        shell: ShellKindResponse,
    ) -> Result<EnvEmitResponse> {
        let manifest = self.load_manifest()?;
        let resolved = match set {
            Some(set) => {
                let env_set =
                    manifest
                        .env
                        .emit
                        .get(set)
                        .ok_or_else(|| AgentStowError::Manifest {
                            message: format!("env emit set 不存在：{set}").into(),
                        })?;
                agentstow_env::Env::resolve_env_set(env_set)?
            }
            None => agentstow_env::Env::resolve_context(&manifest.env, &manifest.workspace_root)?,
        };
        let text = agentstow_env::Env::emit_shell(shell_kind(shell), &resolved)?;
        Ok(EnvEmitResponse { text })
    }
}

pub(crate) fn build_env_usage_index(
    manifest: &Manifest,
) -> BTreeMap<String, Vec<EnvUsageRefResponse>> {
    let mut usage = BTreeMap::<String, Vec<EnvUsageRefResponse>>::new();

    for (env_set_id, env_set) in &manifest.env.emit {
        for env_var in &env_set.vars {
            usage
                .entry(env_var.key.clone())
                .or_default()
                .push(EnvUsageRefResponse {
                    owner_kind: EnvUsageOwnerKindResponse::EnvEmitSet,
                    owner_id: env_set_id.clone(),
                    label: format!("Env Emit Set · {env_set_id}"),
                });
        }
    }

    for (script_id, script) in &manifest.scripts {
        for env_var in &script.env {
            usage
                .entry(env_var.key.clone())
                .or_default()
                .push(EnvUsageRefResponse {
                    owner_kind: EnvUsageOwnerKindResponse::Script,
                    owner_id: script_id.clone(),
                    label: format!("Script · {script_id}"),
                });
        }
    }

    for (server_id, server) in &manifest.mcp_servers {
        for env_var in &server.env {
            usage
                .entry(env_var.key.clone())
                .or_default()
                .push(EnvUsageRefResponse {
                    owner_kind: EnvUsageOwnerKindResponse::McpServer,
                    owner_id: server_id.clone(),
                    label: format!("MCP · {server_id}"),
                });
        }
    }

    usage
}

pub(crate) fn build_env_var_summaries(
    envs: &[EnvVarDef],
    usage: &BTreeMap<String, Vec<EnvUsageRefResponse>>,
) -> Vec<EnvVarSummaryResponse> {
    envs.iter()
        .map(|env_var| {
            let (binding_kind, source_env_var, rendered_placeholder, available, diagnostic) =
                inspect_secret_binding(&env_var.binding);
            EnvVarSummaryResponse {
                key: env_var.key.clone(),
                binding: summarize_secret_binding(&env_var.binding),
                binding_kind,
                source_env_var,
                rendered_placeholder,
                available,
                diagnostic,
                referrers: usage.get(&env_var.key).cloned().unwrap_or_default(),
            }
        })
        .collect()
}

pub(crate) fn collect_env_referrers(vars: &[EnvVarSummaryResponse]) -> Vec<EnvUsageRefResponse> {
    let mut dedup = BTreeMap::new();
    for var in vars {
        for referrer in &var.referrers {
            dedup
                .entry(format!("{:?}:{}", referrer.owner_kind, referrer.owner_id))
                .or_insert_with(|| referrer.clone());
        }
    }
    dedup.into_values().collect()
}

pub(crate) fn inspect_secret_binding(
    binding: &SecretBinding,
) -> (
    SecretBindingKindResponse,
    Option<String>,
    String,
    bool,
    Option<String>,
) {
    match binding {
        SecretBinding::Literal { .. } => (
            SecretBindingKindResponse::Literal,
            None,
            "<literal>".to_string(),
            true,
            None,
        ),
        SecretBinding::Env { var } => {
            let available = std::env::var_os(var).is_some();
            (
                SecretBindingKindResponse::Env,
                Some(var.clone()),
                format!("${{{var}}}"),
                available,
                if available {
                    None
                } else {
                    Some(format!("缺少环境变量：{var}"))
                },
            )
        }
    }
}

fn summarize_secret_binding(binding: &SecretBinding) -> String {
    match binding {
        SecretBinding::Literal { .. } => "literal".to_string(),
        SecretBinding::Env { var } => format!("env:{var}"),
    }
}
