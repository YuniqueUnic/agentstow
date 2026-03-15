use std::path::{Component, Path};

use agentstow_core::{AgentStowError, ArtifactKind, InstallMethod, Result, ValidateAs};
use agentstow_mcp::McpDryRunCheckStatus;
use agentstow_web_types::{
    ArtifactKindResponse, InstallMethodResponse, McpCheckStatusResponse, ShellKindResponse,
    ValidateAsResponse, WatchModeResponse, WatchStatusResponse, WatchTraceEventResponse,
    WatchTraceLevelResponse,
};

use crate::watch::{WatchMode, WatchStatusSnapshot, WatchTraceLevel};

pub(crate) fn shell_kind(shell: ShellKindResponse) -> agentstow_core::ShellKind {
    match shell {
        ShellKindResponse::Bash => agentstow_core::ShellKind::Bash,
        ShellKindResponse::Zsh => agentstow_core::ShellKind::Zsh,
        ShellKindResponse::Fish => agentstow_core::ShellKind::Fish,
        ShellKindResponse::Powershell => agentstow_core::ShellKind::Powershell,
        ShellKindResponse::Cmd => agentstow_core::ShellKind::Cmd,
    }
}

pub(crate) fn workspace_relative_display(workspace_root: &Path, path: &Path) -> String {
    let canonical_root =
        fs_err::canonicalize(workspace_root).unwrap_or_else(|_| workspace_root.to_path_buf());
    let canonical_path = fs_err::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());

    canonical_path
        .strip_prefix(&canonical_root)
        .or_else(|_| path.strip_prefix(workspace_root))
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

pub(crate) fn mcp_check_status_response(status: McpDryRunCheckStatus) -> McpCheckStatusResponse {
    match status {
        McpDryRunCheckStatus::Ok => McpCheckStatusResponse::Ok,
        McpDryRunCheckStatus::Warn => McpCheckStatusResponse::Warn,
        McpDryRunCheckStatus::Error => McpCheckStatusResponse::Error,
    }
}

pub(crate) fn ensure_safe_workspace_relative_path(path: &Path) -> Result<()> {
    if path.is_absolute() {
        return Err(AgentStowError::InvalidArgs {
            message: "不允许编辑绝对路径 source（请改为 workspace 内相对路径）".into(),
        });
    }

    for component in path.components() {
        match component {
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(AgentStowError::InvalidArgs {
                    message: "artifact source 仅允许 workspace 内的安全相对路径（禁止 ..）".into(),
                });
            }
            Component::CurDir | Component::Normal(_) => {}
        }
    }

    Ok(())
}

pub(crate) fn install_method_response(method: InstallMethod) -> InstallMethodResponse {
    match method {
        InstallMethod::Symlink => InstallMethodResponse::Symlink,
        InstallMethod::Junction => InstallMethodResponse::Junction,
        InstallMethod::Copy => InstallMethodResponse::Copy,
    }
}

pub(crate) fn artifact_kind_response(kind: ArtifactKind) -> ArtifactKindResponse {
    match kind {
        ArtifactKind::File => ArtifactKindResponse::File,
        ArtifactKind::Dir => ArtifactKindResponse::Dir,
    }
}

pub(crate) fn validate_as_response(validate_as: ValidateAs) -> ValidateAsResponse {
    match validate_as {
        ValidateAs::None => ValidateAsResponse::None,
        ValidateAs::Json => ValidateAsResponse::Json,
        ValidateAs::Toml => ValidateAsResponse::Toml,
        ValidateAs::Markdown => ValidateAsResponse::Markdown,
        ValidateAs::Shell => ValidateAsResponse::Shell,
    }
}

pub(crate) fn watch_status_response(snapshot: WatchStatusSnapshot) -> WatchStatusResponse {
    WatchStatusResponse {
        mode: match snapshot.mode {
            WatchMode::Native => WatchModeResponse::Native,
            WatchMode::Poll => WatchModeResponse::Poll,
            WatchMode::Manual => WatchModeResponse::Manual,
        },
        healthy: snapshot.healthy,
        revision: snapshot.revision,
        poll_interval_ms: snapshot.poll_interval_ms,
        last_event: snapshot.last_event,
        last_event_at: snapshot.last_event_at,
        last_error: snapshot.last_error,
        watch_roots: snapshot.watch_roots,
        recent_events: snapshot
            .recent_events
            .into_iter()
            .map(|event| WatchTraceEventResponse {
                revision: event.revision,
                level: match event.level {
                    WatchTraceLevel::Change => WatchTraceLevelResponse::Change,
                    WatchTraceLevel::Error => WatchTraceLevelResponse::Error,
                },
                summary: event.summary,
                at: event.at,
            })
            .collect(),
    }
}
