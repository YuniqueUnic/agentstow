use std::path::{Path, PathBuf};

use agentstow_core::{
    AgentStowError, ArtifactId, ArtifactKind, ProfileName, Result, normalize_for_display,
};
use agentstow_manifest::Manifest;
use agentstow_mcp::Mcp;
use tera::Context;
use tracing::instrument;

#[derive(Debug, Clone)]
pub struct RenderedFile {
    pub artifact_id: ArtifactId,
    pub profile: ProfileName,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct RenderedDir {
    pub artifact_id: ArtifactId,
    pub profile: ProfileName,
    pub entries: Vec<RenderedDirEntry>,
}

#[derive(Debug, Clone)]
pub struct RenderedDirEntry {
    pub relative_path: PathBuf,
    pub kind: RenderedDirEntryKind,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderedDirEntryKind {
    Dir,
    File,
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
                    message: format!("artifact 不存在：{artifact_id}").into(),
                })?;

        if artifact.kind != ArtifactKind::File {
            return Err(AgentStowError::Render {
                message: format!("当前仅支持渲染 file artifact（收到 {:?}）", artifact.kind).into(),
            });
        }

        let ctx = build_tera_context(manifest, profile)?;
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

    #[instrument(skip_all, fields(artifact_id=%artifact_id, profile=%profile))]
    pub fn render_dir(
        manifest: &Manifest,
        artifact_id: &ArtifactId,
        profile: &ProfileName,
    ) -> Result<RenderedDir> {
        let artifact =
            manifest
                .artifacts
                .get(artifact_id)
                .ok_or_else(|| AgentStowError::Manifest {
                    message: format!("artifact 不存在：{artifact_id}").into(),
                })?;

        if artifact.kind != ArtifactKind::Dir {
            return Err(AgentStowError::Render {
                message: format!("当前仅支持渲染 dir artifact（收到 {:?}）", artifact.kind).into(),
            });
        }

        let ctx = build_tera_context(manifest, profile)?;
        let source_root = artifact.source_path(&manifest.workspace_root);
        if !source_root.is_dir() {
            return Err(AgentStowError::Render {
                message: format!(
                    "目录 artifact source 不是目录：{}",
                    normalize_for_display(&source_root)
                )
                .into(),
            });
        }

        let mut entries = Vec::new();
        collect_rendered_dir_entries(
            &source_root,
            &source_root,
            artifact.template,
            &ctx,
            &mut entries,
        )?;

        Ok(RenderedDir {
            artifact_id: artifact_id.clone(),
            profile: profile.clone(),
            entries,
        })
    }
}

fn build_tera_context(manifest: &Manifest, profile: &ProfileName) -> Result<Context> {
    let mut vars = manifest.profile_vars(profile)?;
    vars.insert("env_files".to_string(), load_env_file_contexts(manifest)?);
    vars.insert("files".to_string(), load_file_contexts(manifest)?);
    vars.insert("mcp_servers".to_string(), load_mcp_contexts(manifest)?);

    Context::from_serialize(&vars).map_err(|e| AgentStowError::Render {
        message: format!("构建 Tera context 失败：{e}").into(),
    })
}

fn load_env_file_contexts(manifest: &Manifest) -> Result<serde_json::Value> {
    let mut out = serde_json::Map::new();
    for (alias, def) in &manifest.render_context.env_files {
        let path = def.absolute_path(&manifest.workspace_root);
        let mut values = serde_json::Map::new();
        let iter = dotenvy::from_path_iter(&path).map_err(|e| AgentStowError::Render {
            message: format!(
                "读取 env file 失败：path={}, {e}",
                normalize_for_display(&path)
            )
            .into(),
        })?;
        for item in iter {
            let (key, value) = item.map_err(|e| AgentStowError::Render {
                message: format!(
                    "解析 env file 失败：path={}, {e}",
                    normalize_for_display(&path)
                )
                .into(),
            })?;
            values.insert(key, serde_json::Value::String(value));
        }
        out.insert(alias.clone(), serde_json::Value::Object(values));
    }
    Ok(serde_json::Value::Object(out))
}

fn load_file_contexts(manifest: &Manifest) -> Result<serde_json::Value> {
    let mut out = serde_json::Map::new();
    for (alias, def) in &manifest.render_context.files {
        let path = def.absolute_path(&manifest.workspace_root);
        let content = fs_err::read_to_string(&path).map_err(|e| AgentStowError::Render {
            message: format!(
                "读取 file context 失败：path={}, {e}",
                normalize_for_display(&path)
            )
            .into(),
        })?;
        out.insert(alias.clone(), serde_json::Value::String(content));
    }
    Ok(serde_json::Value::Object(out))
}

fn load_mcp_contexts(manifest: &Manifest) -> Result<serde_json::Value> {
    let mut out = serde_json::Map::new();
    for (alias, def) in &manifest.render_context.mcp_servers {
        let server =
            manifest
                .mcp_servers
                .get(&def.server)
                .ok_or_else(|| AgentStowError::Manifest {
                    message: format!(
                        "render_context.mcp_servers 引用不存在的 mcp server: {}",
                        def.server
                    )
                    .into(),
                })?;
        let rendered = Mcp::render_server_json(&def.server, server)?;
        let value = serde_json::from_str(&rendered).map_err(|e| AgentStowError::Render {
            message: format!("解析 mcp render output 失败：server={}, {e}", def.server).into(),
        })?;
        out.insert(alias.clone(), value);
    }
    Ok(serde_json::Value::Object(out))
}

fn collect_rendered_dir_entries(
    source_root: &Path,
    current_dir: &Path,
    template_enabled: bool,
    ctx: &Context,
    entries: &mut Vec<RenderedDirEntry>,
) -> Result<()> {
    let mut dir_entries = fs_err::read_dir(current_dir)
        .map_err(AgentStowError::from)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(AgentStowError::from)?;
    dir_entries.sort_by_key(|entry| entry.file_name());

    for entry in dir_entries {
        let path = entry.path();
        let relative_path = path
            .strip_prefix(source_root)
            .map_err(|e| AgentStowError::Render {
                message: format!(
                    "计算目录相对路径失败：root={}, path={}, {e}",
                    normalize_for_display(source_root),
                    normalize_for_display(&path)
                )
                .into(),
            })?
            .to_path_buf();
        let file_type = entry.file_type().map_err(AgentStowError::from)?;
        if file_type.is_dir() {
            entries.push(RenderedDirEntry {
                relative_path: relative_path.clone(),
                kind: RenderedDirEntryKind::Dir,
                bytes: Vec::new(),
            });
            collect_rendered_dir_entries(source_root, &path, template_enabled, ctx, entries)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }

        let (render_path, bytes) = if template_enabled && is_tera_template_file(&path) {
            (
                strip_tera_suffix(relative_path),
                render_tera_template_file(&path, ctx)?,
            )
        } else {
            (
                relative_path,
                fs_err::read(&path).map_err(AgentStowError::from)?,
            )
        };

        entries.push(RenderedDirEntry {
            relative_path: render_path,
            kind: RenderedDirEntryKind::File,
            bytes,
        });
    }

    Ok(())
}

fn is_tera_template_file(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == "tera")
}

fn strip_tera_suffix(path: PathBuf) -> PathBuf {
    match path.file_name().and_then(|name| name.to_str()) {
        Some(name) if name.ends_with(".tera") => {
            let stripped = name.trim_end_matches(".tera");
            match path.parent() {
                Some(parent) => parent.join(stripped),
                None => PathBuf::from(stripped),
            }
        }
        _ => path,
    }
}

fn render_tera_template_file(path: &Path, ctx: &Context) -> Result<Vec<u8>> {
    let template = fs_err::read_to_string(path).map_err(AgentStowError::from)?;
    let rendered =
        tera::Tera::one_off(&template, ctx, false).map_err(|e| AgentStowError::Render {
            message: format!(
                "Tera render 失败：path={}, {e}; detail={e:?}",
                normalize_for_display(path)
            )
            .into(),
        })?;
    Ok(rendered.into_bytes())
}

#[cfg(test)]
mod tests;
