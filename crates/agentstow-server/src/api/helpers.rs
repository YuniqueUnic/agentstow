use std::path::Path;
use std::sync::Arc;

use agentstow_core::{AgentStowError, Result, normalize_for_display};
use agentstow_web_types::{ApiError, WorkspaceProbeResponse, WorkspaceSelectResponse};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};

use crate::ServerState;
use crate::services::WorkspaceQueryService;

pub(super) fn api_error(status: StatusCode, message: impl ToString) -> Response {
    (
        status,
        Json(ApiError {
            message: message.to_string(),
        }),
    )
        .into_response()
}

pub(super) fn handle_result<T: serde::Serialize>(result: Result<T, AgentStowError>) -> Response {
    match result {
        Ok(payload) => Json(payload).into_response(),
        Err(error) => {
            let status = match error {
                AgentStowError::InvalidArgs { .. }
                | AgentStowError::Manifest { .. }
                | AgentStowError::Render { .. }
                | AgentStowError::Validate { .. }
                | AgentStowError::Mcp { .. }
                | AgentStowError::Git { .. } => StatusCode::BAD_REQUEST,
                AgentStowError::LinkConflict { .. } => StatusCode::CONFLICT,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            api_error(status, error)
        }
    }
}

pub(super) async fn queries_from_state(
    st: &Arc<ServerState>,
) -> Result<WorkspaceQueryService, AgentStowError> {
    let workspace_root = selected_workspace_root(st).await?;
    Ok(WorkspaceQueryService::new(workspace_root))
}

pub(super) async fn selected_workspace_root(
    st: &Arc<ServerState>,
) -> Result<std::path::PathBuf, AgentStowError> {
    st.workspace_root
        .read()
        .await
        .clone()
        .ok_or_else(|| AgentStowError::InvalidArgs {
            message: "workspace 未选择，请先通过 /api/workspace 设置或使用 CLI --workspace".into(),
        })
}

pub(super) async fn record_watch_change(st: &Arc<ServerState>, summary: String) {
    let watch = st.watch.read().await.clone();
    watch.record_change(summary);
}

pub(super) fn watch_change_summary(
    workspace_root: &Path,
    source_path: &str,
    action: &str,
) -> String {
    format!(
        "{action} {}",
        workspace_relative_display(workspace_root, source_path)
    )
}

pub(super) fn workspace_relative_display(workspace_root: &Path, source_path: &str) -> String {
    let source_path = Path::new(source_path);
    match source_path.strip_prefix(workspace_root) {
        Ok(relative) => normalize_for_display(relative),
        Err(_) => source_path.display().to_string(),
    }
}

pub(super) fn probe_workspace(
    requested_workspace_root: &str,
) -> Result<agentstow_manifest::WorkspaceProbe, AgentStowError> {
    agentstow_manifest::probe_workspace_path(Path::new(requested_workspace_root))
}

pub(super) fn workspace_probe_response(
    requested_workspace_root: &str,
    probe: agentstow_manifest::WorkspaceProbe,
) -> WorkspaceProbeResponse {
    WorkspaceProbeResponse {
        requested_workspace_root: requested_workspace_root.to_string(),
        resolved_workspace_root: normalize_for_display(&probe.resolved_workspace_root),
        exists: probe.exists,
        is_directory: probe.is_directory,
        manifest_present: probe.manifest_present,
        manifest_path: normalize_for_display(&probe.manifest_path),
        git_present: probe.git_present,
        selectable: probe.selectable,
        initializable: probe.initializable,
        reason: probe.reason,
    }
}

pub(super) fn workspace_select_response(
    requested_workspace_root: &str,
    probe: agentstow_manifest::WorkspaceProbe,
) -> WorkspaceSelectResponse {
    let workspace = workspace_probe_response(requested_workspace_root, probe);
    WorkspaceSelectResponse {
        workspace_root: workspace.resolved_workspace_root.clone(),
        manifest_present: workspace.manifest_present,
        workspace,
    }
}

pub(super) fn pick_workspace_folder() -> Result<Option<std::path::PathBuf>, AgentStowError> {
    Ok(rfd::FileDialog::new()
        .set_title("Select AgentStow workspace")
        .pick_folder())
}

pub(super) async fn select_workspace(st: &Arc<ServerState>, workspace_root: std::path::PathBuf) {
    {
        let mut guard = st.workspace_root.write().await;
        *guard = Some(workspace_root.clone());
    }
    {
        let mut guard = st.watch.write().await;
        *guard = crate::watch::WatchStatusHandle::start(workspace_root);
    }
}
