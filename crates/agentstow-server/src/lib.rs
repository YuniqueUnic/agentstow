use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use agentstow_core::{
    AgentStowDirs, AgentStowError, ArtifactId, InstallMethod, ProfileName, Result,
    normalize_for_display,
};
use agentstow_manifest::Manifest;
use agentstow_render::Renderer;
use agentstow_state::StateDb;
use agentstow_validate::Validator;
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
    let ServerConfig {
        workspace_root,
        addr,
    } = cfg;
    let app = build_app(workspace_root);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(AgentStowError::from)?;
    axum::serve(listener, app)
        .await
        .map_err(|e| AgentStowError::Other(anyhow::anyhow!("axum serve 失败: {e}")))?;
    Ok(())
}

pub fn build_app(workspace_root: PathBuf) -> Router {
    let state = ServerState { workspace_root };
    app_router(Arc::new(state))
}

fn app_router(state: Arc<ServerState>) -> Router {
    Router::new()
        .route("/", get(ui_index))
        .route("/api/health", get(api_health))
        .route("/api/manifest", get(api_manifest))
        .route("/api/render", get(api_render))
        .route("/api/links", get(api_links))
        .route("/api/link-status", get(api_link_status))
        .with_state(state)
}

async fn ui_index() -> Html<&'static str> {
    Html(include_str!("ui/index.html"))
}

#[derive(Debug, Serialize)]
struct ApiError {
    message: String,
}

fn api_error(status: StatusCode, message: impl ToString) -> impl IntoResponse {
    (
        status,
        Json(ApiError {
            message: message.to_string(),
        }),
    )
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
        Err(e) => api_error(StatusCode::BAD_REQUEST, e).into_response(),
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
        Err(e) => return api_error(StatusCode::BAD_REQUEST, e).into_response(),
    };
    let artifact_id = match ArtifactId::parse(q.artifact) {
        Ok(a) => a,
        Err(e) => return api_error(StatusCode::BAD_REQUEST, e).into_response(),
    };
    let profile = match ProfileName::parse(q.profile) {
        Ok(p) => p,
        Err(e) => return api_error(StatusCode::BAD_REQUEST, e).into_response(),
    };
    let out = match Renderer::render_file(&manifest, &artifact_id, &profile) {
        Ok(o) => o,
        Err(e) => return api_error(StatusCode::BAD_REQUEST, e).into_response(),
    };

    let artifact_def = match manifest.artifacts.get(&artifact_id) {
        Some(def) => def,
        None => {
            return api_error(StatusCode::BAD_REQUEST, "artifact 不存在").into_response();
        }
    };
    if let Err(e) = Validator::validate_rendered_file(artifact_def, &out.bytes) {
        return api_error(StatusCode::BAD_REQUEST, e).into_response();
    }

    let s = String::from_utf8_lossy(&out.bytes).to_string();
    Json(serde_json::json!({ "text": s })).into_response()
}

#[instrument(skip_all)]
async fn api_links(State(st): State<Arc<ServerState>>) -> impl IntoResponse {
    let dirs = match AgentStowDirs::from_env() {
        Ok(d) => d,
        Err(e) => return api_error(StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    };
    let db = match StateDb::open(&dirs) {
        Ok(db) => db,
        Err(e) => return api_error(StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    };
    let records = match db.list_link_instances(&st.workspace_root) {
        Ok(r) => r,
        Err(e) => return api_error(StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    };

    Json(records).into_response()
}

#[derive(Debug, Serialize)]
struct LinkStatusRespItem {
    artifact_id: String,
    profile: String,
    target_path: String,
    method: InstallMethod,
    ok: bool,
    message: String,
}

#[instrument(skip_all)]
async fn api_link_status(State(st): State<Arc<ServerState>>) -> impl IntoResponse {
    let manifest = match Manifest::load_from_dir(&st.workspace_root) {
        Ok(m) => m,
        Err(e) => return api_error(StatusCode::BAD_REQUEST, e).into_response(),
    };

    let dirs = match AgentStowDirs::from_env() {
        Ok(d) => d,
        Err(e) => return api_error(StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    };
    let db = match StateDb::open(&dirs) {
        Ok(db) => db,
        Err(e) => return api_error(StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    };
    let records = match db.list_link_instances(&st.workspace_root) {
        Ok(r) => r,
        Err(e) => return api_error(StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    };

    let mut out = Vec::new();
    for rec in records {
        let artifact_def = match manifest.artifacts.get(&rec.artifact_id) {
            Some(a) => a,
            None => {
                out.push(LinkStatusRespItem {
                    artifact_id: rec.artifact_id.as_str().to_string(),
                    profile: rec.profile.as_str().to_string(),
                    target_path: normalize_for_display(&rec.target_path),
                    method: rec.method,
                    ok: false,
                    message: "artifact_missing".to_string(),
                });
                continue;
            }
        };

        let ok = match rec.method {
            InstallMethod::Symlink => match rec.rendered_path.as_ref() {
                Some(src) => {
                    agentstow_linker::check_symlink(&rec.target_path, src).unwrap_or(false)
                }
                None => false,
            },
            InstallMethod::Junction => match rec.rendered_path.as_ref() {
                Some(src) => {
                    agentstow_linker::check_junction(&rec.target_path, src).unwrap_or(false)
                }
                None => false,
            },
            InstallMethod::Copy => match artifact_def.kind {
                agentstow_core::ArtifactKind::File => {
                    if !rec.target_path.is_file() {
                        false
                    } else {
                        let existing =
                            match fs_err::read(&rec.target_path).map_err(AgentStowError::from) {
                                Ok(b) => b,
                                Err(_) => {
                                    out.push(LinkStatusRespItem {
                                        artifact_id: rec.artifact_id.as_str().to_string(),
                                        profile: rec.profile.as_str().to_string(),
                                        target_path: normalize_for_display(&rec.target_path),
                                        method: rec.method,
                                        ok: false,
                                        message: "read_failed".to_string(),
                                    });
                                    continue;
                                }
                            };
                        let desired = match Renderer::render_file(
                            &manifest,
                            &rec.artifact_id,
                            &rec.profile,
                        ) {
                            Ok(r) => r.bytes,
                            Err(_) => {
                                out.push(LinkStatusRespItem {
                                    artifact_id: rec.artifact_id.as_str().to_string(),
                                    profile: rec.profile.as_str().to_string(),
                                    target_path: normalize_for_display(&rec.target_path),
                                    method: rec.method,
                                    ok: false,
                                    message: "render_failed".to_string(),
                                });
                                continue;
                            }
                        };
                        existing == desired
                    }
                }
                agentstow_core::ArtifactKind::Dir => {
                    let desired_source = artifact_def.source_path(&manifest.workspace_root);
                    agentstow_linker::check_copy_dir(&rec.target_path, &desired_source)
                        .unwrap_or(false)
                }
            },
        };

        out.push(LinkStatusRespItem {
            artifact_id: rec.artifact_id.as_str().to_string(),
            profile: rec.profile.as_str().to_string(),
            target_path: normalize_for_display(&rec.target_path),
            method: rec.method,
            ok,
            message: if ok { "healthy" } else { "unhealthy" }.to_string(),
        });
    }

    Json(out).into_response()
}

#[cfg(test)]
mod tests;
