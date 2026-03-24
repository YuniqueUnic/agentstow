use std::sync::Arc;

use agentstow_core::{ArtifactId, ProfileName, normalize_for_display};
use agentstow_web_types::{EnvEmitRequest, ProfileVarsUpdateRequest, ScriptRunRequest};
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use serde::Deserialize;

use crate::ServerState;
use crate::api::helpers::{
    api_error, handle_result, queries_from_state, record_watch_change, selected_workspace_root,
    watch_change_summary,
};

#[derive(Debug, Deserialize)]
pub(super) struct ImpactQuery {
    artifact: Option<String>,
    profile: Option<String>,
}

pub(super) async fn api_workspace_summary(State(st): State<Arc<ServerState>>) -> Response {
    let queries = match queries_from_state(&st).await {
        Ok(queries) => queries,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    handle_result(queries.workspace_summary())
}

pub(super) async fn api_mcp_validate(
    State(st): State<Arc<ServerState>>,
    AxumPath(server): AxumPath<String>,
) -> Response {
    if server.trim().is_empty() {
        return api_error(StatusCode::BAD_REQUEST, "server id 不能为空");
    }

    let queries = match queries_from_state(&st).await {
        Ok(queries) => queries,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    handle_result(queries.mcp_validate(&server))
}

pub(super) async fn api_mcp_render(
    State(st): State<Arc<ServerState>>,
    AxumPath(server): AxumPath<String>,
) -> Response {
    if server.trim().is_empty() {
        return api_error(StatusCode::BAD_REQUEST, "server id 不能为空");
    }

    let queries = match queries_from_state(&st).await {
        Ok(queries) => queries,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    handle_result(queries.mcp_render(&server))
}

pub(super) async fn api_mcp_test(
    State(st): State<Arc<ServerState>>,
    AxumPath(server): AxumPath<String>,
) -> Response {
    if server.trim().is_empty() {
        return api_error(StatusCode::BAD_REQUEST, "server id 不能为空");
    }

    let queries = match queries_from_state(&st).await {
        Ok(queries) => queries,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    handle_result(queries.mcp_test(&server))
}

pub(super) async fn api_env_emit(
    State(st): State<Arc<ServerState>>,
    Json(req): Json<EnvEmitRequest>,
) -> Response {
    if req.set.as_ref().is_some_and(|set| set.trim().is_empty()) {
        return api_error(StatusCode::BAD_REQUEST, "set 不能为空");
    }

    let queries = match queries_from_state(&st).await {
        Ok(queries) => queries,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    handle_result(queries.env_emit(req.set.as_deref(), req.shell))
}

pub(super) async fn api_script_run(
    State(st): State<Arc<ServerState>>,
    AxumPath(script): AxumPath<String>,
    Json(req): Json<ScriptRunRequest>,
) -> Response {
    if script.trim().is_empty() {
        return api_error(StatusCode::BAD_REQUEST, "script id 不能为空");
    }

    let queries = match queries_from_state(&st).await {
        Ok(queries) => queries,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    handle_result(queries.script_run(&script, req.stdin.as_deref()).await)
}

pub(super) async fn api_profile_detail(
    State(st): State<Arc<ServerState>>,
    AxumPath(profile): AxumPath<String>,
) -> Response {
    let profile_name = match ProfileName::parse(profile) {
        Ok(profile_name) => profile_name,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    let queries = match queries_from_state(&st).await {
        Ok(queries) => queries,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    handle_result(queries.profile_detail(&profile_name))
}

pub(super) async fn api_profile_vars_update(
    State(st): State<Arc<ServerState>>,
    AxumPath(profile): AxumPath<String>,
    Json(req): Json<ProfileVarsUpdateRequest>,
) -> Response {
    let profile_name = match ProfileName::parse(profile) {
        Ok(profile_name) => profile_name,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    let queries = match queries_from_state(&st).await {
        Ok(queries) => queries,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    match queries.update_profile_vars(&profile_name, &req.vars) {
        Ok(response) => {
            if let Ok(workspace_root) = selected_workspace_root(&st).await {
                let manifest_path = workspace_root.join(agentstow_manifest::DEFAULT_MANIFEST_FILE);
                record_watch_change(
                    &st,
                    watch_change_summary(
                        &workspace_root,
                        &normalize_for_display(&manifest_path),
                        "save",
                    ),
                )
                .await;
            }
            Json(response).into_response()
        }
        Err(error) => api_error(StatusCode::BAD_REQUEST, error),
    }
}

pub(super) async fn api_impact(
    State(st): State<Arc<ServerState>>,
    Query(query): Query<ImpactQuery>,
) -> Response {
    let artifact = match query.artifact {
        Some(artifact) => match ArtifactId::parse(artifact) {
            Ok(artifact_id) => Some(artifact_id),
            Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
        },
        None => None,
    };
    let profile = match query.profile {
        Some(profile) => match ProfileName::parse(profile) {
            Ok(profile_name) => Some(profile_name),
            Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
        },
        None => None,
    };

    let queries = match queries_from_state(&st).await {
        Ok(queries) => queries,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    handle_result(queries.impact_analysis(artifact.as_ref(), profile.as_ref()))
}
