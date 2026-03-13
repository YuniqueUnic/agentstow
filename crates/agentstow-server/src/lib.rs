use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use agentstow_core::{AgentStowError, Result};
use axum::routing::get;
use axum::Router;
use tower_http::services::ServeDir;

mod api;
mod services;
mod ui;

use services::WorkspaceQueryService;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub workspace_root: PathBuf,
    pub addr: SocketAddr,
}

#[derive(Clone)]
pub(crate) struct ServerState {
    pub(crate) queries: WorkspaceQueryService,
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
        .map_err(|error| AgentStowError::Other(anyhow::anyhow!("axum serve 失败: {error}")))?;
    Ok(())
}

pub fn build_app(workspace_root: PathBuf) -> Router {
    let ui_dist_dir = ui::default_ui_dist_dir(&workspace_root);
    build_app_with_ui_dist(workspace_root, ui_dist_dir)
}

pub(crate) fn build_app_with_ui_dist(workspace_root: PathBuf, ui_dist_dir: PathBuf) -> Router {
    let assets_dir = ui_dist_dir.join("assets");
    let state = Arc::new(ServerState {
        queries: WorkspaceQueryService::new(workspace_root),
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
pub(crate) use ui::ui_dist_missing_page;

#[cfg(test)]
mod tests;
