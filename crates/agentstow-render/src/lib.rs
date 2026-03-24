use std::collections::HashMap;
use std::path::{Path, PathBuf};

use agentstow_core::{
    AgentStowError, ArtifactId, ArtifactKind, ProfileName, Result, ValidateAs,
    normalize_for_display,
};
use agentstow_manifest::Manifest;
use agentstow_mcp::{Mcp, McpSnippetFormat, McpTargetAdapter};
use tera::{Context, Filter, Tera, Value};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum McpContextFormat {
    Json,
    Toml,
    Yaml,
}

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

        let source_path = artifact.source_path(&manifest.workspace_root);
        let bytes = if artifact.template {
            let ctx = build_tera_context(
                manifest,
                profile,
                infer_mcp_context_format(&source_path, artifact.validate_as),
            )?;
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
            manifest,
            profile,
            &source_root,
            &source_root,
            artifact.template,
            artifact.validate_as,
            &mut entries,
        )?;

        Ok(RenderedDir {
            artifact_id: artifact_id.clone(),
            profile: profile.clone(),
            entries,
        })
    }
}

fn build_tera_context(
    manifest: &Manifest,
    profile: &ProfileName,
    mcp_format: McpContextFormat,
) -> Result<Context> {
    let mut vars = manifest.profile_vars(profile)?;
    vars.insert("env".to_string(), load_env_context(manifest)?);
    vars.insert("file".to_string(), load_file_contexts(manifest)?);
    vars.insert(
        "mcp_servers".to_string(),
        load_mcp_contexts(manifest, mcp_format)?,
    );

    Context::from_serialize(&vars).map_err(|e| AgentStowError::Render {
        message: format!("构建 Tera context 失败：{e}").into(),
    })
}

fn load_env_context(manifest: &Manifest) -> Result<serde_json::Value> {
    let mut out = serde_json::Map::new();
    for path in manifest.env.files.absolute_paths(&manifest.workspace_root) {
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
            out.insert(key, serde_json::Value::String(value));
        }
    }

    for (key, value) in &manifest.env.vars {
        out.insert(key.clone(), serde_json::Value::String(value.clone()));
    }

    Ok(serde_json::Value::Object(out))
}

fn load_file_contexts(manifest: &Manifest) -> Result<serde_json::Value> {
    let mut out = serde_json::Map::new();
    for (alias, def) in &manifest.file {
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

fn load_mcp_contexts(manifest: &Manifest, format: McpContextFormat) -> Result<serde_json::Value> {
    let mut out = serde_json::Map::new();
    for (name, server) in &manifest.mcp_servers {
        let rendered = Mcp::render_generic_server_snippet(name, server, format.into())?;
        out.insert(name.clone(), serde_json::Value::String(rendered));
    }
    Ok(serde_json::Value::Object(out))
}

fn collect_rendered_dir_entries(
    manifest: &Manifest,
    profile: &ProfileName,
    source_root: &Path,
    current_dir: &Path,
    template_enabled: bool,
    validate_as: ValidateAs,
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
            collect_rendered_dir_entries(
                manifest,
                profile,
                source_root,
                &path,
                template_enabled,
                validate_as,
                entries,
            )?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }

        let (render_path, bytes) = if template_enabled && is_tera_template_file(&path) {
            let render_path = strip_tera_suffix(relative_path);
            let ctx = build_tera_context(
                manifest,
                profile,
                infer_mcp_context_format(&render_path, validate_as),
            )?;
            (render_path, render_tera_template_file(&path, &ctx)?)
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
    let mut tera = Tera::default();
    tera.autoescape_on(Vec::<&'static str>::new());
    tera.register_filter("toml", McpSnippetFilter::new(McpSnippetFormat::Toml));
    tera.register_filter("json", McpSnippetFilter::new(McpSnippetFormat::Json));
    tera.register_filter("yaml", McpSnippetFilter::new(McpSnippetFormat::Yaml));
    tera.register_filter("codex", McpAdapterFilter::new(McpTargetAdapter::Codex));
    tera.add_raw_template("inline", &template)
        .map_err(|e| AgentStowError::Render {
            message: format!(
                "加载 Tera 模板失败：path={}, {e}; detail={e:?}",
                normalize_for_display(path)
            )
            .into(),
        })?;
    let rendered = tera
        .render("inline", ctx)
        .map_err(|e| AgentStowError::Render {
            message: format!(
                "Tera render 失败：path={}, {e}; detail={e:?}",
                normalize_for_display(path)
            )
            .into(),
        })?;
    Ok(rendered.into_bytes())
}

fn infer_mcp_context_format(path: &Path, validate_as: ValidateAs) -> McpContextFormat {
    match validate_as {
        ValidateAs::Toml => McpContextFormat::Toml,
        ValidateAs::Json => McpContextFormat::Json,
        ValidateAs::None | ValidateAs::Markdown | ValidateAs::Shell => match path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())
        {
            Some(ext) if ext == "toml" => McpContextFormat::Toml,
            Some(ext) if ext == "yaml" || ext == "yml" => McpContextFormat::Yaml,
            _ => McpContextFormat::Json,
        },
    }
}

impl From<McpContextFormat> for McpSnippetFormat {
    fn from(value: McpContextFormat) -> Self {
        match value {
            McpContextFormat::Json => McpSnippetFormat::Json,
            McpContextFormat::Toml => McpSnippetFormat::Toml,
            McpContextFormat::Yaml => McpSnippetFormat::Yaml,
        }
    }
}

struct McpSnippetFilter {
    format: McpSnippetFormat,
}

impl McpSnippetFilter {
    fn new(format: McpSnippetFormat) -> Self {
        Self { format }
    }
}

impl Filter for McpSnippetFilter {
    fn filter(&self, value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
        let input = value
            .as_str()
            .ok_or_else(|| tera::Error::msg("mcp 片段过滤器只接受字符串输入"))?;
        let rendered = Mcp::convert_server_snippet(input, self.format)
            .map_err(|e| tera::Error::msg(e.to_string()))?;
        Ok(Value::String(rendered))
    }
}

struct McpAdapterFilter {
    adapter: McpTargetAdapter,
}

impl McpAdapterFilter {
    fn new(adapter: McpTargetAdapter) -> Self {
        Self { adapter }
    }
}

impl Filter for McpAdapterFilter {
    fn filter(&self, value: &Value, args: &HashMap<String, Value>) -> tera::Result<Value> {
        let input = value
            .as_str()
            .ok_or_else(|| tera::Error::msg("mcp adapter 过滤器只接受字符串输入"))?;
        let format = args.get("format").map(parse_snippet_format).transpose()?;
        let rendered = Mcp::adapt_server_snippet(input, self.adapter, format)
            .map_err(|e| tera::Error::msg(e.to_string()))?;
        Ok(Value::String(rendered))
    }
}

fn parse_snippet_format(value: &Value) -> tera::Result<McpSnippetFormat> {
    let value = value
        .as_str()
        .ok_or_else(|| tera::Error::msg("format 参数必须是字符串"))?;
    match value {
        "json" => Ok(McpSnippetFormat::Json),
        "toml" => Ok(McpSnippetFormat::Toml),
        "yaml" | "yml" => Ok(McpSnippetFormat::Yaml),
        other => Err(tera::Error::msg(format!(
            "不支持的 mcp format: {other}；可选 json/toml/yaml"
        ))),
    }
}

#[cfg(test)]
mod tests;
