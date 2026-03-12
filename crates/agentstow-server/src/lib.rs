use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use agentstow_core::{AgentStowDirs, AgentStowError, ArtifactId, ProfileName, Result};
use agentstow_manifest::Manifest;
use agentstow_render::Renderer;
use agentstow_state::StateDb;
use axum::Router;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Json};
use axum::routing::get;
use serde::{Deserialize, Serialize};
use tracing::instrument;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub workspace_root: PathBuf,
    pub addr: SocketAddr,
}

#[derive(Clone)]
struct ServerState {
    workspace_root: PathBuf,
}

pub async fn serve(cfg: ServerConfig) -> Result<()> {
    let state = ServerState {
        workspace_root: cfg.workspace_root.clone(),
    };

    let app = Router::new()
        .route("/", get(ui_index))
        .route("/api/health", get(api_health))
        .route("/api/manifest", get(api_manifest))
        .route("/api/render", get(api_render))
        .route("/api/links", get(api_links))
        .with_state(Arc::new(state));

    let listener = tokio::net::TcpListener::bind(cfg.addr)
        .await
        .map_err(AgentStowError::from)?;
    axum::serve(listener, app)
        .await
        .map_err(|e| AgentStowError::Other(anyhow::anyhow!("axum serve 失败: {e}")))?;
    Ok(())
}

async fn ui_index() -> Html<&'static str> {
    Html(include_str!("ui/index.html"))
}

#[derive(Debug, Serialize)]
struct HealthResp {
    ok: bool,
}

async fn api_health() -> Json<HealthResp> {
    Json(HealthResp { ok: true })
}

#[derive(Debug, Serialize)]
struct ManifestResp {
    workspace_root: String,
    profiles: Vec<String>,
    artifacts: Vec<String>,
    targets: Vec<String>,
}

#[instrument(skip_all)]
async fn api_manifest(State(st): State<Arc<ServerState>>) -> impl IntoResponse {
    match Manifest::load_from_dir(&st.workspace_root) {
        Ok(m) => Json(ManifestResp {
            workspace_root: m.workspace_root.display().to_string(),
            profiles: m.profiles.keys().map(|p| p.as_str().to_string()).collect(),
            artifacts: m.artifacts.keys().map(|a| a.as_str().to_string()).collect(),
            targets: m.targets.keys().map(|t| t.as_str().to_string()).collect(),
        })
        .into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct RenderQuery {
    artifact: String,
    profile: String,
}

#[instrument(skip_all)]
async fn api_render(
    State(st): State<Arc<ServerState>>,
    Query(q): Query<RenderQuery>,
) -> impl IntoResponse {
    let manifest = match Manifest::load_from_dir(&st.workspace_root) {
        Ok(m) => m,
        Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };
    let out = match Renderer::render_file(
        &manifest,
        &ArtifactId::new_unchecked(q.artifact),
        &ProfileName::new_unchecked(q.profile),
    ) {
        Ok(o) => o,
        Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };

    let s = String::from_utf8_lossy(&out.bytes).to_string();
    Json(serde_json::json!({ "text": s })).into_response()
}

#[instrument(skip_all)]
async fn api_links(State(st): State<Arc<ServerState>>) -> impl IntoResponse {
    let dirs = match AgentStowDirs::from_env() {
        Ok(d) => d,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    let db = match StateDb::open(&dirs) {
        Ok(db) => db,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    let records = match db.list_link_instances(&st.workspace_root) {
        Ok(r) => r,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    Json(records).into_response()
}
