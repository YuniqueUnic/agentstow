use std::env;
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
use axum::extract::{Query, Request, State};
use axum::http::{Method, StatusCode, header};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::get;
use serde::Deserialize;
use tokio::fs;
use tower_http::services::ServeDir;
use tracing::instrument;

const WEB_DIST_ENV: &str = "AGENTSTOW_WEB_DIST";

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub workspace_root: PathBuf,
    pub addr: SocketAddr,
}

#[derive(Debug, Clone)]
struct WebUiAssets {
    dist_dir: PathBuf,
    index_file: PathBuf,
}

#[derive(Clone)]
struct ServerState {
    workspace_root: PathBuf,
    web_ui: Option<WebUiAssets>,
    web_ui_lookup_paths: Vec<PathBuf>,
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
    let (web_ui, web_ui_lookup_paths) = detect_web_ui_assets(&workspace_root);
    let state = ServerState {
        workspace_root,
        web_ui,
        web_ui_lookup_paths,
    };
    app_router(Arc::new(state))
}

fn app_router(state: Arc<ServerState>) -> Router {
    Router::new()
        .route("/api/health", get(api_health))
        .route("/api/manifest", get(api_manifest))
        .route("/api/render", get(api_render))
        .route("/api/links", get(api_links))
        .route("/api/link-status", get(api_link_status))
        .fallback(ui_fallback)
        .with_state(state)
}

#[instrument(skip_all)]
async fn ui_fallback(State(st): State<Arc<ServerState>>, request: Request) -> Response {
    let Some(web_ui) = st.web_ui.as_ref() else {
        return ui_unavailable_response(&st.web_ui_lookup_paths);
    };

    let method = request.method().clone();
    let path = request.uri().path().to_string();

    let mut static_files = ServeDir::new(web_ui.dist_dir.clone());
    match static_files.try_call(request).await {
        Ok(response) if response.status() != StatusCode::NOT_FOUND => response.into_response(),
        Ok(response) if should_serve_spa_index(&path) => serve_index_html(&method, &web_ui.index_file).await,
        Ok(response) => response.into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            format!("静态资源服务失败: {err}"),
        )
            .into_response(),
    }
}

fn api_error(status: StatusCode, message: impl ToString) -> impl IntoResponse {
    (
        status,
        Json(ApiError {
            message: message.to_string(),
        }),
    )
}

async fn api_health() -> Json<HealthResponse> {
    Json(HealthResponse { ok: true })
}

#[instrument(skip_all)]
async fn api_manifest(State(st): State<Arc<ServerState>>) -> impl IntoResponse {
    match Manifest::load_from_dir(&st.workspace_root) {
        Ok(manifest) => Json(map_manifest_response(&manifest)).into_response(),
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

    Json(RenderResponse {
        text: String::from_utf8_lossy(&out.bytes).to_string(),
    })
    .into_response()
}

#[instrument(skip_all)]
async fn api_links(State(st): State<Arc<ServerState>>) -> impl IntoResponse {
    match list_link_records(&st.workspace_root) {
        Ok(records) => Json(records.into_iter().map(map_link_record).collect::<Vec<_>>()).into_response(),
        Err(e) => api_error(StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

#[instrument(skip_all)]
async fn api_link_status(State(st): State<Arc<ServerState>>) -> impl IntoResponse {
    let manifest = match Manifest::load_from_dir(&st.workspace_root) {
        Ok(m) => m,
        Err(e) => return api_error(StatusCode::BAD_REQUEST, e).into_response(),
    };
    let records = match list_link_records(&st.workspace_root) {
        Ok(r) => r,
        Err(e) => return api_error(StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    };

    let mut out = Vec::with_capacity(records.len());
    for rec in records {
        let artifact_def = match manifest.artifacts.get(&rec.artifact_id) {
            Some(artifact) => artifact,
            None => {
                out.push(LinkStatusResponseItem {
                    artifact_id: rec.artifact_id.as_str().to_string(),
                    profile: rec.profile.as_str().to_string(),
                    target_path: normalize_for_display(&rec.target_path),
                    method: map_install_method(rec.method),
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
                ArtifactKind::File => {
                    if !rec.target_path.is_file() {
                        false
                    } else {
                        let existing =
                            match fs_err::read(&rec.target_path).map_err(AgentStowError::from) {
                                Ok(bytes) => bytes,
                                Err(_) => {
                                    out.push(LinkStatusResponseItem {
                                        artifact_id: rec.artifact_id.as_str().to_string(),
                                        profile: rec.profile.as_str().to_string(),
                                        target_path: normalize_for_display(&rec.target_path),
                                        method: map_install_method(rec.method),
                                        ok: false,
                                        message: "read_failed".to_string(),
                                    });
                                    continue;
                                }
                            };
                        let desired =
                            match Renderer::render_file(&manifest, &rec.artifact_id, &rec.profile) {
                                Ok(rendered) => rendered.bytes,
                                Err(_) => {
                                    out.push(LinkStatusResponseItem {
                                        artifact_id: rec.artifact_id.as_str().to_string(),
                                        profile: rec.profile.as_str().to_string(),
                                        target_path: normalize_for_display(&rec.target_path),
                                        method: map_install_method(rec.method),
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
                    agentstow_linker::check_copy_dir(&rec.target_path, &desired_source)
                        .unwrap_or(false)
                }
            },
        };

        out.push(LinkStatusResponseItem {
            artifact_id: rec.artifact_id.as_str().to_string(),
            profile: rec.profile.as_str().to_string(),
            target_path: normalize_for_display(&rec.target_path),
            method: map_install_method(rec.method),
            ok,
            message: if ok { "healthy" } else { "unhealthy" }.to_string(),
        });
    }

    Json(out).into_response()
}

fn map_manifest_response(manifest: &Manifest) -> ManifestResponse {
    let mut profiles = manifest
        .profiles
        .keys()
        .map(|profile| profile.as_str().to_string())
        .collect::<Vec<_>>();
    let mut artifacts = manifest
        .artifacts
        .keys()
        .map(|artifact| artifact.as_str().to_string())
        .collect::<Vec<_>>();
    let mut targets = manifest
        .targets
        .keys()
        .map(|target| target.as_str().to_string())
        .collect::<Vec<_>>();

    profiles.sort();
    artifacts.sort();
    targets.sort();

    ManifestResponse {
        workspace_root: normalize_for_display(&manifest.workspace_root),
        profiles,
        artifacts,
        targets,
    }
}

fn list_link_records(workspace_root: &Path) -> Result<Vec<LinkInstanceRecord>> {
    let dirs = AgentStowDirs::from_env()?;
    let db = StateDb::open(&dirs)?;
    let mut records = db.list_link_instances(workspace_root)?;
    records.sort_by(|left, right| {
        (
            left.artifact_id.as_str(),
            left.profile.as_str(),
            left.target_path.as_os_str(),
        )
            .cmp(&(
                right.artifact_id.as_str(),
                right.profile.as_str(),
                right.target_path.as_os_str(),
            ))
    });
    Ok(records)
}

fn map_link_record(rec: LinkInstanceRecord) -> LinkRecordResponse {
    LinkRecordResponse {
        artifact_id: rec.artifact_id.as_str().to_string(),
        profile: rec.profile.as_str().to_string(),
        target_path: normalize_for_display(&rec.target_path),
        method: map_install_method(rec.method),
        rendered_path: rec
            .rendered_path
            .as_ref()
            .map(|path| normalize_for_display(path)),
        blake3: rec.blake3,
        updated_at: rec.updated_at.to_string(),
    }
}

fn map_install_method(method: InstallMethod) -> InstallMethodResponse {
    match method {
        InstallMethod::Symlink => InstallMethodResponse::Symlink,
        InstallMethod::Junction => InstallMethodResponse::Junction,
        InstallMethod::Copy => InstallMethodResponse::Copy,
    }
}

fn detect_web_ui_assets(workspace_root: &Path) -> (Option<WebUiAssets>, Vec<PathBuf>) {
    if let Some(path) = env::var_os(WEB_DIST_ENV) {
        let candidate = PathBuf::from(path);
        let index_file = candidate.join("index.html");
        let web_ui = if candidate.is_dir() && index_file.is_file() {
            Some(WebUiAssets {
                dist_dir: candidate.clone(),
                index_file,
            })
        } else {
            None
        };
        return (web_ui, vec![candidate]);
    }

    let mut candidates = Vec::new();
    candidates.push(workspace_root.join("web/dist"));
    candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../web/dist"));
    if let Ok(current_dir) = env::current_dir() {
        candidates.push(current_dir.join("web/dist"));
    }

    let mut lookup_paths = Vec::new();
    for candidate in candidates {
        if lookup_paths.iter().all(|existing| existing != &candidate) {
            lookup_paths.push(candidate.clone());
        }

        let index_file = candidate.join("index.html");
        if candidate.is_dir() && index_file.is_file() {
            return (
                Some(WebUiAssets {
                    dist_dir: candidate,
                    index_file,
                }),
                lookup_paths,
            );
        }
    }

    (None, lookup_paths)
}

fn should_serve_spa_index(path: &str) -> bool {
    if path == "/" {
        return true;
    }

    let trimmed = path.trim_start_matches('/').trim_end_matches('/');
    if trimmed.is_empty() {
        return true;
    }

    Path::new(trimmed).extension().is_none()
}

async fn serve_index_html(method: &Method, index_file: &Path) -> Response {
    match fs::read(index_file).await {
        Ok(bytes) => {
            let body = if *method == Method::HEAD {
                Vec::new()
            } else {
                bytes
            };
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                body,
            )
                .into_response()
        }
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            format!("读取前端入口失败: {err}"),
        )
            .into_response(),
    }
}

fn ui_unavailable_response(lookup_paths: &[PathBuf]) -> Response {
    let mut details = String::new();
    for path in lookup_paths {
        details.push_str(&format!(
            "<li><code>{}</code></li>",
            normalize_for_display(path)
        ));
    }

    (
        StatusCode::SERVICE_UNAVAILABLE,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        format!(
            concat!(
                "<!doctype html><html lang=\"zh-CN\"><head><meta charset=\"utf-8\">",
                "<title>AgentStow Web UI Unavailable</title></head><body>",
                "<main><h1>AgentStow Web UI 未构建</h1>",
                "<p>未找到可托管的 <code>web/dist</code>，请先在仓库根目录执行 <code>bun install && bun run build --cwd web</code>。</p>",
                "<p>已检查以下路径：</p><ul>{}</ul>",
                "</main></body></html>"
            ),
            details
        ),
    )
        .into_response()
}

#[cfg(test)]
mod tests;
