use std::path::{Path, PathBuf};
use std::sync::Arc;

use agentstow_core::normalize_for_display;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use tokio::fs;

use crate::ServerState;

pub(crate) fn default_ui_dist_dir(workspace_root: &Path) -> PathBuf {
    workspace_root.join("web/dist")
}

pub(crate) async fn ui_shell(State(st): State<Arc<ServerState>>) -> Response {
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

pub(crate) fn ui_dist_missing_page(index_path: &Path) -> String {
    format!(
        "<!doctype html><html lang=\"zh-CN\"><head><meta charset=\"utf-8\"><title>AgentStow Web Dist Missing</title></head><body><h1>web/dist 尚未构建</h1><p>缺少前端入口文件：<code>{}</code></p><p>请先在仓库根目录构建前端，例如进入 <code>web/</code> 后执行安装与构建命令。</p></body></html>",
        normalize_for_display(index_path)
    )
}
