use std::path::Path;

use agentstow_core::{AgentStowError, ArtifactId, ArtifactKind, ProfileName, Result};
use agentstow_manifest::Manifest;
use tera::Context;
use tracing::instrument;

#[derive(Debug, Clone)]
pub struct RenderedFile {
    pub artifact_id: ArtifactId,
    pub profile: ProfileName,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Renderer;

impl Renderer {
    #[instrument(skip_all, fields(artifact_id=%artifact_id, profile=%profile))]
    pub fn render_file(
        manifest: &Manifest,
        artifact_id: &ArtifactId,
        profile: &ProfileName,
    ) -> Result<RenderedFile> {
        let artifact =
            manifest
                .artifacts
                .get(artifact_id)
                .ok_or_else(|| AgentStowError::Manifest {
                    message: format!("artifact 不存在: {artifact_id}").into(),
                })?;

        if artifact.kind != ArtifactKind::File {
            return Err(AgentStowError::Render {
                message: format!("当前仅支持渲染 file artifact（收到 {:?}）", artifact.kind).into(),
            });
        }

        let vars = manifest.profile_vars(profile)?;
        let ctx = Context::from_serialize(&vars).map_err(|e| AgentStowError::Render {
            message: format!("构建 Tera context 失败: {e}").into(),
        })?;

        let source_path = artifact.source_path(&manifest.workspace_root);
        let bytes = if artifact.template {
            render_tera_template_file(&source_path, &ctx)?
        } else {
            fs_err::read(&source_path).map_err(AgentStowError::from)?
        };

        Ok(RenderedFile {
            artifact_id: artifact_id.clone(),
            profile: profile.clone(),
            bytes,
        })
    }
}

fn render_tera_template_file(path: &Path, ctx: &Context) -> Result<Vec<u8>> {
    let template = fs_err::read_to_string(path).map_err(AgentStowError::from)?;
    let rendered =
        tera::Tera::one_off(&template, ctx, false).map_err(|e| AgentStowError::Render {
            message: format!("Tera render 失败: {e}").into(),
        })?;
    Ok(rendered.into_bytes())
}

#[cfg(test)]
mod tests;
