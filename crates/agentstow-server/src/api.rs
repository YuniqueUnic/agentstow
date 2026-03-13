use std::sync::Arc;

use agentstow_core::{AgentStowError, ArtifactId, ProfileName};
use agentstow_web_types::{ApiError, HealthResponse};
use axum::Router;
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{any, get};
use serde::Deserialize;

use crate::ServerState;

pub(crate) fn routes() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/api/health", get(api_health))
        .route("/api/manifest", get(api_manifest))
        .route("/api/render", get(api_render))
        .route("/api/links", get(api_links))
        .route("/api/link-status", get(api_link_status))
        .route("/api/watch-status", get(api_watch_status))
        .route("/api/workspace-summary", get(api_workspace_summary))
        .route("/api/artifacts/{artifact}", get(api_artifact_detail))
        .route("/api/profiles/{profile}", get(api_profile_detail))
        .route("/api/impact", get(api_impact))
        .route("/api/{*path}", any(api_not_found))
}

#[derive(Debug, Deserialize)]
struct RenderQuery {
    artifact: String,
    profile: String,
}

#[derive(Debug, Deserialize)]
struct ImpactQuery {
    artifact: Option<String>,
    profile: Option<String>,
}

async fn api_not_found() -> Response {
    api_error(StatusCode::NOT_FOUND, "api 路由不存在")
}

fn api_error(status: StatusCode, message: impl ToString) -> Response {
    (
        status,
        Json(ApiError {
            message: message.to_string(),
        }),
    )
        .into_response()
}

async fn api_health() -> Json<HealthResponse> {
    Json(HealthResponse { ok: true })
}

async fn api_manifest(State(st): State<Arc<ServerState>>) -> Response {
    match st.queries.manifest_overview() {
        Ok(response) => Json(response).into_response(),
        Err(error) => api_error(StatusCode::BAD_REQUEST, error),
    }
}

async fn api_render(
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

    handle_result(st.queries.render_preview(&artifact_id, &profile))
}

async fn api_links(State(st): State<Arc<ServerState>>) -> Response {
    match st.queries.link_records() {
        Ok(records) => Json(records).into_response(),
        Err(error) => api_error(StatusCode::INTERNAL_SERVER_ERROR, error),
    }
}

async fn api_link_status(State(st): State<Arc<ServerState>>) -> Response {
    match st.queries.link_status() {
        Ok(status) => Json(status).into_response(),
        Err(error) => api_error(StatusCode::INTERNAL_SERVER_ERROR, error),
    }
}

async fn api_watch_status(State(st): State<Arc<ServerState>>) -> Response {
    Json(crate::services::watch_status_response(st.watch.snapshot())).into_response()
}

async fn api_workspace_summary(State(st): State<Arc<ServerState>>) -> Response {
    handle_result(st.queries.workspace_summary())
}

async fn api_artifact_detail(
    State(st): State<Arc<ServerState>>,
    AxumPath(artifact): AxumPath<String>,
) -> Response {
    let artifact_id = match ArtifactId::parse(artifact) {
        Ok(artifact_id) => artifact_id,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    handle_result(st.queries.artifact_detail(&artifact_id))
}

async fn api_profile_detail(
    State(st): State<Arc<ServerState>>,
    AxumPath(profile): AxumPath<String>,
) -> Response {
    let profile_name = match ProfileName::parse(profile) {
        Ok(profile_name) => profile_name,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    handle_result(st.queries.profile_detail(&profile_name))
}

async fn api_impact(
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

    handle_result(
        st.queries
            .impact_analysis(artifact.as_ref(), profile.as_ref()),
    )
}

fn handle_result<T: serde::Serialize>(result: Result<T, AgentStowError>) -> Response {
    match result {
        Ok(payload) => Json(payload).into_response(),
        Err(error) => {
            let status = match error {
                AgentStowError::InvalidArgs { .. }
                | AgentStowError::Manifest { .. }
                | AgentStowError::Render { .. }
                | AgentStowError::Validate { .. } => StatusCode::BAD_REQUEST,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            api_error(status, error)
        }
    }
}
