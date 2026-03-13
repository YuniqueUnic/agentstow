use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use agentstow_core::{AgentStowError, Result};
use axum::Router;
use axum::routing::get;
use tokio::sync::RwLock;
use tower_http::services::ServeDir;

mod api;
mod services;
mod ui;
mod watch;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub workspace_root: Option<PathBuf>,
    pub addr: SocketAddr,
}

pub(crate) struct ServerState {
    pub(crate) workspace_root: RwLock<Option<PathBuf>>,
    pub(crate) watch: RwLock<watch::WatchStatusHandle>,
    pub(crate) ui_dist_dir: PathBuf,
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
        .map_err(|error| AgentStowError::Other(anyhow::anyhow!("axum serve 失败：{error}")))?;
    Ok(())
}

pub fn build_app(workspace_root: Option<PathBuf>) -> Router {
    let ui_dist_dir = ui::default_ui_dist_dir();
    build_app_with_ui_dist(workspace_root, ui_dist_dir)
}

pub(crate) fn build_app_with_ui_dist(
    workspace_root: Option<PathBuf>,
    ui_dist_dir: PathBuf,
) -> Router {
    let watch = match workspace_root.as_ref() {
        Some(root) => watch::WatchStatusHandle::start(root.clone()),
        None => watch::WatchStatusHandle::manual(
            Vec::new(),
            Some("workspace 未选择（可在 Web UI 中选择或通过 CLI --workspace 指定）".to_string()),
        ),
    };
    build_app_with_ui_dist_and_watch(workspace_root, ui_dist_dir, watch)
}

pub(crate) fn build_app_with_ui_dist_and_watch(
    workspace_root: Option<PathBuf>,
    ui_dist_dir: PathBuf,
    watch: watch::WatchStatusHandle,
) -> Router {
    let assets_dir = ui_dist_dir.join("assets");
    let state = Arc::new(ServerState {
        workspace_root: RwLock::new(workspace_root),
        watch: RwLock::new(watch),
        ui_dist_dir,
    });

    Router::new()
        .merge(api::routes())
        .nest_service("/assets", ServeDir::new(assets_dir))
        .route("/", get(ui::ui_shell))
        .route("/{*path}", get(ui::ui_shell))
        .with_state(state)
}

#[cfg(test)]
pub(crate) use ui::resolve_ui_dist_dir_for_test;
#[cfg(test)]
pub(crate) use ui::ui_dist_missing_page;
#[cfg(test)]
pub(crate) use watch::{
    WatchMode, WatchPlan, WatchStatusHandle, WatchStatusSnapshot, summarize_events,
};

#[cfg(test)]
mod tests;
