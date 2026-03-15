use std::io::Write as _;

use agentstow_core::{AgentStowError, ArtifactId, ArtifactKind, Result, normalize_for_display};
use agentstow_manifest::{ArtifactDef, Manifest};
use agentstow_render::Renderer;
use agentstow_validate::Validator;
use agentstow_web_types::{ArtifactSourceResponse, ManifestSourceResponse, RenderResponse};

use super::WorkspaceQueryService;
use super::common::{
    artifact_kind_response, ensure_safe_workspace_relative_path, validate_as_response,
};

impl WorkspaceQueryService {
    pub(crate) fn render_preview(
        &self,
        artifact_id: &ArtifactId,
        profile: &agentstow_core::ProfileName,
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

    pub(crate) fn artifact_source(
        &self,
        artifact_id: &ArtifactId,
    ) -> Result<ArtifactSourceResponse> {
        let manifest = self.load_manifest()?;
        let artifact_def = file_artifact_definition(&manifest, artifact_id)?;
        ensure_safe_workspace_relative_path(&artifact_def.source)?;

        let source_path = artifact_def.source_path(&manifest.workspace_root);
        let content = fs_err::read_to_string(&source_path).map_err(AgentStowError::from)?;

        Ok(artifact_source_response(
            &manifest,
            artifact_id,
            artifact_def,
            content,
        ))
    }

    pub(crate) fn update_artifact_source(
        &self,
        artifact_id: &ArtifactId,
        content: &str,
    ) -> Result<ArtifactSourceResponse> {
        let manifest = self.load_manifest()?;
        let artifact_def = file_artifact_definition(&manifest, artifact_id)?;
        ensure_safe_workspace_relative_path(&artifact_def.source)?;

        let source_path = artifact_def.source_path(&manifest.workspace_root);
        agentstow_core::ensure_parent_dir(&source_path)?;
        fs_err::write(&source_path, content).map_err(AgentStowError::from)?;

        Ok(artifact_source_response(
            &manifest,
            artifact_id,
            artifact_def,
            content.to_string(),
        ))
    }

    pub(crate) fn manifest_source(&self) -> Result<ManifestSourceResponse> {
        let manifest_path = self
            .workspace_root
            .join(agentstow_manifest::DEFAULT_MANIFEST_FILE);
        let content = fs_err::read_to_string(&manifest_path).map_err(AgentStowError::from)?;

        Ok(ManifestSourceResponse {
            source_path: normalize_for_display(&manifest_path),
            content,
        })
    }

    pub(crate) fn update_manifest_source(&self, content: &str) -> Result<ManifestSourceResponse> {
        let manifest_path = self
            .workspace_root
            .join(agentstow_manifest::DEFAULT_MANIFEST_FILE);
        let mut temp =
            tempfile::NamedTempFile::new_in(&self.workspace_root).map_err(AgentStowError::from)?;
        temp.write_all(content.as_bytes())
            .map_err(AgentStowError::from)?;
        temp.flush().map_err(AgentStowError::from)?;

        // Validate before replacing the real manifest to keep the workspace recoverable.
        Manifest::load_from_path(temp.path())?;
        temp.persist(&manifest_path)
            .map_err(|error| AgentStowError::Other(anyhow::anyhow!(error.error)))?;

        Ok(ManifestSourceResponse {
            source_path: normalize_for_display(&manifest_path),
            content: content.to_string(),
        })
    }
}

fn file_artifact_definition<'a>(
    manifest: &'a Manifest,
    artifact_id: &ArtifactId,
) -> Result<&'a ArtifactDef> {
    let artifact_def =
        manifest
            .artifacts
            .get(artifact_id)
            .ok_or_else(|| AgentStowError::Manifest {
                message: format!("artifact 不存在：{}", artifact_id.as_str()).into(),
            })?;

    if artifact_def.kind != ArtifactKind::File {
        return Err(AgentStowError::InvalidArgs {
            message: "仅支持 file artifact 的 source 操作".into(),
        });
    }

    Ok(artifact_def)
}

fn artifact_source_response(
    manifest: &Manifest,
    artifact_id: &ArtifactId,
    artifact_def: &ArtifactDef,
    content: String,
) -> ArtifactSourceResponse {
    ArtifactSourceResponse {
        artifact_id: artifact_id.as_str().to_string(),
        kind: artifact_kind_response(artifact_def.kind),
        source_path: normalize_for_display(&artifact_def.source_path(&manifest.workspace_root)),
        template: artifact_def.template,
        validate_as: validate_as_response(artifact_def.validate_as),
        content,
    }
}
