use std::collections::{BTreeMap, BTreeSet};

use agentstow_core::{ProfileName, Result, normalize_for_display};
use agentstow_manifest::{Manifest, McpTransport, Profile, ProfileVarSyntaxMode};
use agentstow_web_types::{
    ArtifactSummaryResponse, EnvEmitSetSummaryResponse, EnvUsageRefResponse, McpHeaderResponse,
    McpServerSummaryResponse, McpTransportKindResponse, ProfileSummaryResponse, ProfileVarResponse,
    ProfileVarSyntaxModeResponse, ScriptSummaryResponse, TargetSummaryResponse,
};

use super::common::{artifact_kind_response, install_method_response, validate_as_response};
use super::env::{build_env_var_summaries, collect_env_referrers};

pub(crate) fn build_target_summaries(manifest: &Manifest) -> Vec<TargetSummaryResponse> {
    manifest
        .targets
        .iter()
        .map(|(target_name, target)| TargetSummaryResponse {
            id: target_name.as_str().to_string(),
            artifact_id: target.artifact.as_str().to_string(),
            profile: target
                .profile
                .as_ref()
                .map(|profile| profile.as_str().to_string()),
            target_path: normalize_for_display(
                &target.absolute_target_path(&manifest.workspace_root),
            ),
            method: install_method_response(target.method),
        })
        .collect()
}

pub(crate) fn build_artifact_summaries(
    manifest: &Manifest,
    targets: &[TargetSummaryResponse],
) -> Vec<ArtifactSummaryResponse> {
    manifest
        .artifacts
        .iter()
        .map(|(artifact_id, artifact)| {
            let matched_targets: Vec<_> = targets
                .iter()
                .filter(|target| target.artifact_id == artifact_id.as_str())
                .collect();
            let profiles = matched_targets
                .iter()
                .filter_map(|target| target.profile.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            let target_ids = matched_targets
                .iter()
                .map(|target| target.id.clone())
                .collect();

            ArtifactSummaryResponse {
                id: artifact_id.as_str().to_string(),
                kind: artifact_kind_response(artifact.kind),
                source_path: normalize_for_display(&artifact.source_path(&manifest.workspace_root)),
                template: artifact.template,
                validate_as: validate_as_response(artifact.validate_as),
                target_ids,
                profiles,
            }
        })
        .collect()
}

pub(crate) fn build_profile_summaries(
    manifest: &Manifest,
    targets: &[TargetSummaryResponse],
) -> Vec<ProfileSummaryResponse> {
    manifest
        .profiles
        .iter()
        .map(|(profile_name, profile)| {
            let matched_targets: Vec<_> = targets
                .iter()
                .filter(|target| target.profile.as_deref() == Some(profile_name.as_str()))
                .collect();
            let target_ids = matched_targets
                .iter()
                .map(|target| target.id.clone())
                .collect();
            let artifact_ids = matched_targets
                .iter()
                .map(|target| target.artifact_id.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();

            ProfileSummaryResponse {
                id: profile_name.as_str().to_string(),
                extends: profile
                    .extends
                    .iter()
                    .map(|parent| parent.as_str().to_string())
                    .collect(),
                variable_keys: profile.vars.keys().cloned().collect(),
                target_ids,
                artifact_ids,
            }
        })
        .collect()
}

pub(crate) fn build_profile_vars(
    manifest: &Manifest,
    profile_name: &ProfileName,
) -> Result<Vec<ProfileVarResponse>> {
    let mut vars: Vec<_> = manifest
        .profile_vars(profile_name)?
        .into_iter()
        .map(|(key, value)| ProfileVarResponse {
            key,
            value_json: serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string()),
        })
        .collect();
    vars.sort_by(|left, right| left.key.cmp(&right.key));
    Ok(vars)
}

pub(crate) fn build_declared_profile_vars(profile: &Profile) -> Vec<ProfileVarResponse> {
    let mut vars: Vec<_> = profile
        .declared_vars()
        .iter()
        .map(|(key, value)| ProfileVarResponse {
            key: key.clone(),
            value_json: serde_json::to_string(value).unwrap_or_else(|_| "null".to_string()),
        })
        .collect();
    vars.sort_by(|left, right| left.key.cmp(&right.key));
    vars
}

pub(crate) fn profile_var_syntax_mode_response(
    mode: ProfileVarSyntaxMode,
) -> ProfileVarSyntaxModeResponse {
    match mode {
        ProfileVarSyntaxMode::Inline => ProfileVarSyntaxModeResponse::Inline,
        ProfileVarSyntaxMode::VarsObject => ProfileVarSyntaxModeResponse::VarsObject,
        ProfileVarSyntaxMode::Mixed => ProfileVarSyntaxModeResponse::Mixed,
    }
}

pub(crate) fn build_env_emit_set_summaries(
    manifest: &Manifest,
    usage: &BTreeMap<String, Vec<EnvUsageRefResponse>>,
) -> Vec<EnvEmitSetSummaryResponse> {
    manifest
        .env
        .emit
        .iter()
        .map(|(name, env_set)| {
            let vars = build_env_var_summaries(&env_set.vars, usage);
            EnvEmitSetSummaryResponse {
                id: name.clone(),
                available_count: vars.iter().filter(|var| var.available).count(),
                missing_count: vars.iter().filter(|var| !var.available).count(),
                referrers: collect_env_referrers(&vars),
                vars,
            }
        })
        .collect()
}

pub(crate) fn build_script_summaries(
    manifest: &Manifest,
    usage: &BTreeMap<String, Vec<EnvUsageRefResponse>>,
) -> Vec<ScriptSummaryResponse> {
    manifest
        .scripts
        .iter()
        .map(|(name, script)| {
            let env_bindings = build_env_var_summaries(&script.env, usage);
            ScriptSummaryResponse {
                id: name.clone(),
                kind: script.kind.clone(),
                entry: script.entry.clone(),
                args: script.args.clone(),
                env_keys: env_bindings.iter().map(|env| env.key.clone()).collect(),
                env_bindings,
                timeout_ms: script.timeout_ms,
            }
        })
        .collect()
}

pub(crate) fn build_mcp_server_summaries(
    manifest: &Manifest,
    usage: &BTreeMap<String, Vec<EnvUsageRefResponse>>,
) -> Vec<McpServerSummaryResponse> {
    manifest
        .mcp_servers
        .iter()
        .map(|(name, server)| {
            let (transport_kind, location, command, args, url, headers) = match &server.transport {
                McpTransport::Stdio { command, args } => (
                    McpTransportKindResponse::Stdio,
                    command.clone(),
                    Some(command.clone()),
                    args.clone(),
                    None,
                    Vec::new(),
                ),
                McpTransport::Http {
                    url,
                    headers: header_map,
                } => {
                    let mut headers: Vec<_> = header_map
                        .iter()
                        .map(|(key, value)| McpHeaderResponse {
                            key: key.clone(),
                            value: value.clone(),
                        })
                        .collect();
                    headers.sort_by(|left, right| left.key.cmp(&right.key));

                    (
                        McpTransportKindResponse::Http,
                        url.clone(),
                        None,
                        Vec::new(),
                        Some(url.clone()),
                        headers,
                    )
                }
            };
            let env_bindings = build_env_var_summaries(&server.env, usage);

            McpServerSummaryResponse {
                id: name.clone(),
                transport_kind,
                location,
                command,
                args,
                url,
                headers,
                env_keys: env_bindings.iter().map(|env| env.key.clone()).collect(),
                env_bindings,
            }
        })
        .collect()
}
