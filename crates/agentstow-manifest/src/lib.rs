use std::collections::{BTreeMap, HashMap, HashSet};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use agentstow_core::{
    AgentStowError, ArtifactId, ArtifactKind, CwdPolicy, InstallMethod, OutputMode, ProfileName,
    Result, SecretBinding, StdinMode, TargetName, ValidateAs, absolutize, normalize_for_display,
};
use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};

pub const DEFAULT_MANIFEST_FILE: &str = "agentstow.toml";

#[derive(Debug, Clone)]
pub struct WorkspaceInitOutcome {
    pub workspace_root: PathBuf,
    pub manifest_path: PathBuf,
    pub created: bool,
}

#[derive(Debug, Clone)]
pub struct WorkspaceProbe {
    pub resolved_workspace_root: PathBuf,
    pub manifest_path: PathBuf,
    pub exists: bool,
    pub is_directory: bool,
    pub manifest_present: bool,
    pub git_present: bool,
    pub selectable: bool,
    pub initializable: bool,
    pub reason: Option<String>,
}

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
    // cycle detection is handled by caller that inserts before recursion
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerDef {
    pub transport: McpTransport,
    #[serde(default)]
    pub env: Vec<EnvVarDef>,
    #[serde(default, skip_serializing_if = "McpServerOptions::is_empty")]
    pub options: McpServerOptions,
}

impl McpServerDef {
    pub fn env_binding_defs(&self) -> Vec<EnvVarDef> {
        self.env.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct McpServerOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub startup_timeout_sec: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_timeout_sec: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enabled_tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disabled_tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trust: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub include_tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude_tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oauth: Option<McpOauthDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_provider_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_audience: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_service_account: Option<String>,
}

impl McpServerOptions {
    pub fn is_empty(&self) -> bool {
        self.startup_timeout_sec.is_none()
            && self.tool_timeout_sec.is_none()
            && self.enabled.is_none()
            && self.required.is_none()
            && self.enabled_tools.is_empty()
            && self.disabled_tools.is_empty()
            && self.timeout.is_none()
            && self.trust.is_none()
            && self.description.is_none()
            && self.include_tools.is_empty()
            && self.exclude_tools.is_empty()
            && self
                .oauth
                .as_ref()
                .map(McpOauthDef::is_empty)
                .unwrap_or(true)
            && self.auth_provider_type.is_none()
            && self.target_audience.is_none()
            && self.target_service_account.is_none()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct McpOauthDef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub callback_port: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_server_metadata_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authorization_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_url: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scopes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub redirect_uri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_param_name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub audiences: Vec<String>,
}

impl McpOauthDef {
    pub fn is_empty(&self) -> bool {
        self.client_id.is_none()
            && self.callback_port.is_none()
            && self.auth_server_metadata_url.is_none()
            && self.enabled.is_none()
            && self.client_secret.is_none()
            && self.authorization_url.is_none()
            && self.token_url.is_none()
            && self.scopes.is_empty()
            && self.redirect_uri.is_none()
            && self.token_param_name.is_none()
            && self.audiences.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum McpTransport {
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        cwd: Option<PathBuf>,
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

#[derive(Debug, Deserialize)]
struct McpServerImportDef {
    path: PathBuf,
}

#[derive(Debug, Deserialize)]
struct ImportedGenericMcpJsonFile {
    #[serde(rename = "mcpServers", default)]
    mcp_servers: BTreeMap<String, McpServerDef>,
}

#[derive(Debug, Deserialize)]
struct ImportedMcpJsonFile {
    #[serde(rename = "mcpServers", default)]
    mcp_servers: BTreeMap<String, ImportedMcpJsonServer>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ImportedMcpJsonServer {
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env_vars: Vec<String>,
        #[serde(default)]
        env: BTreeMap<String, String>,
        #[serde(default)]
        cwd: Option<PathBuf>,
        #[serde(flatten)]
        options: ImportedMcpServerOptions,
    },
    Http {
        url: String,
        #[serde(default)]
        bearer_token_env_var: Option<String>,
        #[serde(default)]
        http_headers: HashMap<String, String>,
        #[serde(default)]
        env_http_headers: HashMap<String, String>,
        #[serde(flatten)]
        options: ImportedMcpServerOptions,
    },
}

#[derive(Debug, Deserialize, Default)]
struct ImportedMcpServerOptions {
    #[serde(default)]
    startup_timeout_sec: Option<u64>,
    #[serde(default)]
    tool_timeout_sec: Option<u64>,
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    required: Option<bool>,
    #[serde(default)]
    enabled_tools: Vec<String>,
    #[serde(default)]
    disabled_tools: Vec<String>,
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

        validate_manifest(&parsed, workspace_root)?;
        let mcp_servers = resolve_mcp_servers(&parsed.mcp_servers, workspace_root)?;

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

fn resolve_mcp_servers(
    raw: &toml::Table,
    workspace_root: &Path,
) -> Result<BTreeMap<String, McpServerDef>> {
    let mut servers = BTreeMap::new();

    for (name, value) in raw {
        if name == "file" {
            let import = value
                .clone()
                .try_into::<McpServerImportDef>()
                .map_err(|error| AgentStowError::Manifest {
                    message: format!("解析 mcp_servers.file 失败：{error}").into(),
                })?;
            import_mcp_servers_from_file(&mut servers, workspace_root, &import.path)?;
            continue;
        }

        let server =
            value
                .clone()
                .try_into::<McpServerDef>()
                .map_err(|error| AgentStowError::Manifest {
                    message: format!("解析 mcp_servers.{name} 失败：{error}").into(),
                })?;
        if servers.insert(name.clone(), server).is_some() {
            return Err(AgentStowError::Manifest {
                message: format!("重复的 mcp server 名称：{name}").into(),
            });
        }
    }

    Ok(servers)
}

fn import_mcp_servers_from_file(
    servers: &mut BTreeMap<String, McpServerDef>,
    workspace_root: &Path,
    path: &Path,
) -> Result<()> {
    let absolute_path = absolutize(workspace_root, path);
    let content = std::fs::read_to_string(&absolute_path).map_err(AgentStowError::from)?;
    if let Ok(imported) = serde_json::from_str::<ImportedGenericMcpJsonFile>(&content) {
        for (name, server) in imported.mcp_servers {
            if servers.insert(name.clone(), server).is_some() {
                return Err(AgentStowError::Manifest {
                    message: format!("导入的 mcp server 与现有名称冲突：{name}").into(),
                });
            }
        }
        return Ok(());
    }

    let imported: ImportedMcpJsonFile =
        serde_json::from_str(&content).map_err(|error| AgentStowError::Manifest {
            message: format!(
                "解析 mcp server 导入文件失败：path={}, {error}",
                normalize_for_display(&absolute_path),
            )
            .into(),
        })?;

    for (name, server) in imported.mcp_servers {
        if servers.contains_key(&name) {
            return Err(AgentStowError::Manifest {
                message: format!("导入的 mcp server 与现有名称冲突：{name}").into(),
            });
        }
        servers.insert(name, imported_mcp_server_to_def(server));
    }

    Ok(())
}

fn imported_mcp_server_to_def(server: ImportedMcpJsonServer) -> McpServerDef {
    match server {
        ImportedMcpJsonServer::Stdio {
            command,
            args,
            env_vars,
            env,
            cwd,
            options,
        } => McpServerDef {
            transport: McpTransport::Stdio { command, args, cwd },
            env: imported_env_map_to_defs(env)
                .into_iter()
                .chain(env_vars.into_iter().map(|var| EnvVarDef {
                    key: var.clone(),
                    binding: SecretBinding::Env { var },
                }))
                .collect(),
            options: imported_mcp_server_options_to_def(options),
        },
        ImportedMcpJsonServer::Http {
            url,
            bearer_token_env_var,
            http_headers,
            env_http_headers,
            options,
        } => McpServerDef {
            transport: McpTransport::Http {
                url,
                headers: http_headers,
            },
            env: bearer_token_env_var
                .into_iter()
                .map(|var| EnvVarDef {
                    key: var.clone(),
                    binding: SecretBinding::Env { var },
                })
                .chain(env_http_headers.into_iter().map(|(key, var)| EnvVarDef {
                    key,
                    binding: SecretBinding::Env { var },
                }))
                .collect(),
            options: imported_mcp_server_options_to_def(options),
        },
    }
}

fn imported_mcp_server_options_to_def(options: ImportedMcpServerOptions) -> McpServerOptions {
    McpServerOptions {
        startup_timeout_sec: options.startup_timeout_sec,
        tool_timeout_sec: options.tool_timeout_sec,
        enabled: options.enabled,
        required: options.required,
        enabled_tools: options.enabled_tools,
        disabled_tools: options.disabled_tools,
        ..Default::default()
    }
}

fn imported_env_map_to_defs(env: BTreeMap<String, String>) -> Vec<EnvVarDef> {
    env.into_iter()
        .map(|(key, value)| EnvVarDef {
            key,
            binding: SecretBinding::Literal { value },
        })
        .collect()
}

pub fn init_workspace_skeleton(workspace_root: &Path) -> Result<WorkspaceInitOutcome> {
    fs_err::create_dir_all(workspace_root).map_err(AgentStowError::from)?;

    let manifest_path = workspace_root.join(DEFAULT_MANIFEST_FILE);
    let created = if manifest_path.exists() {
        false
    } else {
        fs_err::create_dir_all(workspace_root.join("artifacts")).map_err(AgentStowError::from)?;
        fs_err::write(
            workspace_root.join("artifacts/hello.txt.tera"),
            "Hello {{ name }}!",
        )
        .map_err(AgentStowError::from)?;
        fs_err::write(
            &manifest_path,
            r#"[profiles.base]
name = "AgentStow"

[artifacts.hello]
kind = "file"
source = "artifacts/hello.txt.tera"
template = true
validate_as = "none"
"#,
        )
        .map_err(AgentStowError::from)?;
        true
    };

    Ok(WorkspaceInitOutcome {
        workspace_root: workspace_root.to_path_buf(),
        manifest_path,
        created,
    })
}

pub fn probe_workspace_path(requested_path: &Path) -> Result<WorkspaceProbe> {
    let requested_path = absolutize_requested_path(requested_path)?;
    let manifest_file_request = is_manifest_file_path(&requested_path);
    let resolved_workspace_root = if manifest_file_request {
        requested_path
            .parent()
            .ok_or_else(|| AgentStowError::Manifest {
                message: "workspace probe 路径没有 parent".into(),
            })?
            .to_path_buf()
    } else {
        requested_path.clone()
    };

    if requested_path.exists() {
        let metadata = fs_err::metadata(&requested_path).map_err(AgentStowError::from)?;
        if metadata.is_dir() {
            return build_existing_workspace_probe(requested_path);
        }
        if manifest_file_request && metadata.is_file() {
            let resolved_workspace_root =
                fs_err::canonicalize(&resolved_workspace_root).map_err(AgentStowError::from)?;
            return Ok(WorkspaceProbe {
                manifest_path: resolved_workspace_root.join(DEFAULT_MANIFEST_FILE),
                git_present: resolved_workspace_root.join(".git").exists(),
                resolved_workspace_root,
                exists: true,
                is_directory: false,
                manifest_present: true,
                selectable: true,
                initializable: false,
                reason: None,
            });
        }

        return Ok(WorkspaceProbe {
            resolved_workspace_root,
            manifest_path: requested_path,
            exists: true,
            is_directory: false,
            manifest_present: false,
            git_present: false,
            selectable: false,
            initializable: false,
            reason: Some("路径是普通文件，不是 workspace 目录，也不是 agentstow.toml".to_string()),
        });
    }

    let reason = if manifest_file_request {
        "manifest 尚不存在，可初始化其所在目录为 workspace"
    } else {
        "路径不存在，可直接初始化 workspace"
    };

    Ok(WorkspaceProbe {
        manifest_path: if manifest_file_request {
            requested_path
        } else {
            resolved_workspace_root.join(DEFAULT_MANIFEST_FILE)
        },
        resolved_workspace_root,
        exists: false,
        is_directory: false,
        manifest_present: false,
        git_present: false,
        selectable: false,
        initializable: true,
        reason: Some(reason.to_string()),
    })
}

fn build_existing_workspace_probe(path: PathBuf) -> Result<WorkspaceProbe> {
    let resolved_workspace_root = fs_err::canonicalize(&path).map_err(AgentStowError::from)?;
    let manifest_path = resolved_workspace_root.join(DEFAULT_MANIFEST_FILE);
    let manifest_present = manifest_path.is_file();

    Ok(WorkspaceProbe {
        git_present: resolved_workspace_root.join(".git").exists(),
        resolved_workspace_root,
        manifest_path,
        exists: true,
        is_directory: true,
        manifest_present,
        selectable: true,
        initializable: !manifest_present,
        reason: (!manifest_present)
            .then_some("目录存在，但还没有 agentstow.toml，可直接初始化".to_string()),
    })
}

fn absolutize_requested_path(requested_path: &Path) -> Result<PathBuf> {
    if requested_path.is_absolute() {
        return Ok(requested_path.to_path_buf());
    }

    Ok(std::env::current_dir()
        .map_err(AgentStowError::from)?
        .join(requested_path))
}

fn is_manifest_file_path(path: &Path) -> bool {
    path.file_name()
        .is_some_and(|name| name == OsStr::new(DEFAULT_MANIFEST_FILE))
}

fn validate_manifest(m: &ManifestToml, workspace_root: &Path) -> Result<()> {
    for profile in m.profiles.values() {
        profile.merged_vars(&m.profiles)?;
    }

    let normalized_targets = m
        .targets
        .iter()
        .map(|(target_name, target)| {
            (
                target_name,
                normalize_path_without_following_symlinks(
                    &target.absolute_target_path(workspace_root),
                ),
            )
        })
        .collect::<Vec<_>>();

    for (index, (left_name, left_path)) in normalized_targets.iter().enumerate() {
        for (right_name, right_path) in normalized_targets.iter().skip(index + 1) {
            if !paths_overlap(left_path, right_path) {
                continue;
            }

            return Err(AgentStowError::Manifest {
                message: format!(
                    "targets target_path 发生重叠：{}={} <-> {}={}",
                    left_name,
                    normalize_for_display(left_path),
                    right_name,
                    normalize_for_display(right_path),
                )
                .into(),
            });
        }
    }

    for (target_name, target) in &m.targets {
        let Some(artifact) = m.artifacts.get(&target.artifact) else {
            return Err(AgentStowError::Manifest {
                message: format!(
                    "target 引用不存在的 artifact: {target_name} -> {}",
                    target.artifact
                )
                .into(),
            });
        };
        if let Some(profile) = &target.profile
            && !m.profiles.contains_key(profile)
        {
            return Err(AgentStowError::Manifest {
                message: format!("target 引用不存在的 profile: {target_name} -> {profile}").into(),
            });
        }

        let source_path = artifact.source_path(workspace_root);
        let target_path = target.absolute_target_path(workspace_root);
        if paths_overlap(&source_path, &target_path) {
            return Err(AgentStowError::Manifest {
                message: format!(
                    "target 路径与 artifact source 重叠：{target_name} -> {} (source={}, target={})",
                    target.artifact,
                    normalize_for_display(&source_path),
                    normalize_for_display(&target_path),
                )
                .into(),
            });
        }
    }
    Ok(())
}

fn paths_overlap(left: &Path, right: &Path) -> bool {
    left == right || left.starts_with(right) || right.starts_with(left)
}

fn normalize_path_without_following_symlinks(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            std::path::Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            std::path::Component::RootDir => {
                normalized.push(Path::new(std::path::MAIN_SEPARATOR_STR))
            }
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !normalized.pop() {
                    normalized.push(component.as_os_str());
                }
            }
            std::path::Component::Normal(part) => normalized.push(part),
        }
    }

    normalized
}

#[cfg(test)]
mod tests;
