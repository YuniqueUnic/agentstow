use std::sync::Arc;

use agentstow_core::{ArtifactId, ProfileName, normalize_for_display};
use agentstow_web_types::{
    ArtifactGitRollbackRequest, ArtifactSourceUpdateRequest, ManifestSourceUpdateRequest,
};
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use serde::Deserialize;

use crate::ServerState;
use crate::api::helpers::{
    api_error, handle_result, queries_from_state, record_watch_change, selected_workspace_root,
    watch_change_summary, workspace_relative_display,
};

#[derive(Debug, Deserialize)]
pub(super) struct RenderQuery {
    artifact: String,
    profile: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct ArtifactGitHistoryQuery {
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ArtifactGitCompareQuery {
    base: String,
    head: Option<String>,
}

pub(super) async fn api_manifest(State(st): State<Arc<ServerState>>) -> Response {
    let queries = match queries_from_state(&st).await {
        Ok(queries) => queries,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    match queries.manifest_overview() {
        Ok(response) => Json(response).into_response(),
        Err(error) => api_error(StatusCode::BAD_REQUEST, error),
    }
}

pub(super) async fn api_manifest_source(State(st): State<Arc<ServerState>>) -> Response {
    let queries = match queries_from_state(&st).await {
        Ok(queries) => queries,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    handle_result(queries.manifest_source())
}

pub(super) async fn api_manifest_source_update(
    State(st): State<Arc<ServerState>>,
    Json(req): Json<ManifestSourceUpdateRequest>,
) -> Response {
    let queries = match queries_from_state(&st).await {
        Ok(queries) => queries,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    match queries.update_manifest_source(&req.content) {
        Ok(response) => {
            if let Ok(workspace_root) = selected_workspace_root(&st).await {
                record_watch_change(
                    &st,
                    watch_change_summary(&workspace_root, &response.source_path, "save"),
                )
                .await;
            }
            Json(response).into_response()
        }
        Err(error) => api_error(StatusCode::BAD_REQUEST, error),
    }
}

pub(super) async fn api_render(
    State(st): State<Arc<ServerState>>,
    Query(query): Query<RenderQuery>,
) -> Response {
    let artifact_id = match ArtifactId::parse(query.artifact) {
        Ok(artifact_id) => artifact_id,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };
    let profile = match ProfileName::parse(query.profile) {
        Ok(profile) => profile,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    let queries = match queries_from_state(&st).await {
        Ok(queries) => queries,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    handle_result(queries.render_preview(&artifact_id, &profile))
}

pub(super) async fn api_artifact_detail(
    State(st): State<Arc<ServerState>>,
    AxumPath(artifact): AxumPath<String>,
) -> Response {
    let artifact_id = match ArtifactId::parse(artifact) {
        Ok(artifact_id) => artifact_id,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    let queries = match queries_from_state(&st).await {
        Ok(queries) => queries,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    handle_result(queries.artifact_detail(&artifact_id))
}

pub(super) async fn api_artifact_source(
    State(st): State<Arc<ServerState>>,
    AxumPath(artifact): AxumPath<String>,
) -> Response {
    let artifact_id = match ArtifactId::parse(artifact) {
        Ok(artifact_id) => artifact_id,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    let queries = match queries_from_state(&st).await {
        Ok(queries) => queries,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    handle_result(queries.artifact_source(&artifact_id))
}

pub(super) async fn api_artifact_source_update(
    State(st): State<Arc<ServerState>>,
    AxumPath(artifact): AxumPath<String>,
    Json(req): Json<ArtifactSourceUpdateRequest>,
) -> Response {
    let artifact_id = match ArtifactId::parse(artifact) {
        Ok(artifact_id) => artifact_id,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    let queries = match queries_from_state(&st).await {
        Ok(queries) => queries,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    match queries.update_artifact_source(&artifact_id, &req.content) {
        Ok(response) => {
            if let Ok(workspace_root) = selected_workspace_root(&st).await {
                record_watch_change(
                    &st,
                    watch_change_summary(&workspace_root, &response.source_path, "save"),
                )
                .await;
            }
            Json(response).into_response()
        }
        Err(error) => api_error(StatusCode::BAD_REQUEST, error),
    }
}

pub(super) async fn api_artifact_git_history(
    State(st): State<Arc<ServerState>>,
    AxumPath(artifact): AxumPath<String>,
    Query(query): Query<ArtifactGitHistoryQuery>,
) -> Response {
    let artifact_id = match ArtifactId::parse(artifact) {
        Ok(artifact_id) => artifact_id,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    let queries = match queries_from_state(&st).await {
        Ok(queries) => queries,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    handle_result(
        queries
            .artifact_git_history(&artifact_id, query.limit.unwrap_or(20))
            .await,
    )
}

pub(super) async fn api_artifact_git_compare(
    State(st): State<Arc<ServerState>>,
    AxumPath(artifact): AxumPath<String>,
    Query(query): Query<ArtifactGitCompareQuery>,
) -> Response {
    if query.base.trim().is_empty() {
        return api_error(StatusCode::BAD_REQUEST, "base revision 不能为空");
    }

    let artifact_id = match ArtifactId::parse(artifact) {
        Ok(artifact_id) => artifact_id,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    let queries = match queries_from_state(&st).await {
        Ok(queries) => queries,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    let head = query
        .head
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(agentstow_git::WORKTREE_REVISION);
    handle_result(
        queries
            .artifact_git_compare(&artifact_id, &query.base, head)
            .await,
    )
}

pub(super) async fn api_artifact_git_rollback(
    State(st): State<Arc<ServerState>>,
    AxumPath(artifact): AxumPath<String>,
    Json(req): Json<ArtifactGitRollbackRequest>,
) -> Response {
    if req.revision.trim().is_empty() {
        return api_error(StatusCode::BAD_REQUEST, "revision 不能为空");
    }

    let artifact_id = match ArtifactId::parse(artifact) {
        Ok(artifact_id) => artifact_id,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    let queries = match queries_from_state(&st).await {
        Ok(queries) => queries,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    match queries
        .artifact_git_rollback(&artifact_id, &req.revision)
        .await
    {
        Ok(response) => {
            if let Ok(workspace_root) = selected_workspace_root(&st).await {
                let summary = format!(
                    "rollback {} <= {}",
                    workspace_relative_display(&workspace_root, &response.source.source_path),
                    response.commit.short_revision
                );
                record_watch_change(&st, summary).await;
            }
            Json(response).into_response()
        }
        Err(error) => api_error(StatusCode::BAD_REQUEST, error),
    }
}
