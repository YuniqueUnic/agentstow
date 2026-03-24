use time::OffsetDateTime;

use agentstow_core::{AgentStowError, ArtifactId, ArtifactKind, ProfileName, Result};
use agentstow_manifest::Manifest;
use agentstow_render::Renderer;
use agentstow_state::LinkInstanceRecord;
use agentstow_validate::Validator;

use crate::types::{InstallSource, LinkJob, RenderStore};

pub fn build_link_job_from_manifest(
    manifest: &Manifest,
    target_name: &agentstow_core::TargetName,
    target: &agentstow_manifest::TargetDef,
    profile: &ProfileName,
    render_store: &RenderStore,
) -> Result<LinkJob> {
    let artifact =
        manifest
            .artifacts
            .get(&target.artifact)
            .ok_or_else(|| AgentStowError::Manifest {
                message: format!("artifact 不存在：{}", target.artifact.as_str()).into(),
            })?;
    let target_path = target.absolute_target_path(&manifest.workspace_root);
    let desired = build_install_source(manifest, &target.artifact, profile, render_store)?;

    Ok(LinkJob {
        target: target_name.clone(),
        artifact_id: target.artifact.clone(),
        profile: profile.clone(),
        artifact_kind: artifact.kind,
        method: target.method,
        target_path,
        desired,
    })
}

pub fn build_link_instance_record(
    manifest: &Manifest,
    job: &LinkJob,
    store: &RenderStore,
    updated_at: OffsetDateTime,
) -> Result<LinkInstanceRecord> {
    let artifact =
        manifest
            .artifacts
            .get(&job.artifact_id)
            .ok_or_else(|| AgentStowError::Manifest {
                message: format!("artifact 不存在：{}", job.artifact_id.as_str()).into(),
            })?;
    let (rendered_path, blake3) = match (&job.method, &job.desired) {
        (agentstow_core::InstallMethod::Symlink, InstallSource::FileBytes(bytes)) => (
            Some(store.rendered_file_path(&job.artifact_id, &job.profile)),
            Some(blake3::hash(bytes).to_hex().to_string()),
        ),
        (agentstow_core::InstallMethod::Copy, InstallSource::FileBytes(bytes)) => {
            (None, Some(blake3::hash(bytes).to_hex().to_string()))
        }
        (agentstow_core::InstallMethod::Symlink, InstallSource::Path(path))
        | (agentstow_core::InstallMethod::Junction, InstallSource::Path(path)) => {
            (Some(path.clone()), None)
        }
        (agentstow_core::InstallMethod::Copy, InstallSource::Path(path)) if artifact.template => {
            (Some(path.clone()), None)
        }
        (agentstow_core::InstallMethod::Copy, InstallSource::Path(_)) => (None, None),
        _ => (None, None),
    };

    Ok(LinkInstanceRecord {
        workspace_root: manifest.workspace_root.clone(),
        artifact_id: job.artifact_id.clone(),
        profile: job.profile.clone(),
        target_path: job.target_path.clone(),
        method: job.method,
        rendered_path,
        blake3,
        updated_at,
    })
}

fn build_install_source(
    manifest: &Manifest,
    artifact_id: &ArtifactId,
    profile: &ProfileName,
    render_store: &RenderStore,
) -> Result<InstallSource> {
    let artifact = manifest
        .artifacts
        .get(artifact_id)
        .ok_or_else(|| AgentStowError::Manifest {
            message: format!("artifact 不存在：{}", artifact_id.as_str()).into(),
        })?;

    match artifact.kind {
        ArtifactKind::File => {
            let rendered = Renderer::render_file(manifest, artifact_id, profile)?;
            Validator::validate_rendered_file(artifact, &rendered.bytes)?;
            Ok(InstallSource::FileBytes(rendered.bytes))
        }
        ArtifactKind::Dir => {
            if artifact.template {
                let rendered = Renderer::render_dir(manifest, artifact_id, profile)?;
                let rendered_path =
                    render_store.write_rendered_dir(artifact_id, profile, &rendered)?;
                Ok(InstallSource::Path(rendered_path))
            } else {
                Ok(InstallSource::Path(
                    artifact.source_path(&manifest.workspace_root),
                ))
            }
        }
    }
}
