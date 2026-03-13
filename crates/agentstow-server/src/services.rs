use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use agentstow_core::{
    AgentStowDirs, AgentStowError, ArtifactId, ArtifactKind, InstallMethod, ProfileName, Result,
    SecretBinding, ValidateAs, normalize_for_display,
};
use agentstow_manifest::{Manifest, McpTransport};
use agentstow_render::Renderer;
use agentstow_state::{LinkInstanceRecord, StateDb};
use agentstow_validate::Validator;
use agentstow_web_types::{
    ArtifactDetailResponse, ArtifactKindResponse, ArtifactSummaryResponse, EnvSetSummaryResponse,
    EnvVarSummaryResponse, ImpactAnalysisResponse, ImpactSubjectKindResponse,
    InstallMethodResponse, LinkRecordResponse, LinkStatusResponseItem, ManifestResponse,
    McpServerSummaryResponse, McpTransportKindResponse, ProfileDetailResponse,
    ProfileSummaryResponse, ProfileVarResponse, RenderResponse, ScriptSummaryResponse,
    TargetSummaryResponse, ValidateAsResponse, ValidationIssueResponse, WatchModeResponse,
    WatchStatusResponse, WorkspaceCountsResponse, WorkspaceSummaryResponse,
};
use time::format_description::well_known::Rfc3339;

use crate::watch::{WatchMode, WatchStatusSnapshot};

#[derive(Debug, Clone)]
pub(crate) struct WorkspaceQueryService {
    workspace_root: PathBuf,
}

impl WorkspaceQueryService {
    pub(crate) fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    pub(crate) fn manifest_overview(&self) -> Result<ManifestResponse> {
        let manifest = self.load_manifest()?;
        Ok(ManifestResponse {
            workspace_root: normalize_for_display(&manifest.workspace_root),
            profiles: manifest
                .profiles
                .keys()
                .map(|profile| profile.as_str().to_string())
                .collect(),
            artifacts: manifest
                .artifacts
                .keys()
                .map(|artifact| artifact.as_str().to_string())
                .collect(),
            targets: manifest
                .targets
                .keys()
                .map(|target| target.as_str().to_string())
                .collect(),
        })
    }

    pub(crate) fn render_preview(
        &self,
        artifact_id: &ArtifactId,
        profile: &ProfileName,
    ) -> Result<RenderResponse> {
        let manifest = self.load_manifest()?;
        let rendered = Renderer::render_file(&manifest, artifact_id, profile)?;
        let artifact_def =
            manifest
                .artifacts
                .get(artifact_id)
                .ok_or_else(|| AgentStowError::Manifest {
                    message: format!("artifact 不存在：{}", artifact_id.as_str()).into(),
                })?;
        Validator::validate_rendered_file(artifact_def, &rendered.bytes)?;
        Ok(RenderResponse {
            text: String::from_utf8_lossy(&rendered.bytes).to_string(),
        })
    }

    pub(crate) fn link_records(&self) -> Result<Vec<LinkRecordResponse>> {
        let db = self.open_state_db()?;
        let records = db.list_link_instances(&self.workspace_root)?;
        Ok(records.into_iter().map(link_record_response).collect())
    }

    pub(crate) fn link_status(&self) -> Result<Vec<LinkStatusResponseItem>> {
        let manifest = self.load_manifest()?;
        self.compute_link_status(&manifest)
    }

    pub(crate) fn workspace_summary(&self) -> Result<WorkspaceSummaryResponse> {
        let manifest = self.load_manifest()?;
        let link_status = self.compute_link_status(&manifest)?;
        let targets = build_target_summaries(&manifest);
        let profiles = build_profile_summaries(&manifest, &targets);
        let artifacts = build_artifact_summaries(&manifest, &targets);
        let env_sets = build_env_set_summaries(&manifest);
        let scripts = build_script_summaries(&manifest);
        let mcp_servers = build_mcp_server_summaries(&manifest);
        let issues = collect_workspace_issues(&manifest, &targets, &link_status);

        let healthy_link_count = link_status.iter().filter(|item| item.ok).count();
        let unhealthy_link_count = link_status.len().saturating_sub(healthy_link_count);

        Ok(WorkspaceSummaryResponse {
            workspace_root: normalize_for_display(&manifest.workspace_root),
            counts: WorkspaceCountsResponse {
                profile_count: profiles.len(),
                artifact_count: artifacts.len(),
                target_count: targets.len(),
                env_set_count: env_sets.len(),
                script_count: scripts.len(),
                mcp_server_count: mcp_servers.len(),
                link_count: link_status.len(),
                healthy_link_count,
                unhealthy_link_count,
            },
            profiles,
            artifacts,
            targets,
            env_sets,
            scripts,
            mcp_servers,
            issues,
        })
    }

    pub(crate) fn artifact_detail(
        &self,
        artifact_id: &ArtifactId,
    ) -> Result<ArtifactDetailResponse> {
        let manifest = self.load_manifest()?;
        let link_status = self.compute_link_status(&manifest)?;
        let targets = build_target_summaries(&manifest);
        let profiles = build_profile_summaries(&manifest, &targets);
        let artifacts = build_artifact_summaries(&manifest, &targets);
        let issues = collect_workspace_issues(&manifest, &targets, &link_status);

        let artifact = artifacts
            .iter()
            .find(|artifact| artifact.id == artifact_id.as_str())
            .cloned()
            .ok_or_else(|| AgentStowError::Manifest {
                message: format!("artifact 不存在：{}", artifact_id.as_str()).into(),
            })?;
        let artifact_targets: Vec<_> = targets
            .iter()
            .filter(|target| target.artifact_id == artifact.id)
            .cloned()
            .collect();
        let profile_ids: BTreeSet<_> = artifact_targets
            .iter()
            .filter_map(|target| target.profile.clone())
            .collect();
        let related_profiles: Vec<_> = profiles
            .into_iter()
            .filter(|profile| profile_ids.contains(&profile.id))
            .collect();

        let subject_ids = collect_subject_ids(
            [artifact.id.clone()],
            artifact_targets.iter().map(|target| target.id.clone()),
            related_profiles.iter().map(|profile| profile.id.clone()),
        );

        Ok(ArtifactDetailResponse {
            artifact,
            targets: artifact_targets,
            profiles: related_profiles,
            issues: filter_issues(&issues, &subject_ids),
        })
    }

    pub(crate) fn profile_detail(
        &self,
        profile_name: &ProfileName,
    ) -> Result<ProfileDetailResponse> {
        let manifest = self.load_manifest()?;
        let link_status = self.compute_link_status(&manifest)?;
        let targets = build_target_summaries(&manifest);
        let profiles = build_profile_summaries(&manifest, &targets);
        let artifacts = build_artifact_summaries(&manifest, &targets);
        let issues = collect_workspace_issues(&manifest, &targets, &link_status);

        let profile = profiles
            .iter()
            .find(|profile| profile.id == profile_name.as_str())
            .cloned()
            .ok_or_else(|| AgentStowError::Manifest {
                message: format!("profile 不存在：{}", profile_name.as_str()).into(),
            })?;
        let merged_vars = build_profile_vars(&manifest, profile_name)?;
        let profile_targets: Vec<_> = targets
            .iter()
            .filter(|target| target.profile.as_deref() == Some(profile_name.as_str()))
            .cloned()
            .collect();
        let artifact_ids: BTreeSet<_> = profile_targets
            .iter()
            .map(|target| target.artifact_id.clone())
            .collect();
        let related_artifacts: Vec<_> = artifacts
            .into_iter()
            .filter(|artifact| artifact_ids.contains(&artifact.id))
            .collect();

        let subject_ids = collect_subject_ids(
            [profile.id.clone()],
            profile_targets.iter().map(|target| target.id.clone()),
            related_artifacts.iter().map(|artifact| artifact.id.clone()),
        );

        Ok(ProfileDetailResponse {
            profile,
            merged_vars,
            targets: profile_targets,
            artifacts: related_artifacts,
            issues: filter_issues(&issues, &subject_ids),
        })
    }

    pub(crate) fn impact_analysis(
        &self,
        artifact: Option<&ArtifactId>,
        profile: Option<&ProfileName>,
    ) -> Result<ImpactAnalysisResponse> {
        if artifact.is_none() && profile.is_none() {
            return Err(AgentStowError::InvalidArgs {
                message: "impact analysis 需要至少指定 artifact 或 profile".into(),
            });
        }

        let manifest = self.load_manifest()?;
        let link_status = self.compute_link_status(&manifest)?;
        let targets = build_target_summaries(&manifest);
        let profiles = build_profile_summaries(&manifest, &targets);
        let artifacts = build_artifact_summaries(&manifest, &targets);
        let issues = collect_workspace_issues(&manifest, &targets, &link_status);

        if let Some(artifact_id) = artifact
            && !manifest.artifacts.contains_key(artifact_id)
        {
            return Err(AgentStowError::Manifest {
                message: format!("artifact 不存在：{}", artifact_id.as_str()).into(),
            });
        }
        if let Some(profile_name) = profile
            && !manifest.profiles.contains_key(profile_name)
        {
            return Err(AgentStowError::Manifest {
                message: format!("profile 不存在：{}", profile_name.as_str()).into(),
            });
        }

        let affected_targets: Vec<_> = targets
            .iter()
            .filter(|target| match (artifact, profile) {
                (Some(artifact_id), Some(profile_name)) => {
                    target.artifact_id == artifact_id.as_str()
                        && target.profile.as_deref() == Some(profile_name.as_str())
                }
                (Some(artifact_id), None) => target.artifact_id == artifact_id.as_str(),
                (None, Some(profile_name)) => {
                    target.profile.as_deref() == Some(profile_name.as_str())
                }
                (None, None) => false,
            })
            .cloned()
            .collect();

        let mut artifact_ids: BTreeSet<String> = affected_targets
            .iter()
            .map(|target| target.artifact_id.clone())
            .collect();
        if let Some(artifact_id) = artifact {
            artifact_ids.insert(artifact_id.as_str().to_string());
        }

        let mut profile_ids: BTreeSet<String> = affected_targets
            .iter()
            .filter_map(|target| target.profile.clone())
            .collect();
        if let Some(profile_name) = profile {
            profile_ids.insert(profile_name.as_str().to_string());
        }

        let affected_artifacts: Vec<_> = artifacts
            .into_iter()
            .filter(|artifact| artifact_ids.contains(&artifact.id))
            .collect();
        let affected_profiles: Vec<_> = profiles
            .into_iter()
            .filter(|profile| profile_ids.contains(&profile.id))
            .collect();
        let affected_link_status: Vec<_> = link_status
            .into_iter()
            .filter(|status| match (artifact, profile) {
                (Some(artifact_id), Some(profile_name)) => {
                    status.artifact_id == artifact_id.as_str()
                        && status.profile == profile_name.as_str()
                }
                (Some(artifact_id), None) => status.artifact_id == artifact_id.as_str(),
                (None, Some(profile_name)) => status.profile == profile_name.as_str(),
                (None, None) => false,
            })
            .collect();

        let subject_kind = match (artifact, profile) {
            (Some(_), Some(_)) => ImpactSubjectKindResponse::ArtifactProfile,
            (Some(_), None) => ImpactSubjectKindResponse::Artifact,
            (None, Some(_)) => ImpactSubjectKindResponse::Profile,
            (None, None) => unreachable!(),
        };
        let subject_id = match (artifact, profile) {
            (Some(artifact_id), Some(profile_name)) => {
                format!("{}@{}", artifact_id.as_str(), profile_name.as_str())
            }
            (Some(artifact_id), None) => artifact_id.as_str().to_string(),
            (None, Some(profile_name)) => profile_name.as_str().to_string(),
            (None, None) => unreachable!(),
        };

        let subject_ids = collect_subject_ids(
            artifact_ids,
            affected_targets.iter().map(|target| target.id.clone()),
            profile_ids,
        );

        Ok(ImpactAnalysisResponse {
            subject_kind,
            subject_id,
            affected_targets,
            affected_artifacts,
            affected_profiles,
            link_status: affected_link_status,
            issues: filter_issues(&issues, &subject_ids),
        })
    }

    fn load_manifest(&self) -> Result<Manifest> {
        Manifest::load_from_dir(&self.workspace_root)
    }

    fn open_state_db(&self) -> Result<StateDb> {
        let dirs = AgentStowDirs::from_env()?;
        StateDb::open(&dirs)
    }

    fn compute_link_status(&self, manifest: &Manifest) -> Result<Vec<LinkStatusResponseItem>> {
        let db = self.open_state_db()?;
        let records = db.list_link_instances(&self.workspace_root)?;
        Ok(records
            .into_iter()
            .map(|record| link_status_item(manifest, &record))
            .collect())
    }
}

fn build_target_summaries(manifest: &Manifest) -> Vec<TargetSummaryResponse> {
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

fn build_artifact_summaries(
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

fn build_profile_summaries(
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

fn build_profile_vars(
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

fn build_env_set_summaries(manifest: &Manifest) -> Vec<EnvSetSummaryResponse> {
    manifest
        .env_sets
        .iter()
        .map(|(name, env_set)| EnvSetSummaryResponse {
            id: name.clone(),
            vars: env_set
                .vars
                .iter()
                .map(|env_var| EnvVarSummaryResponse {
                    key: env_var.key.clone(),
                    binding: summarize_secret_binding(&env_var.binding),
                })
                .collect(),
        })
        .collect()
}

fn build_script_summaries(manifest: &Manifest) -> Vec<ScriptSummaryResponse> {
    manifest
        .scripts
        .iter()
        .map(|(name, script)| ScriptSummaryResponse {
            id: name.clone(),
            kind: script.kind.clone(),
            entry: script.entry.clone(),
            args: script.args.clone(),
            env_keys: script.env.iter().map(|env| env.key.clone()).collect(),
            timeout_ms: script.timeout_ms,
        })
        .collect()
}

fn build_mcp_server_summaries(manifest: &Manifest) -> Vec<McpServerSummaryResponse> {
    manifest
        .mcp_servers
        .iter()
        .map(|(name, server)| {
            let (transport_kind, location) = match &server.transport {
                McpTransport::Stdio { command, .. } => {
                    (McpTransportKindResponse::Stdio, command.clone())
                }
                McpTransport::Http { url, .. } => (McpTransportKindResponse::Http, url.clone()),
            };

            McpServerSummaryResponse {
                id: name.clone(),
                transport_kind,
                location,
                env_keys: server.env.iter().map(|env| env.key.clone()).collect(),
            }
        })
        .collect()
}

fn collect_workspace_issues(
    manifest: &Manifest,
    targets: &[TargetSummaryResponse],
    link_status: &[LinkStatusResponseItem],
) -> Vec<ValidationIssueResponse> {
    let target_lookup: BTreeMap<_, _> = targets
        .iter()
        .map(|target| (target.id.clone(), target))
        .collect();
    let target_by_path: BTreeMap<_, _> = targets
        .iter()
        .map(|target| (target.target_path.clone(), target.id.clone()))
        .collect();

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
        if artifact.kind == ArtifactKind::File {
            match Renderer::render_file(manifest, &target.artifact, profile) {
                Ok(rendered) => {
                    if let Err(error) = Validator::validate_rendered_file(artifact, &rendered.bytes)
                    {
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
    }

    for status in link_status.iter().filter(|status| !status.ok) {
        let subject_id = target_by_path
            .get(&status.target_path)
            .cloned()
            .unwrap_or_else(|| status.target_path.clone());
        issues.push(issue(
            "error",
            "link",
            subject_id,
            "link_unhealthy",
            format!(
                "target `{}` 当前 link 状态不健康：{}",
                status.target_path, status.message
            ),
        ));
    }

    for target in targets {
        if !target_lookup.contains_key(&target.id) {
            issues.push(issue(
                "error",
                "target",
                &target.id,
                "target_not_indexed",
                format!("target `{}` 未能进入工作区索引", target.id),
            ));
        }
    }

    issues
}

fn collect_subject_ids<I, J, K>(primary: I, secondary: J, tertiary: K) -> BTreeSet<String>
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

fn filter_issues(
    issues: &[ValidationIssueResponse],
    subject_ids: &BTreeSet<String>,
) -> Vec<ValidationIssueResponse> {
    issues
        .iter()
        .filter(|issue| subject_ids.contains(&issue.subject_id))
        .cloned()
        .collect()
}

fn issue(
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

fn summarize_secret_binding(binding: &SecretBinding) -> String {
    match binding {
        SecretBinding::Literal { .. } => "literal".to_string(),
        SecretBinding::Env { var } => format!("env:{var}"),
    }
}

fn link_record_response(record: LinkInstanceRecord) -> LinkRecordResponse {
    LinkRecordResponse {
        artifact_id: record.artifact_id.as_str().to_string(),
        profile: record.profile.as_str().to_string(),
        target_path: normalize_for_display(&record.target_path),
        method: install_method_response(record.method),
        rendered_path: record
            .rendered_path
            .as_ref()
            .map(|path| normalize_for_display(path)),
        blake3: record.blake3,
        updated_at: record.updated_at.format(&Rfc3339).unwrap_or_default(),
    }
}

fn link_status_item(manifest: &Manifest, record: &LinkInstanceRecord) -> LinkStatusResponseItem {
    let target_path = normalize_for_display(&record.target_path);
    let method = install_method_response(record.method);
    let artifact_id = record.artifact_id.as_str().to_string();
    let profile = record.profile.as_str().to_string();

    let Some(artifact_def) = manifest.artifacts.get(&record.artifact_id) else {
        return LinkStatusResponseItem {
            artifact_id,
            profile,
            target_path,
            method,
            ok: false,
            message: "artifact_missing".to_string(),
        };
    };

    let status = match record.method {
        InstallMethod::Symlink => match record.rendered_path.as_ref() {
            Some(source_path) => {
                agentstow_linker::check_symlink(&record.target_path, source_path).unwrap_or(false)
            }
            None => false,
        },
        InstallMethod::Junction => match record.rendered_path.as_ref() {
            Some(source_path) => {
                agentstow_linker::check_junction(&record.target_path, source_path).unwrap_or(false)
            }
            None => false,
        },
        InstallMethod::Copy => match artifact_def.kind {
            ArtifactKind::File => {
                if !record.target_path.is_file() {
                    false
                } else {
                    match fs_err::read(&record.target_path).map_err(AgentStowError::from) {
                        Ok(existing_bytes) => match Renderer::render_file(
                            manifest,
                            &record.artifact_id,
                            &record.profile,
                        ) {
                            Ok(rendered) => existing_bytes == rendered.bytes,
                            Err(_) => false,
                        },
                        Err(_) => false,
                    }
                }
            }
            ArtifactKind::Dir => {
                let desired_source = artifact_def.source_path(&manifest.workspace_root);
                agentstow_linker::check_copy_dir(&record.target_path, &desired_source)
                    .unwrap_or(false)
            }
        },
    };

    LinkStatusResponseItem {
        artifact_id,
        profile,
        target_path,
        method,
        ok: status,
        message: if status {
            "healthy".to_string()
        } else {
            "unhealthy".to_string()
        },
    }
}

fn install_method_response(method: InstallMethod) -> InstallMethodResponse {
    match method {
        InstallMethod::Symlink => InstallMethodResponse::Symlink,
        InstallMethod::Junction => InstallMethodResponse::Junction,
        InstallMethod::Copy => InstallMethodResponse::Copy,
    }
}

fn artifact_kind_response(kind: ArtifactKind) -> ArtifactKindResponse {
    match kind {
        ArtifactKind::File => ArtifactKindResponse::File,
        ArtifactKind::Dir => ArtifactKindResponse::Dir,
    }
}

fn validate_as_response(validate_as: ValidateAs) -> ValidateAsResponse {
    match validate_as {
        ValidateAs::None => ValidateAsResponse::None,
        ValidateAs::Json => ValidateAsResponse::Json,
        ValidateAs::Toml => ValidateAsResponse::Toml,
        ValidateAs::Markdown => ValidateAsResponse::Markdown,
        ValidateAs::Shell => ValidateAsResponse::Shell,
    }
}

pub(crate) fn watch_status_response(snapshot: WatchStatusSnapshot) -> WatchStatusResponse {
    WatchStatusResponse {
        mode: match snapshot.mode {
            WatchMode::Native => WatchModeResponse::Native,
            WatchMode::Poll => WatchModeResponse::Poll,
            WatchMode::Manual => WatchModeResponse::Manual,
        },
        healthy: snapshot.healthy,
        revision: snapshot.revision,
        poll_interval_ms: snapshot.poll_interval_ms,
        last_event: snapshot.last_event,
        last_event_at: snapshot.last_event_at,
        last_error: snapshot.last_error,
        watch_roots: snapshot.watch_roots,
    }
}
