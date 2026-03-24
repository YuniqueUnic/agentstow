use std::sync::Arc;

use agentstow_web_types::{
    HealthResponse, WorkspaceGitSummaryResponse, WorkspaceInitRequest, WorkspaceInitResponse,
    WorkspaceProbeRequest, WorkspaceSelectRequest, WorkspaceSelectResponse, WorkspaceStateResponse,
};
use axum::extract::{Json, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json as JsonResp, Response};

use crate::ServerState;
use crate::api::helpers::{
    api_error, handle_result, pick_workspace_folder, probe_workspace, select_workspace,
    workspace_probe_response, workspace_select_response,
};
use crate::services::WorkspaceQueryService;

pub(super) async fn api_health() -> JsonResp<HealthResponse> {
    JsonResp(HealthResponse { ok: true })
}

pub(super) async fn api_workspace_state(State(st): State<Arc<ServerState>>) -> Response {
    let workspace_root = st.workspace_root.read().await.clone();
    let workspace = match workspace_root {
        Some(workspace_root) => {
            let requested_workspace_root = agentstow_core::normalize_for_display(&workspace_root);
            match probe_workspace(&requested_workspace_root) {
                Ok(probe) => Some(workspace_probe_response(&requested_workspace_root, probe)),
                Err(error) => return api_error(StatusCode::INTERNAL_SERVER_ERROR, error),
            }
        }
        None => None,
    };

    JsonResp(WorkspaceStateResponse {
        workspace_root: workspace
            .as_ref()
            .map(|workspace| workspace.resolved_workspace_root.clone()),
        manifest_present: workspace
            .as_ref()
            .is_some_and(|workspace| workspace.manifest_present),
        workspace,
    })
    .into_response()
}

pub(super) async fn api_workspace_probe(Json(req): Json<WorkspaceProbeRequest>) -> Response {
    let requested_workspace_root = req.workspace_root.trim();
    if requested_workspace_root.is_empty() {
        return api_error(StatusCode::BAD_REQUEST, "workspace_root 不能为空");
    }

    match probe_workspace(requested_workspace_root) {
        Ok(probe) => {
            JsonResp(workspace_probe_response(requested_workspace_root, probe)).into_response()
        }
        Err(error) => api_error(StatusCode::BAD_REQUEST, error),
    }
}

pub(super) async fn api_workspace_git(State(st): State<Arc<ServerState>>) -> Response {
    let workspace_root = st.workspace_root.read().await.clone();
    let Some(workspace_root) = workspace_root else {
        return JsonResp::<Option<WorkspaceGitSummaryResponse>>(None).into_response();
    };

    let queries = WorkspaceQueryService::new(workspace_root);
    handle_result(queries.workspace_git().await)
}

pub(super) async fn api_workspace_pick(State(st): State<Arc<ServerState>>) -> Response {
    let picked = match tokio::task::spawn_blocking(pick_workspace_folder).await {
        Ok(Ok(path)) => path,
        Ok(Err(error)) => return api_error(StatusCode::BAD_REQUEST, error),
        Err(error) => {
            return api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("workspace picker 执行失败：{error}"),
            );
        }
    };

    let Some(workspace_root) = picked else {
        return JsonResp::<Option<WorkspaceSelectResponse>>(None).into_response();
    };

    let requested_workspace_root = agentstow_core::normalize_for_display(&workspace_root);
    let probe = match probe_workspace(&requested_workspace_root) {
        Ok(probe) => probe,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };
    if !probe.selectable {
        return api_error(
            StatusCode::BAD_REQUEST,
            probe.reason.as_deref().unwrap_or("workspace 路径不可选择"),
        );
    }

    let response = workspace_select_response(&requested_workspace_root, probe.clone());
    select_workspace(&st, probe.resolved_workspace_root).await;

    JsonResp(Some(response)).into_response()
}

pub(super) async fn api_workspace_select(
    State(st): State<Arc<ServerState>>,
    Json(req): Json<WorkspaceSelectRequest>,
) -> Response {
    let requested_workspace_root = req.workspace_root.trim();
    if requested_workspace_root.is_empty() {
        return api_error(StatusCode::BAD_REQUEST, "workspace_root 不能为空");
    }

    let probe = match probe_workspace(requested_workspace_root) {
        Ok(probe) => probe,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };
    if !probe.selectable {
        return api_error(
            StatusCode::BAD_REQUEST,
            probe.reason.as_deref().unwrap_or("workspace 路径不可选择"),
        );
    }
    let response = workspace_select_response(requested_workspace_root, probe.clone());
    select_workspace(&st, probe.resolved_workspace_root).await;

    JsonResp(response).into_response()
}

pub(super) async fn api_workspace_init(
    State(st): State<Arc<ServerState>>,
    Json(req): Json<WorkspaceInitRequest>,
) -> Response {
    let requested_workspace_root = req.workspace_root.trim();
    if requested_workspace_root.is_empty() {
        return api_error(StatusCode::BAD_REQUEST, "workspace_root 不能为空");
    }

    let probe = match probe_workspace(requested_workspace_root) {
        Ok(probe) => probe,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };
    if probe.exists && !probe.is_directory && !probe.manifest_present {
        return api_error(
            StatusCode::BAD_REQUEST,
            probe
                .reason
                .as_deref()
                .unwrap_or("workspace 路径不可初始化"),
        );
    }

    let workspace_root = probe.resolved_workspace_root.clone();
    let created = match agentstow_manifest::init_workspace_skeleton(&workspace_root) {
        Ok(outcome) => outcome.created,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    if req.git_init && !workspace_root.join(".git").exists() {
        match tokio::process::Command::new("git")
            .arg("init")
            .current_dir(&workspace_root)
            .status()
            .await
        {
            Ok(status) if status.success() => {}
            Ok(status) => {
                return api_error(
                    StatusCode::BAD_REQUEST,
                    format!("git init 失败（exit={}）", status.code().unwrap_or(-1)),
                );
            }
            Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
        }
    }

    let selected_probe = match probe_workspace(requested_workspace_root) {
        Ok(probe) => probe,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };
    let response = workspace_probe_response(requested_workspace_root, selected_probe.clone());
    select_workspace(&st, selected_probe.resolved_workspace_root).await;

    JsonResp(WorkspaceInitResponse {
        workspace_root: response.resolved_workspace_root.clone(),
        manifest_path: response.manifest_path.clone(),
        created,
        workspace: response,
    })
    .into_response()
}

pub(super) async fn api_watch_status(State(st): State<Arc<ServerState>>) -> Response {
    let snapshot = st.watch.read().await.snapshot();
    JsonResp(crate::services::watch_status_response(snapshot)).into_response()
}
