use std::path::{Path, PathBuf};

use agentstow_core::{AgentStowError, ProfileName, Result};
use agentstow_manifest::Manifest;

use crate::cli::Cli;

#[derive(Debug, Clone)]
pub struct CommandContext {
    cwd: PathBuf,
    json: bool,
    workspace: Option<PathBuf>,
    default_profile: Option<String>,
    timeout: Option<u64>,
}

impl CommandContext {
    pub fn from_cli(cli: &Cli) -> Result<Self> {
        Ok(Self {
            cwd: std::env::current_dir().map_err(AgentStowError::from)?,
            json: cli.json,
            workspace: cli.workspace.clone(),
            default_profile: cli.profile.clone(),
            timeout: cli.timeout,
        })
    }

    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    pub fn json(&self) -> bool {
        self.json
    }

    pub fn timeout(&self) -> Option<u64> {
        self.timeout
    }

    pub fn workspace_init_root(&self) -> PathBuf {
        self.workspace.clone().unwrap_or_else(|| self.cwd.clone())
    }

    pub fn default_profile_unchecked(&self) -> Option<ProfileName> {
        self.default_profile.clone().map(ProfileName::new_unchecked)
    }

    pub fn resolve_workspace_root_optional(&self) -> Result<Option<PathBuf>> {
        resolve_workspace_root_optional(&self.cwd, self.workspace.as_deref())
    }

    pub fn load_manifest(&self) -> Result<Manifest> {
        load_manifest(&self.cwd, self.workspace.as_deref())
    }

    pub fn resolve_profile(
        &self,
        override_profile: Option<&str>,
        manifest: Option<&Manifest>,
    ) -> Result<ProfileName> {
        resolve_profile(self.default_profile.as_deref(), override_profile, manifest)
    }
}

pub fn apply_cli_cwd(cwd: Option<&Path>) -> Result<()> {
    if let Some(cwd) = cwd {
        std::env::set_current_dir(cwd).map_err(AgentStowError::from)?;
    }
    Ok(())
}

fn resolve_workspace_root(cwd: &Path, override_workspace: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = override_workspace {
        return Ok(path.to_path_buf());
    }

    let manifest_path = Manifest::find_from(cwd)?;
    Ok(manifest_path
        .parent()
        .ok_or_else(|| AgentStowError::Manifest {
            message: "manifest path 没有 parent".into(),
        })?
        .to_path_buf())
}

fn resolve_workspace_root_optional(
    cwd: &Path,
    override_workspace: Option<&Path>,
) -> Result<Option<PathBuf>> {
    if let Some(path) = override_workspace {
        return Ok(Some(path.to_path_buf()));
    }

    match Manifest::find_from(cwd) {
        Ok(manifest_path) => Ok(Some(
            manifest_path
                .parent()
                .ok_or_else(|| AgentStowError::Manifest {
                    message: "manifest path 没有 parent".into(),
                })?
                .to_path_buf(),
        )),
        Err(_) => Ok(None),
    }
}

fn load_manifest(cwd: &Path, override_workspace: Option<&Path>) -> Result<Manifest> {
    let workspace_root = resolve_workspace_root(cwd, override_workspace)?;
    Manifest::load_from_dir(&workspace_root)
}

fn resolve_profile(
    default_profile: Option<&str>,
    override_profile: Option<&str>,
    manifest: Option<&Manifest>,
) -> Result<ProfileName> {
    let chosen = override_profile
        .map(str::to_string)
        .or_else(|| default_profile.map(str::to_string))
        .ok_or_else(|| AgentStowError::InvalidArgs {
            message: "需要指定 --profile（或在 target 内配置 profile）".into(),
        })?;
    let profile = ProfileName::parse(chosen)?;
    if let Some(manifest) = manifest
        && !manifest.profiles.contains_key(&profile)
    {
        return Err(AgentStowError::Manifest {
            message: format!("profile 不存在：{}", profile.as_str()).into(),
        });
    }
    Ok(profile)
}
