use std::path::{Path, PathBuf};

use agentstow_core::{AgentStowError, Result, normalize_for_display};

use crate::ManifestToml;

pub(crate) fn validate_manifest(m: &ManifestToml, workspace_root: &Path) -> Result<()> {
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
