use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use agentstow_core::{
    AgentStowDirs, AgentStowError, ArtifactId, ArtifactKind, InstallMethod, ProfileName, Result,
    normalize_for_display,
};
use agentstow_manifest::Manifest;
use agentstow_render::Renderer;
use agentstow_state::{LinkInstanceRecord, StateDb};
use agentstow_validate::Validator;
use agentstow_web_types::{
    ApiError, HealthResponse, InstallMethodResponse, LinkRecordResponse, LinkStatusResponseItem,
    ManifestResponse, RenderResponse,
};
use axum::Router;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Json, Response};
use axum::routing::{any, get};
use serde::Deserialize;
use time::format_description::well_known::Rfc3339;
use tokio::fs;
use tower_http::services::ServeDir;
use tracing::instrument;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub workspace_root: PathBuf,
    pub addr: SocketAddr,
}

#[derive(Clone)]
struct ServerState {
    workspace_root: PathBuf,
    ui_dist_dir: PathBuf,
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
    let ui_dist_dir = default_ui_dist_dir(&workspace_root);
    build_app_with_ui_dist(workspace_root, ui_dist_dir)
}

fn build_app_with_ui_dist(workspace_root: PathBuf, ui_dist_dir: PathBuf) -> Router {
    let assets_dir = ui_dist_dir.join("assets");
    let state = ServerState {
        workspace_root,
        ui_dist_dir,
    };

    Router::new()
        .route("/api/health", get(api_health))
        .route("/api/manifest", get(api_manifest))
        .route("/api/render", get(api_render))
        .route("/api/links", get(api_links))
        .route("/api/link-status", get(api_link_status))
        .route("/api/{*path}", any(api_not_found))
        .nest_service("/assets", ServeDir::new(assets_dir))
        .route("/", get(ui_shell))
        .route("/{*path}", get(ui_shell))
        .with_state(Arc::new(state))
}

fn default_ui_dist_dir(workspace_root: &Path) -> PathBuf {
    workspace_root.join("web/dist")
}

async fn ui_shell(State(st): State<Arc<ServerState>>) -> Response {
    serve_ui_shell(&st.ui_dist_dir).await
}

async fn serve_ui_shell(ui_dist_dir: &Path) -> Response {
    let index_path = ui_dist_dir.join("index.html");
    match fs::read_to_string(&index_path).await {
        Ok(html) => Html(html).into_response(),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Html(ui_dist_missing_page(&index_path)),
        )
            .into_response(),
    }
}

fn ui_dist_missing_page(index_path: &Path) -> String {
    format!(
        "<!doctype html><html lang=\"zh-CN\"><head><meta charset=\"utf-8\"><title>AgentStow Web Dist Missing</title></head><body><h1>web/dist 尚未构建</h1><p>缺少前端入口文件：<code>{}</code></p><p>请先在仓库根目录构建前端，例如进入 <code>web/</code> 后执行安装与构建命令。</p></body></html>",
        normalize_for_display(index_path)
    )
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

#[instrument(skip_all)]
async fn api_manifest(State(st): State<Arc<ServerState>>) -> Response {
    match Manifest::load_from_dir(&st.workspace_root) {
        Ok(m) => Json(ManifestResponse {
            workspace_root: normalize_for_display(&m.workspace_root),
            profiles: m.profiles.keys().map(|p| p.as_str().to_string()).collect(),
            artifacts: m.artifacts.keys().map(|a| a.as_str().to_string()).collect(),
            targets: m.targets.keys().map(|t| t.as_str().to_string()).collect(),
        })
        .into_response(),
        Err(e) => api_error(StatusCode::BAD_REQUEST, e),
    }
}

#[derive(Debug, Deserialize)]
struct RenderQuery {
    artifact: String,
    profile: String,
}

#[instrument(skip_all)]
async fn api_render(State(st): State<Arc<ServerState>>, Query(q): Query<RenderQuery>) -> Response {
    let manifest = match Manifest::load_from_dir(&st.workspace_root) {
        Ok(m) => m,
        Err(e) => return api_error(StatusCode::BAD_REQUEST, e),
    };
    let artifact_id = match ArtifactId::parse(q.artifact) {
        Ok(a) => a,
        Err(e) => return api_error(StatusCode::BAD_REQUEST, e),
    };
    let profile = match ProfileName::parse(q.profile) {
        Ok(p) => p,
        Err(e) => return api_error(StatusCode::BAD_REQUEST, e),
    };
    let rendered = match Renderer::render_file(&manifest, &artifact_id, &profile) {
        Ok(output) => output,
        Err(e) => return api_error(StatusCode::BAD_REQUEST, e),
    };

    let artifact_def = match manifest.artifacts.get(&artifact_id) {
        Some(def) => def,
        None => return api_error(StatusCode::BAD_REQUEST, "artifact 不存在"),
    };
    if let Err(e) = Validator::validate_rendered_file(artifact_def, &rendered.bytes) {
        return api_error(StatusCode::BAD_REQUEST, e);
    }

    Json(RenderResponse {
        text: String::from_utf8_lossy(&rendered.bytes).to_string(),
    })
    .into_response()
}

#[instrument(skip_all)]
async fn api_links(State(st): State<Arc<ServerState>>) -> Response {
    let db = match open_state_db() {
        Ok(db) => db,
        Err(e) => return api_error(StatusCode::INTERNAL_SERVER_ERROR, e),
    };
    let records = match db.list_link_instances(&st.workspace_root) {
        Ok(records) => records,
        Err(e) => return api_error(StatusCode::INTERNAL_SERVER_ERROR, e),
    };

    Json(
        records
            .into_iter()
            .map(link_record_response)
            .collect::<Vec<_>>(),
    )
    .into_response()
}

#[instrument(skip_all)]
async fn api_link_status(State(st): State<Arc<ServerState>>) -> Response {
    let manifest = match Manifest::load_from_dir(&st.workspace_root) {
        Ok(m) => m,
        Err(e) => return api_error(StatusCode::BAD_REQUEST, e),
    };
    let db = match open_state_db() {
        Ok(db) => db,
        Err(e) => return api_error(StatusCode::INTERNAL_SERVER_ERROR, e),
    };
    let records = match db.list_link_instances(&st.workspace_root) {
        Ok(records) => records,
        Err(e) => return api_error(StatusCode::INTERNAL_SERVER_ERROR, e),
    };

    let mut out = Vec::new();
    for record in records {
        let artifact_def = match manifest.artifacts.get(&record.artifact_id) {
            Some(artifact) => artifact,
            None => {
                out.push(LinkStatusResponseItem {
                    artifact_id: record.artifact_id.as_str().to_string(),
                    profile: record.profile.as_str().to_string(),
                    target_path: normalize_for_display(&record.target_path),
                    method: install_method_response(record.method),
                    ok: false,
                    message: "artifact_missing".to_string(),
                });
                continue;
            }
        };

        let ok = match record.method {
            InstallMethod::Symlink => match record.rendered_path.as_ref() {
                Some(src) => {
                    agentstow_linker::check_symlink(&record.target_path, src).unwrap_or(false)
                }
                None => false,
            },
            InstallMethod::Junction => match record.rendered_path.as_ref() {
                Some(src) => {
                    agentstow_linker::check_junction(&record.target_path, src).unwrap_or(false)
                }
                None => false,
            },
            InstallMethod::Copy => match artifact_def.kind {
                ArtifactKind::File => {
                    if !record.target_path.is_file() {
                        false
                    } else {
                        let existing =
                            match fs_err::read(&record.target_path).map_err(AgentStowError::from) {
                                Ok(bytes) => bytes,
                                Err(_) => {
                                    out.push(LinkStatusResponseItem {
                                        artifact_id: record.artifact_id.as_str().to_string(),
                                        profile: record.profile.as_str().to_string(),
                                        target_path: normalize_for_display(&record.target_path),
                                        method: install_method_response(record.method),
                                        ok: false,
                                        message: "read_failed".to_string(),
                                    });
                                    continue;
                                }
                            };
                        let desired = match Renderer::render_file(
                            &manifest,
                            &record.artifact_id,
                            &record.profile,
                        ) {
                            Ok(rendered) => rendered.bytes,
                            Err(_) => {
                                out.push(LinkStatusResponseItem {
                                    artifact_id: record.artifact_id.as_str().to_string(),
                                    profile: record.profile.as_str().to_string(),
                                    target_path: normalize_for_display(&record.target_path),
                                    method: install_method_response(record.method),
                                    ok: false,
                                    message: "render_failed".to_string(),
                                });
                                continue;
                            }
                        };
                        existing == desired
                    }
                }
                ArtifactKind::Dir => {
                    let desired_source = artifact_def.source_path(&manifest.workspace_root);
                    agentstow_linker::check_copy_dir(&record.target_path, &desired_source)
                        .unwrap_or(false)
                }
            },
        };

        out.push(LinkStatusResponseItem {
            artifact_id: record.artifact_id.as_str().to_string(),
            profile: record.profile.as_str().to_string(),
            target_path: normalize_for_display(&record.target_path),
            method: install_method_response(record.method),
            ok,
            message: if ok { "healthy" } else { "unhealthy" }.to_string(),
        });
    }

    Json(out).into_response()
}

fn open_state_db() -> Result<StateDb> {
    let dirs = AgentStowDirs::from_env()?;
    StateDb::open(&dirs)
}

fn link_record_response(record: LinkInstanceRecord) -> LinkRecordResponse {
    LinkRecordResponse {
        artifact_id: record.artifact_id.as_str().to_string(),
        profile: record.profile.as_str().to_string(),
        target_path: normalize_for_display(&record.target_path),
        method: install_method_response(record.method),
        rendered_path: record
            .rendered_path
            .as_ref()
            .map(|path| normalize_for_display(path)),
        blake3: record.blake3,
        updated_at: record.updated_at.format(&Rfc3339).unwrap_or_default(),
    }
}

fn install_method_response(method: InstallMethod) -> InstallMethodResponse {
    match method {
        InstallMethod::Symlink => InstallMethodResponse::Symlink,
        InstallMethod::Junction => InstallMethodResponse::Junction,
        InstallMethod::Copy => InstallMethodResponse::Copy,
    }
}

#[cfg(test)]
mod tests;
