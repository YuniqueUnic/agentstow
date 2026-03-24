use std::collections::BTreeMap;
use std::collections::BTreeSet;

use agentstow_core::ArtifactKind;
use agentstow_manifest::Manifest;
use agentstow_mcp::Mcp;
use agentstow_render::Renderer;
use agentstow_validate::Validator;
use agentstow_web_types::{LinkStatusResponseItem, TargetSummaryResponse, ValidationIssueResponse};

use super::env::inspect_secret_binding;

pub(crate) fn collect_workspace_issues(
    manifest: &Manifest,
    targets: &[TargetSummaryResponse],
    link_status: &[LinkStatusResponseItem],
) -> Vec<ValidationIssueResponse> {
    let mut issues = Vec::new();
    issues.extend(collect_target_render_issues(manifest));
    issues.extend(collect_env_emit_set_binding_issues(manifest));
    issues.extend(collect_script_binding_issues(manifest));
    issues.extend(collect_mcp_issues(manifest));
    issues.extend(collect_link_issues(targets, link_status));

    issues
}

fn collect_target_render_issues(manifest: &Manifest) -> Vec<ValidationIssueResponse> {
    let mut issues = Vec::new();

    for (target_name, target) in &manifest.targets {
        if target.profile.is_none() {
            issues.push(issue(
                "warning",
                "target",
                target_name.as_str(),
                "target_profile_missing",
                format!(
                    "target `{}` 未声明 profile，服务端无法直接给出确定的 render/link 结果",
                    target_name.as_str()
                ),
            ));
            continue;
        }

        let profile = target.profile.as_ref().expect("checked above");
        let artifact = manifest
            .artifacts
            .get(&target.artifact)
            .expect("manifest already validated");
        if artifact.kind != ArtifactKind::File {
            continue;
        }

        match Renderer::render_file(manifest, &target.artifact, profile) {
            Ok(rendered) => {
                if let Err(error) = Validator::validate_rendered_file(artifact, &rendered.bytes) {
                    issues.push(issue(
                        "error",
                        "target",
                        target_name.as_str(),
                        "render_validation_failed",
                        error.to_string(),
                    ));
                }
            }
            Err(error) => issues.push(issue(
                "error",
                "target",
                target_name.as_str(),
                "render_failed",
                error.to_string(),
            )),
        }
    }

    issues
}

fn collect_env_emit_set_binding_issues(manifest: &Manifest) -> Vec<ValidationIssueResponse> {
    collect_binding_issues(manifest.env.emit.iter().flat_map(|(env_set_id, env_set)| {
        env_set.vars.iter().map(move |env_var| {
            (
                "env_emit_set",
                env_set_id.as_str(),
                "env_binding_unavailable",
                env_var,
            )
        })
    }))
}

fn collect_script_binding_issues(manifest: &Manifest) -> Vec<ValidationIssueResponse> {
    collect_binding_issues(manifest.scripts.iter().flat_map(|(script_id, script)| {
        script.env.iter().map(move |env_var| {
            (
                "script",
                script_id.as_str(),
                "script_env_unavailable",
                env_var,
            )
        })
    }))
}

fn collect_mcp_issues(manifest: &Manifest) -> Vec<ValidationIssueResponse> {
    let mut issues = Vec::new();

    for (server_id, server) in &manifest.mcp_servers {
        if let Err(error) = Mcp::validate_server(server_id, server) {
            issues.push(issue(
                "error",
                "mcp_server",
                server_id,
                "mcp_invalid",
                error.to_string(),
            ));
        }
    }

    for (server_id, server) in &manifest.mcp_servers {
        issues.extend(collect_binding_issues(
            server.env_binding_defs().iter().map(|env_var| {
                (
                    "mcp_server",
                    server_id.as_str(),
                    "mcp_env_unavailable",
                    env_var,
                )
            }),
        ));
    }

    issues
}

fn collect_binding_issues<'a>(
    bindings: impl IntoIterator<
        Item = (
            &'static str,
            &'a str,
            &'static str,
            &'a agentstow_manifest::EnvVarDef,
        ),
    >,
) -> Vec<ValidationIssueResponse> {
    bindings
        .into_iter()
        .filter_map(|(scope, subject_id, code, env_var)| {
            inspect_secret_binding(&env_var.binding).4.map(|message| {
                issue(
                    "warning",
                    scope,
                    subject_id,
                    code,
                    format!("{}: {}", env_var.key, message),
                )
            })
        })
        .collect()
}

fn collect_link_issues(
    targets: &[TargetSummaryResponse],
    link_status: &[LinkStatusResponseItem],
) -> Vec<ValidationIssueResponse> {
    let target_by_path: BTreeMap<_, _> = targets
        .iter()
        .map(|target| (target.target_path.clone(), target.id.clone()))
        .collect();

    link_status
        .iter()
        .filter(|status| !status.ok)
        .map(|status| {
            let subject_id = target_by_path
                .get(&status.target_path)
                .cloned()
                .unwrap_or_else(|| status.target_path.clone());
            issue(
                "error",
                "link",
                subject_id,
                "link_unhealthy",
                format!(
                    "target `{}` 当前 link 状态不健康：{}",
                    status.target_path, status.message
                ),
            )
        })
        .collect()
}

pub(crate) fn collect_subject_ids<I, J, K>(
    primary: I,
    secondary: J,
    tertiary: K,
) -> BTreeSet<String>
where
    I: IntoIterator<Item = String>,
    J: IntoIterator<Item = String>,
    K: IntoIterator<Item = String>,
{
    primary
        .into_iter()
        .chain(secondary)
        .chain(tertiary)
        .collect()
}

pub(crate) fn filter_issues(
    issues: &[ValidationIssueResponse],
    subject_ids: &BTreeSet<String>,
) -> Vec<ValidationIssueResponse> {
    issues
        .iter()
        .filter(|issue| subject_ids.contains(&issue.subject_id))
        .cloned()
        .collect()
}

pub(crate) fn issue(
    severity: impl Into<String>,
    scope: impl Into<String>,
    subject_id: impl Into<String>,
    code: impl Into<String>,
    message: impl Into<String>,
) -> ValidationIssueResponse {
    ValidationIssueResponse {
        severity: severity.into(),
        scope: scope.into(),
        subject_id: subject_id.into(),
        code: code.into(),
        message: message.into(),
    }
}
