use std::sync::Arc;

use axum::Router;
use axum::response::Response;
use axum::routing::{any, get, post, put};

use crate::ServerState;

mod artifacts;
mod helpers;
mod links;
mod read_models;
mod workspace;

pub(crate) fn routes() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/api/health", get(workspace::api_health))
        .route(
            "/api/workspace",
            get(workspace::api_workspace_state).post(workspace::api_workspace_select),
        )
        .route("/api/workspace/probe", post(workspace::api_workspace_probe))
        .route("/api/workspace/pick", post(workspace::api_workspace_pick))
        .route("/api/workspace/init", post(workspace::api_workspace_init))
        .route("/api/workspace/git", get(workspace::api_workspace_git))
        .route("/api/manifest", get(artifacts::api_manifest))
        .route(
            "/api/manifest/source",
            get(artifacts::api_manifest_source).put(artifacts::api_manifest_source_update),
        )
        .route("/api/render", get(artifacts::api_render))
        .route("/api/links", get(links::api_links))
        .route("/api/link-status", get(links::api_link_status))
        .route("/api/links/plan", post(links::api_links_plan))
        .route("/api/links/apply", post(links::api_links_apply))
        .route("/api/links/repair", post(links::api_links_repair))
        .route("/api/watch-status", get(workspace::api_watch_status))
        .route(
            "/api/workspace-summary",
            get(read_models::api_workspace_summary),
        )
        .route(
            "/api/artifacts/{artifact}",
            get(artifacts::api_artifact_detail),
        )
        .route(
            "/api/artifacts/{artifact}/source",
            get(artifacts::api_artifact_source).put(artifacts::api_artifact_source_update),
        )
        .route(
            "/api/artifacts/{artifact}/git/history",
            get(artifacts::api_artifact_git_history),
        )
        .route(
            "/api/artifacts/{artifact}/git/compare",
            get(artifacts::api_artifact_git_compare),
        )
        .route(
            "/api/artifacts/{artifact}/git/rollback",
            post(artifacts::api_artifact_git_rollback),
        )
        .route(
            "/api/mcp/{server}/validate",
            post(read_models::api_mcp_validate),
        )
        .route("/api/mcp/{server}/render", get(read_models::api_mcp_render))
        .route("/api/mcp/{server}/test", post(read_models::api_mcp_test))
        .route("/api/env/emit", post(read_models::api_env_emit))
        .route(
            "/api/scripts/{script}/run",
            post(read_models::api_script_run),
        )
        .route(
            "/api/profiles/{profile}",
            get(read_models::api_profile_detail),
        )
        .route(
            "/api/profiles/{profile}/vars",
            put(read_models::api_profile_vars_update),
        )
        .route("/api/impact", get(read_models::api_impact))
        .route("/api/{*path}", any(api_not_found))
}

async fn api_not_found() -> Response {
    helpers::api_error(axum::http::StatusCode::NOT_FOUND, "api 路由不存在")
}
