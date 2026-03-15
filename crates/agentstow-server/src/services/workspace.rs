use std::collections::BTreeSet;

use agentstow_core::{AgentStowError, ArtifactId, ProfileName, Result, normalize_for_display};
use agentstow_manifest::Manifest;
use agentstow_web_types::{
    ArtifactDetailResponse, ArtifactSummaryResponse, ImpactAnalysisResponse,
    ImpactSubjectKindResponse, LinkStatusResponseItem, ManifestResponse, ProfileDetailResponse,
    ProfileSummaryResponse, TargetSummaryResponse, ValidationIssueResponse,
    WorkspaceCountsResponse, WorkspaceSummaryResponse,
};

use super::WorkspaceQueryService;
use super::env::build_env_usage_index;
use super::issues::{collect_subject_ids, collect_workspace_issues, filter_issues};
use super::summary::{
    build_artifact_summaries, build_declared_profile_vars, build_env_set_summaries,
    build_mcp_server_summaries, build_profile_summaries, build_profile_vars,
    build_script_summaries, build_target_summaries, profile_var_syntax_mode_response,
};

struct WorkspaceProjection {
    manifest: Manifest,
    link_status: Vec<LinkStatusResponseItem>,
    targets: Vec<TargetSummaryResponse>,
    profiles: Vec<ProfileSummaryResponse>,
    artifacts: Vec<ArtifactSummaryResponse>,
    issues: Vec<ValidationIssueResponse>,
}

impl WorkspaceProjection {
    fn load(service: &WorkspaceQueryService) -> Result<Self> {
        let manifest = service.load_manifest()?;
        let link_status = service.compute_link_status(&manifest)?;
        let targets = build_target_summaries(&manifest);
        let profiles = build_profile_summaries(&manifest, &targets);
        let artifacts = build_artifact_summaries(&manifest, &targets);
        let issues = collect_workspace_issues(&manifest, &targets, &link_status);

        Ok(Self {
            manifest,
            link_status,
            targets,
            profiles,
            artifacts,
            issues,
        })
    }
}

impl WorkspaceQueryService {
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

    pub(crate) fn workspace_summary(&self) -> Result<WorkspaceSummaryResponse> {
        let projection = WorkspaceProjection::load(self)?;
        let env_usage = build_env_usage_index(&projection.manifest);
        let env_sets = build_env_set_summaries(&projection.manifest, &env_usage);
        let scripts = build_script_summaries(&projection.manifest, &env_usage);
        let mcp_servers = build_mcp_server_summaries(&projection.manifest, &env_usage);
        let healthy_link_count = projection.link_status.iter().filter(|item| item.ok).count();
        let unhealthy_link_count = projection
            .link_status
            .len()
            .saturating_sub(healthy_link_count);

        Ok(WorkspaceSummaryResponse {
            workspace_root: normalize_for_display(&projection.manifest.workspace_root),
            counts: WorkspaceCountsResponse {
                profile_count: projection.profiles.len(),
                artifact_count: projection.artifacts.len(),
                target_count: projection.targets.len(),
                env_set_count: env_sets.len(),
                script_count: scripts.len(),
                mcp_server_count: mcp_servers.len(),
                link_count: projection.link_status.len(),
                healthy_link_count,
                unhealthy_link_count,
            },
            profiles: projection.profiles,
            artifacts: projection.artifacts,
            targets: projection.targets,
            env_sets,
            scripts,
            mcp_servers,
            issues: projection.issues,
        })
    }

    pub(crate) fn artifact_detail(
        &self,
        artifact_id: &ArtifactId,
    ) -> Result<ArtifactDetailResponse> {
        let projection = WorkspaceProjection::load(self)?;
        let artifact = projection
            .artifacts
            .iter()
            .find(|artifact| artifact.id == artifact_id.as_str())
            .cloned()
            .ok_or_else(|| AgentStowError::Manifest {
                message: format!("artifact 不存在：{}", artifact_id.as_str()).into(),
            })?;
        let artifact_targets: Vec<_> = projection
            .targets
            .iter()
            .filter(|target| target.artifact_id == artifact.id)
            .cloned()
            .collect();
        let profile_ids: BTreeSet<_> = artifact_targets
            .iter()
            .filter_map(|target| target.profile.clone())
            .collect();
        let related_profiles: Vec<_> = projection
            .profiles
            .iter()
            .filter(|profile| profile_ids.contains(&profile.id))
            .cloned()
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
            issues: filter_issues(&projection.issues, &subject_ids),
        })
    }

    pub(crate) fn profile_detail(
        &self,
        profile_name: &ProfileName,
    ) -> Result<ProfileDetailResponse> {
        let projection = WorkspaceProjection::load(self)?;
        let profile = projection
            .profiles
            .iter()
            .find(|profile| profile.id == profile_name.as_str())
            .cloned()
            .ok_or_else(|| AgentStowError::Manifest {
                message: format!("profile 不存在：{}", profile_name.as_str()).into(),
            })?;
        let profile_def = projection
            .manifest
            .profiles
            .get(profile_name)
            .ok_or_else(|| AgentStowError::Manifest {
                message: format!("profile 不存在：{}", profile_name.as_str()).into(),
            })?;
        let declared_vars = build_declared_profile_vars(profile_def);
        let merged_vars = build_profile_vars(&projection.manifest, profile_name)?;
        let profile_targets: Vec<_> = projection
            .targets
            .iter()
            .filter(|target| target.profile.as_deref() == Some(profile_name.as_str()))
            .cloned()
            .collect();
        let artifact_ids: BTreeSet<_> = profile_targets
            .iter()
            .map(|target| target.artifact_id.clone())
            .collect();
        let related_artifacts: Vec<_> = projection
            .artifacts
            .iter()
            .filter(|artifact| artifact_ids.contains(&artifact.id))
            .cloned()
            .collect();

        let subject_ids = collect_subject_ids(
            [profile.id.clone()],
            profile_targets.iter().map(|target| target.id.clone()),
            related_artifacts.iter().map(|artifact| artifact.id.clone()),
        );

        Ok(ProfileDetailResponse {
            profile,
            syntax_mode: profile_var_syntax_mode_response(profile_def.var_syntax_mode()),
            declared_vars,
            merged_vars,
            targets: profile_targets,
            artifacts: related_artifacts,
            issues: filter_issues(&projection.issues, &subject_ids),
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

        let projection = WorkspaceProjection::load(self)?;
        ensure_impact_subject_exists(&projection.manifest, artifact, profile)?;
        let affected_targets: Vec<_> = projection
            .targets
            .iter()
            .filter(|target| target_matches_impact(target, artifact, profile))
            .cloned()
            .collect();
        let artifact_ids = collect_artifact_ids(&affected_targets, artifact);
        let profile_ids = collect_profile_ids(&affected_targets, profile);

        let affected_artifacts: Vec<_> = projection
            .artifacts
            .iter()
            .filter(|artifact| artifact_ids.contains(&artifact.id))
            .cloned()
            .collect();
        let affected_profiles: Vec<_> = projection
            .profiles
            .iter()
            .filter(|profile| profile_ids.contains(&profile.id))
            .cloned()
            .collect();
        let affected_link_status: Vec<_> = projection
            .link_status
            .iter()
            .filter(|status| link_status_matches_impact(status, artifact, profile))
            .cloned()
            .collect();
        let (subject_kind, subject_id) = impact_subject(artifact, profile);

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
            issues: filter_issues(&projection.issues, &subject_ids),
        })
    }
}

fn ensure_impact_subject_exists(
    manifest: &Manifest,
    artifact: Option<&ArtifactId>,
    profile: Option<&ProfileName>,
) -> Result<()> {
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

    Ok(())
}

fn target_matches_impact(
    target: &TargetSummaryResponse,
    artifact: Option<&ArtifactId>,
    profile: Option<&ProfileName>,
) -> bool {
    match (artifact, profile) {
        (Some(artifact_id), Some(profile_name)) => {
            target.artifact_id == artifact_id.as_str()
                && target.profile.as_deref() == Some(profile_name.as_str())
        }
        (Some(artifact_id), None) => target.artifact_id == artifact_id.as_str(),
        (None, Some(profile_name)) => target.profile.as_deref() == Some(profile_name.as_str()),
        (None, None) => false,
    }
}

fn link_status_matches_impact(
    status: &LinkStatusResponseItem,
    artifact: Option<&ArtifactId>,
    profile: Option<&ProfileName>,
) -> bool {
    match (artifact, profile) {
        (Some(artifact_id), Some(profile_name)) => {
            status.artifact_id == artifact_id.as_str() && status.profile == profile_name.as_str()
        }
        (Some(artifact_id), None) => status.artifact_id == artifact_id.as_str(),
        (None, Some(profile_name)) => status.profile == profile_name.as_str(),
        (None, None) => false,
    }
}

fn collect_artifact_ids(
    affected_targets: &[TargetSummaryResponse],
    artifact: Option<&ArtifactId>,
) -> BTreeSet<String> {
    let mut artifact_ids = affected_targets
        .iter()
        .map(|target| target.artifact_id.clone())
        .collect::<BTreeSet<_>>();
    if let Some(artifact_id) = artifact {
        artifact_ids.insert(artifact_id.as_str().to_string());
    }
    artifact_ids
}

fn collect_profile_ids(
    affected_targets: &[TargetSummaryResponse],
    profile: Option<&ProfileName>,
) -> BTreeSet<String> {
    let mut profile_ids = affected_targets
        .iter()
        .filter_map(|target| target.profile.clone())
        .collect::<BTreeSet<_>>();
    if let Some(profile_name) = profile {
        profile_ids.insert(profile_name.as_str().to_string());
    }
    profile_ids
}

fn impact_subject(
    artifact: Option<&ArtifactId>,
    profile: Option<&ProfileName>,
) -> (ImpactSubjectKindResponse, String) {
    match (artifact, profile) {
        (Some(artifact_id), Some(profile_name)) => (
            ImpactSubjectKindResponse::ArtifactProfile,
            format!("{}@{}", artifact_id.as_str(), profile_name.as_str()),
        ),
        (Some(artifact_id), None) => (
            ImpactSubjectKindResponse::Artifact,
            artifact_id.as_str().to_string(),
        ),
        (None, Some(profile_name)) => (
            ImpactSubjectKindResponse::Profile,
            profile_name.as_str().to_string(),
        ),
        (None, None) => unreachable!(),
    }
}
