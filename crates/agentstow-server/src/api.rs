use std::path::Path;
use std::sync::Arc;

use agentstow_core::{AgentStowError, ArtifactId, ProfileName, normalize_for_display};
use agentstow_web_types::{
    ApiError, ArtifactGitRollbackRequest, ArtifactSourceUpdateRequest, EnvEmitRequest,
    HealthResponse, LinkApplyRequest, LinkPlanRequest, LinkRepairRequest,
    ManifestSourceUpdateRequest, ScriptRunRequest, WorkspaceGitSummaryResponse,
    WorkspaceInitRequest, WorkspaceInitResponse, WorkspaceSelectRequest, WorkspaceSelectResponse,
    WorkspaceStateResponse,
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
        .route(
            "/api/workspace",
            get(api_workspace_state).post(api_workspace_select),
        )
        .route("/api/workspace/init", post(api_workspace_init))
        .route("/api/workspace/git", get(api_workspace_git))
        .route("/api/manifest", get(api_manifest))
        .route(
            "/api/manifest/source",
            get(api_manifest_source).put(api_manifest_source_update),
        )
        .route("/api/render", get(api_render))
        .route("/api/links", get(api_links))
        .route("/api/link-status", get(api_link_status))
        .route("/api/links/plan", post(api_links_plan))
        .route("/api/links/apply", post(api_links_apply))
        .route("/api/links/repair", post(api_links_repair))
        .route("/api/watch-status", get(api_watch_status))
        .route("/api/workspace-summary", get(api_workspace_summary))
        .route("/api/artifacts/{artifact}", get(api_artifact_detail))
        .route(
            "/api/artifacts/{artifact}/source",
            get(api_artifact_source).put(api_artifact_source_update),
        )
        .route(
            "/api/artifacts/{artifact}/git/history",
            get(api_artifact_git_history),
        )
        .route(
            "/api/artifacts/{artifact}/git/compare",
            get(api_artifact_git_compare),
        )
        .route(
            "/api/artifacts/{artifact}/git/rollback",
            post(api_artifact_git_rollback),
        )
        .route("/api/mcp/{server}/validate", post(api_mcp_validate))
        .route("/api/mcp/{server}/render", get(api_mcp_render))
        .route("/api/mcp/{server}/test", post(api_mcp_test))
        .route("/api/env/emit", post(api_env_emit))
        .route("/api/scripts/{script}/run", post(api_script_run))
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

#[derive(Debug, Deserialize)]
struct ArtifactGitHistoryQuery {
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct ArtifactGitCompareQuery {
    base: String,
    head: Option<String>,
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
    let manifest_present = workspace_root.as_ref().is_some_and(|root| {
        root.join(agentstow_manifest::DEFAULT_MANIFEST_FILE)
            .is_file()
    });

    Json(WorkspaceStateResponse {
        workspace_root: workspace_root
            .as_deref()
            .map(agentstow_core::normalize_for_display),
        manifest_present,
    })
    .into_response()
}

async fn api_workspace_git(State(st): State<Arc<ServerState>>) -> Response {
    let workspace_root = st.workspace_root.read().await.clone();
    let Some(workspace_root) = workspace_root else {
        return Json::<Option<WorkspaceGitSummaryResponse>>(None).into_response();
    };

    let queries = WorkspaceQueryService::new(workspace_root);
    handle_result(queries.workspace_git().await)
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

    // Init implies "select" so the UI can proceed without restart.
    let canonical = match fs_err::canonicalize(&workspace_root) {
        Ok(path) => path,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };
    let manifest_path = canonical.join(agentstow_manifest::DEFAULT_MANIFEST_FILE);
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

async fn api_manifest_source(State(st): State<Arc<ServerState>>) -> Response {
    let queries = match queries_from_state(&st).await {
        Ok(queries) => queries,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    handle_result(queries.manifest_source())
}

async fn api_manifest_source_update(
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

async fn api_links_plan(
    State(st): State<Arc<ServerState>>,
    Json(req): Json<LinkPlanRequest>,
) -> Response {
    let queries = match queries_from_state(&st).await {
        Ok(queries) => queries,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    handle_result(queries.link_plan(req))
}

async fn api_links_apply(
    State(st): State<Arc<ServerState>>,
    Json(req): Json<LinkApplyRequest>,
) -> Response {
    let queries = match queries_from_state(&st).await {
        Ok(queries) => queries,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    handle_result(queries.link_apply(req))
}

async fn api_links_repair(
    State(st): State<Arc<ServerState>>,
    Json(req): Json<LinkRepairRequest>,
) -> Response {
    let queries = match queries_from_state(&st).await {
        Ok(queries) => queries,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    handle_result(queries.link_repair(req))
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

async fn api_artifact_git_history(
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

async fn api_artifact_git_compare(
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

async fn api_artifact_git_rollback(
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

async fn api_mcp_validate(
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

async fn api_mcp_render(
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

async fn api_mcp_test(
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

async fn api_env_emit(
    State(st): State<Arc<ServerState>>,
    Json(req): Json<EnvEmitRequest>,
) -> Response {
    if req.env_set_id.trim().is_empty() {
        return api_error(StatusCode::BAD_REQUEST, "env_set_id 不能为空");
    }

    let queries = match queries_from_state(&st).await {
        Ok(queries) => queries,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, error),
    };

    handle_result(queries.env_emit(&req.env_set_id, req.shell))
}

async fn api_script_run(
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

async fn queries_from_state(
    st: &Arc<ServerState>,
) -> Result<WorkspaceQueryService, AgentStowError> {
    let workspace_root = selected_workspace_root(st).await?;
    Ok(WorkspaceQueryService::new(workspace_root))
}

async fn selected_workspace_root(
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

async fn record_watch_change(st: &Arc<ServerState>, summary: String) {
    let watch = st.watch.read().await.clone();
    watch.record_change(summary);
}

fn watch_change_summary(workspace_root: &Path, source_path: &str, action: &str) -> String {
    format!(
        "{action} {}",
        workspace_relative_display(workspace_root, source_path)
    )
}

fn workspace_relative_display(workspace_root: &Path, source_path: &str) -> String {
    let source_path = Path::new(source_path);
    match source_path.strip_prefix(workspace_root) {
        Ok(relative) => normalize_for_display(relative),
        Err(_) => source_path.display().to_string(),
    }
}
