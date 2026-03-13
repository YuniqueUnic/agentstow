use std::sync::Arc;

use agentstow_core::{AgentStowError, ArtifactId, ProfileName};
use agentstow_web_types::{
    ApiError, ArtifactSourceUpdateRequest, HealthResponse, WorkspaceInitRequest,
    WorkspaceInitResponse, WorkspaceSelectRequest, WorkspaceSelectResponse, WorkspaceStateResponse,
};
use axum::Router;
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{any, get, post};
use serde::Deserialize;

use crate::ServerState;
use crate::services::WorkspaceQueryService;

pub(crate) fn routes() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/api/health", get(api_health))
        .route("/api/workspace", get(api_workspace_state).post(api_workspace_select))
        .route("/api/workspace/init", post(api_workspace_init))
        .route("/api/manifest", get(api_manifest))
        .route("/api/render", get(api_render))
        .route("/api/links", get(api_links))
        .route("/api/link-status", get(api_link_status))
        .route("/api/watch-status", get(api_watch_status))
        .route("/api/workspace-summary", get(api_workspace_summary))
        .route("/api/artifacts/{artifact}", get(api_artifact_detail))
        .route(
            "/api/artifacts/{artifact}/source",
            get(api_artifact_source).put(api_artifact_source_update),
        )
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

async fn api_workspace_state(State(st): State<Arc<ServerState>>) -> Response {
    let workspace_root = st.workspace_root.read().await.clone();
    let manifest_present = workspace_root
        .as_ref()
        .is_some_and(|root| root.join(agentstow_manifest::DEFAULT_MANIFEST_FILE).is_file());

    Json(WorkspaceStateResponse {
        workspace_root: workspace_root
            .as_deref()
            .map(agentstow_core::normalize_for_display),
        manifest_present,
    })
    .into_response()
}

async fn api_workspace_select(
    State(st): State<Arc<ServerState>>,
    Json(req): Json<WorkspaceSelectRequest>,
) -> Response {
    if req.workspace_root.trim().is_empty() {
        return api_error(StatusCode::BAD_REQUEST, "workspace_root 不能为空");
    }

    let raw_path = std::path::PathBuf::from(req.workspace_root);
    let canonical = match fs_err::canonicalize(&raw_path) {
        Ok(path) => path,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    {
        let mut guard = st.workspace_root.write().await;
        *guard = Some(canonical.clone());
    }
    {
        let mut guard = st.watch.write().await;
        *guard = crate::watch::WatchStatusHandle::start(canonical.clone());
    }

    let manifest_present = canonical
        .join(agentstow_manifest::DEFAULT_MANIFEST_FILE)
        .is_file();

    Json(WorkspaceSelectResponse {
        workspace_root: agentstow_core::normalize_for_display(&canonical),
        manifest_present,
    })
    .into_response()
}

async fn api_workspace_init(
    State(st): State<Arc<ServerState>>,
    Json(req): Json<WorkspaceInitRequest>,
) -> Response {
    if req.workspace_root.trim().is_empty() {
        return api_error(StatusCode::BAD_REQUEST, "workspace_root 不能为空");
    }

    let workspace_root = std::path::PathBuf::from(req.workspace_root);
    if let Err(error) = fs_err::create_dir_all(&workspace_root) {
        return api_error(StatusCode::BAD_REQUEST, error);
    }

    let manifest_path = workspace_root.join(agentstow_manifest::DEFAULT_MANIFEST_FILE);
    let created = if manifest_path.exists() {
        false
    } else {
        if let Err(error) = fs_err::create_dir_all(workspace_root.join("artifacts")) {
            return api_error(StatusCode::BAD_REQUEST, error);
        }
        if let Err(error) = fs_err::write(
            workspace_root.join("artifacts/hello.txt.tera"),
            "Hello {{ name }}!",
        ) {
            return api_error(StatusCode::BAD_REQUEST, error);
        }
        if let Err(error) = fs_err::write(
            &manifest_path,
            r#"[profiles.base]
vars = { name = "AgentStow" }

[artifacts.hello]
kind = "file"
source = "artifacts/hello.txt.tera"
template = true
validate_as = "none"
"#,
        ) {
            return api_error(StatusCode::BAD_REQUEST, error);
        }
        true
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

    // Init implies "select" so the UI can proceed without restart.
    let canonical = match fs_err::canonicalize(&workspace_root) {
        Ok(path) => path,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };
    {
        let mut guard = st.workspace_root.write().await;
        *guard = Some(canonical.clone());
    }
    {
        let mut guard = st.watch.write().await;
        *guard = crate::watch::WatchStatusHandle::start(canonical.clone());
    }

    Json(WorkspaceInitResponse {
        workspace_root: agentstow_core::normalize_for_display(&canonical),
        manifest_path: agentstow_core::normalize_for_display(&manifest_path),
        created,
    })
    .into_response()
}

async fn api_manifest(State(st): State<Arc<ServerState>>) -> Response {
    let queries = match queries_from_state(&st).await {
        Ok(queries) => queries,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    match queries.manifest_overview() {
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

    let queries = match queries_from_state(&st).await {
        Ok(queries) => queries,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    handle_result(queries.render_preview(&artifact_id, &profile))
}

async fn api_links(State(st): State<Arc<ServerState>>) -> Response {
    let queries = match queries_from_state(&st).await {
        Ok(queries) => queries,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    match queries.link_records() {
        Ok(records) => Json(records).into_response(),
        Err(error) => api_error(StatusCode::INTERNAL_SERVER_ERROR, error),
    }
}

async fn api_link_status(State(st): State<Arc<ServerState>>) -> Response {
    let queries = match queries_from_state(&st).await {
        Ok(queries) => queries,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    match queries.link_status() {
        Ok(status) => Json(status).into_response(),
        Err(error) => api_error(StatusCode::INTERNAL_SERVER_ERROR, error),
    }
}

async fn api_watch_status(State(st): State<Arc<ServerState>>) -> Response {
    let snapshot = st.watch.read().await.snapshot();
    Json(crate::services::watch_status_response(snapshot)).into_response()
}

async fn api_workspace_summary(State(st): State<Arc<ServerState>>) -> Response {
    let queries = match queries_from_state(&st).await {
        Ok(queries) => queries,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    handle_result(queries.workspace_summary())
}

async fn api_artifact_detail(
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

async fn api_artifact_source(
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

async fn api_artifact_source_update(
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

    handle_result(queries.update_artifact_source(&artifact_id, &req.content))
}

async fn api_profile_detail(
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

    let queries = match queries_from_state(&st).await {
        Ok(queries) => queries,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    handle_result(queries.impact_analysis(artifact.as_ref(), profile.as_ref()))
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

async fn queries_from_state(st: &Arc<ServerState>) -> Result<WorkspaceQueryService, AgentStowError> {
    let workspace_root = st.workspace_root.read().await.clone().ok_or_else(|| {
        AgentStowError::InvalidArgs {
            message: "workspace 未选择，请先通过 /api/workspace 设置或使用 CLI --workspace".into(),
        }
    })?;
    Ok(WorkspaceQueryService::new(workspace_root))
}
