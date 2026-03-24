use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use agentstow_core::{
    AgentStowError, ArtifactId, ArtifactKind, CwdPolicy, InstallMethod, OutputMode, ProfileName,
    Result, SecretBinding, StdinMode, TargetName, ValidateAs, absolutize,
};
use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};

mod mcp;
mod validate;
mod workspace;

pub use mcp::{McpOauthDef, McpServerDef, McpServerOptions, McpTransport};
pub use workspace::{
    WorkspaceInitOutcome, WorkspaceProbe, init_workspace_skeleton, probe_workspace_path,
};

pub const DEFAULT_MANIFEST_FILE: &str = "agentstow.toml";

#[derive(Debug, Clone)]
pub struct Manifest {
    pub workspace_root: PathBuf,
    pub profiles: BTreeMap<ProfileName, Profile>,
    pub env: EnvContextDef,
    pub file: BTreeMap<String, FileContextDef>,
    pub artifacts: BTreeMap<ArtifactId, ArtifactDef>,
    pub targets: BTreeMap<TargetName, TargetDef>,
    pub scripts: BTreeMap<String, ScriptDef>,
    pub mcp_servers: BTreeMap<String, McpServerDef>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ProfileVarSyntaxMode {
    #[default]
    Inline,
    VarsObject,
    Mixed,
}

#[derive(Debug, Clone, Default)]
pub struct Profile {
    pub extends: Vec<ProfileName>,
    pub vars: serde_json::Map<String, serde_json::Value>,
    pub var_syntax: ProfileVarSyntaxMode,
}

#[derive(Debug, Deserialize)]
struct RawProfile {
    #[serde(default)]
    extends: Vec<ProfileName>,
    #[serde(default)]
    vars: toml::Table,
    #[serde(flatten)]
    inline_vars: toml::Table,
}

impl<'de> Deserialize<'de> for Profile {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawProfile::deserialize(deserializer)?;
        let mut vars = json_map_from_toml_table::<D::Error>(raw.vars)?;
        let inline_vars = json_map_from_toml_table::<D::Error>(raw.inline_vars)?;

        if let Some(duplicate) = vars
            .keys()
            .find(|key| inline_vars.contains_key(*key))
            .cloned()
        {
            return Err(de::Error::custom(format!(
                "profile 变量 `{duplicate}` 同时出现在 `vars` 和 profile 顶层，请只保留一种写法"
            )));
        }

        let var_syntax = match (vars.is_empty(), inline_vars.is_empty()) {
            (true, true) | (true, false) => ProfileVarSyntaxMode::Inline,
            (false, true) => ProfileVarSyntaxMode::VarsObject,
            (false, false) => ProfileVarSyntaxMode::Mixed,
        };

        vars.extend(inline_vars);

        Ok(Self {
            extends: raw.extends,
            vars,
            var_syntax,
        })
    }
}

fn json_map_from_toml_table<E>(
    table: toml::Table,
) -> std::result::Result<serde_json::Map<String, serde_json::Value>, E>
where
    E: de::Error,
{
    match serde_json::to_value(table).map_err(E::custom)? {
        serde_json::Value::Object(map) => Ok(map),
        _ => Err(E::custom("profile vars 必须是对象")),
    }
}

impl Profile {
    pub fn merged_vars(
        &self,
        all: &BTreeMap<ProfileName, Profile>,
    ) -> Result<serde_json::Map<String, serde_json::Value>> {
        let mut visited = HashSet::<ProfileName>::new();
        merge_profile_vars(self, all, &mut visited)
    }

    pub fn declared_vars(&self) -> &serde_json::Map<String, serde_json::Value> {
        &self.vars
    }

    pub fn var_syntax_mode(&self) -> ProfileVarSyntaxMode {
        self.var_syntax
    }
}

fn merge_profile_vars(
    profile: &Profile,
    all: &BTreeMap<ProfileName, Profile>,
    visited: &mut HashSet<ProfileName>,
) -> Result<serde_json::Map<String, serde_json::Value>> {
    let mut merged = serde_json::Map::new();

    for parent_name in &profile.extends {
        let parent = all
            .get(parent_name)
            .ok_or_else(|| AgentStowError::Manifest {
                message: format!("profile extends 不存在：{parent_name}").into(),
            })?;
        if visited.contains(parent_name) {
            return Err(AgentStowError::Manifest {
                message: format!("profile extends 存在循环引用：{parent_name}").into(),
            });
        }
        visited.insert(parent_name.clone());
        let parent_vars = merge_profile_vars(parent, all, visited)?;
        for (k, v) in parent_vars {
            merged.insert(k, v);
        }
        visited.remove(parent_name);
    }

    for (k, v) in &profile.vars {
        merged.insert(k.clone(), v.clone());
    }

    Ok(merged)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactDef {
    pub kind: ArtifactKind,
    pub source: PathBuf,
    #[serde(default)]
    pub template: bool,
    #[serde(default)]
    pub validate_as: ValidateAs,
}

impl ArtifactDef {
    pub fn source_path(&self, workspace_root: &Path) -> PathBuf {
        absolutize(workspace_root, &self.source)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetDef {
    pub artifact: ArtifactId,
    #[serde(default)]
    pub profile: Option<ProfileName>,
    pub target_path: PathBuf,
    #[serde(default)]
    pub method: InstallMethod,
}

impl TargetDef {
    pub fn absolute_target_path(&self, workspace_root: &Path) -> PathBuf {
        absolutize(workspace_root, &self.target_path)
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct EnvContextDef {
    pub files: EnvFilesDef,
    pub emit: BTreeMap<String, EnvSet>,
    pub vars: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct RawEnvContextDef {
    #[serde(default)]
    files: EnvFilesDef,
    #[serde(default)]
    emit: BTreeMap<String, EnvSet>,
    #[serde(flatten)]
    vars: toml::Table,
}

impl<'de> Deserialize<'de> for EnvContextDef {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawEnvContextDef::deserialize(deserializer)?;
        let vars = raw
            .vars
            .into_iter()
            .map(|(key, value)| {
                toml_value_to_env_string::<D::Error>(&key, value).map(|value| (key, value))
            })
            .collect::<std::result::Result<BTreeMap<_, _>, _>>()?;

        Ok(Self {
            files: raw.files,
            emit: raw.emit,
            vars,
        })
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnvFilesDef {
    #[serde(default)]
    pub paths: Vec<PathBuf>,
}

impl EnvFilesDef {
    pub fn absolute_paths(&self, workspace_root: &Path) -> Vec<PathBuf> {
        self.paths
            .iter()
            .map(|path| absolutize(workspace_root, path))
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContextDef {
    pub path: PathBuf,
}

impl FileContextDef {
    pub fn absolute_path(&self, workspace_root: &Path) -> PathBuf {
        absolutize(workspace_root, &self.path)
    }
}

fn toml_value_to_env_string<E>(key: &str, value: toml::Value) -> std::result::Result<String, E>
where
    E: de::Error,
{
    match value {
        toml::Value::String(value) => Ok(value),
        toml::Value::Integer(value) => Ok(value.to_string()),
        toml::Value::Float(value) => Ok(value.to_string()),
        toml::Value::Boolean(value) => Ok(value.to_string()),
        toml::Value::Datetime(value) => Ok(value.to_string()),
        toml::Value::Array(_) | toml::Value::Table(_) => Err(E::custom(format!(
            "env.{key} 必须是字符串或可字符串化的标量"
        ))),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVarDef {
    pub key: String,
    pub binding: SecretBinding,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnvSet {
    #[serde(default)]
    pub vars: Vec<EnvVarDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptDef {
    pub kind: String,
    pub entry: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub cwd_policy: CwdPolicy,
    #[serde(default)]
    pub env: Vec<EnvVarDef>,
    #[serde(default)]
    pub stdin_mode: StdinMode,
    #[serde(default)]
    pub stdout_mode: OutputMode,
    #[serde(default)]
    pub stderr_mode: OutputMode,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub expected_exit_codes: Vec<i32>,
}

#[derive(Debug, Deserialize)]
struct ManifestToml {
    #[serde(default)]
    profiles: BTreeMap<ProfileName, Profile>,
    #[serde(default)]
    env: EnvContextDef,
    #[serde(default)]
    file: BTreeMap<String, FileContextDef>,
    #[serde(default)]
    artifacts: BTreeMap<ArtifactId, ArtifactDef>,
    #[serde(default)]
    targets: BTreeMap<TargetName, TargetDef>,
    #[serde(default)]
    scripts: BTreeMap<String, ScriptDef>,
    #[serde(default)]
    mcp_servers: toml::Table,
}

impl Manifest {
    pub fn load_from_dir(workspace_root: &Path) -> Result<Self> {
        let path = workspace_root.join(DEFAULT_MANIFEST_FILE);
        Self::load_from_path(&path)
    }

    pub fn load_from_path(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(AgentStowError::from)?;
        let parsed: ManifestToml =
            toml::from_str(&content).map_err(|e| AgentStowError::Manifest {
                message: format!("解析 manifest 失败：{e}").into(),
            })?;

        let workspace_root = path.parent().ok_or_else(|| AgentStowError::Manifest {
            message: "manifest path 没有 parent".into(),
        })?;

        validate::validate_manifest(&parsed, workspace_root)?;
        let mcp_servers = mcp::resolve_mcp_servers(&parsed.mcp_servers, workspace_root)?;

        Ok(Self {
            workspace_root: workspace_root.to_path_buf(),
            profiles: parsed.profiles,
            env: parsed.env,
            file: parsed.file,
            artifacts: parsed.artifacts,
            targets: parsed.targets,
            scripts: parsed.scripts,
            mcp_servers,
        })
    }

    pub fn find_from(start_dir: &Path) -> Result<PathBuf> {
        let mut cur = start_dir;
        loop {
            let candidate = cur.join(DEFAULT_MANIFEST_FILE);
            if candidate.is_file() {
                return Ok(candidate);
            }
            match cur.parent() {
                Some(parent) => cur = parent,
                None => {
                    return Err(AgentStowError::Manifest {
                        message: format!(
                            "未找到 {DEFAULT_MANIFEST_FILE}（从 {} 向上搜索）。你可以运行 `agentstow workspace init` 初始化，或使用 `--workspace` 指定已有 workspace，或启动 `agentstow serve` 通过 Web 引导。",
                            agentstow_core::normalize_for_display(start_dir)
                        )
                        .into(),
                    });
                }
            }
        }
    }

    pub fn profile_vars(
        &self,
        name: &ProfileName,
    ) -> Result<serde_json::Map<String, serde_json::Value>> {
        let profile = self
            .profiles
            .get(name)
            .ok_or_else(|| AgentStowError::Manifest {
                message: format!("profile 不存在：{name}").into(),
            })?;
        profile.merged_vars(&self.profiles)
    }
}

#[cfg(test)]
mod tests;
