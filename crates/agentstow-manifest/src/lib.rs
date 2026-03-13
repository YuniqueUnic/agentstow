use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use agentstow_core::{
    AgentStowError, ArtifactId, ArtifactKind, CwdPolicy, InstallMethod, OutputMode, ProfileName,
    Result, SecretBinding, StdinMode, TargetName, ValidateAs, absolutize,
};
use serde::{Deserialize, Serialize};

pub const DEFAULT_MANIFEST_FILE: &str = "agentstow.toml";

#[derive(Debug, Clone)]
pub struct Manifest {
    pub workspace_root: PathBuf,
    pub profiles: BTreeMap<ProfileName, Profile>,
    pub artifacts: BTreeMap<ArtifactId, ArtifactDef>,
    pub targets: BTreeMap<TargetName, TargetDef>,
    pub env_sets: BTreeMap<String, EnvSet>,
    pub scripts: BTreeMap<String, ScriptDef>,
    pub mcp_servers: BTreeMap<String, McpServerDef>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Profile {
    #[serde(default)]
    pub extends: Vec<ProfileName>,
    #[serde(default)]
    pub vars: serde_json::Map<String, serde_json::Value>,
}

impl Profile {
    pub fn merged_vars(
        &self,
        all: &BTreeMap<ProfileName, Profile>,
    ) -> Result<serde_json::Map<String, serde_json::Value>> {
        let mut visited = HashSet::<ProfileName>::new();
        merge_profile_vars(self, all, &mut visited)
    }
}

fn merge_profile_vars(
    profile: &Profile,
    all: &BTreeMap<ProfileName, Profile>,
    visited: &mut HashSet<ProfileName>,
) -> Result<serde_json::Map<String, serde_json::Value>> {
    // cycle detection is handled by caller that inserts before recursion
    let mut merged = serde_json::Map::new();

    for parent_name in &profile.extends {
        let parent = all
            .get(parent_name)
            .ok_or_else(|| AgentStowError::Manifest {
                message: format!("profile extends 不存在: {parent_name}").into(),
            })?;
        if visited.contains(parent_name) {
            return Err(AgentStowError::Manifest {
                message: format!("profile extends 存在循环引用: {parent_name}").into(),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerDef {
    pub transport: McpTransport,
    #[serde(default)]
    pub env: Vec<EnvVarDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum McpTransport {
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
    },
    Http {
        url: String,
        #[serde(default)]
        headers: HashMap<String, String>,
    },
}

#[derive(Debug, Deserialize)]
struct ManifestToml {
    #[serde(default)]
    profiles: BTreeMap<ProfileName, Profile>,
    #[serde(default)]
    artifacts: BTreeMap<ArtifactId, ArtifactDef>,
    #[serde(default)]
    targets: BTreeMap<TargetName, TargetDef>,
    #[serde(default)]
    env_sets: BTreeMap<String, EnvSet>,
    #[serde(default)]
    scripts: BTreeMap<String, ScriptDef>,
    #[serde(default)]
    mcp_servers: BTreeMap<String, McpServerDef>,
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
                message: format!("解析 manifest 失败: {e}").into(),
            })?;

        let workspace_root = path.parent().ok_or_else(|| AgentStowError::Manifest {
            message: "manifest path 没有 parent".into(),
        })?;

        validate_manifest(&parsed)?;

        Ok(Self {
            workspace_root: workspace_root.to_path_buf(),
            profiles: parsed.profiles,
            artifacts: parsed.artifacts,
            targets: parsed.targets,
            env_sets: parsed.env_sets,
            scripts: parsed.scripts,
            mcp_servers: parsed.mcp_servers,
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
                            "未找到 {DEFAULT_MANIFEST_FILE}（从 {} 向上搜索）",
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
                message: format!("profile 不存在: {name}").into(),
            })?;
        profile.merged_vars(&self.profiles)
    }
}

fn validate_manifest(m: &ManifestToml) -> Result<()> {
    for profile in m.profiles.values() {
        profile.merged_vars(&m.profiles)?;
    }

    for (target_name, target) in &m.targets {
        if !m.artifacts.contains_key(&target.artifact) {
            return Err(AgentStowError::Manifest {
                message: format!(
                    "target 引用不存在的 artifact: {target_name} -> {}",
                    target.artifact
                )
                .into(),
            });
        }
        if let Some(profile) = &target.profile
            && !m.profiles.contains_key(profile)
        {
            return Err(AgentStowError::Manifest {
                message: format!("target 引用不存在的 profile: {target_name} -> {profile}").into(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
