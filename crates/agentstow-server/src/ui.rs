use std::path::{Path, PathBuf};
use std::sync::Arc;

use agentstow_core::normalize_for_display;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use tokio::fs;

use crate::ServerState;

#[derive(Debug, Clone)]
struct UiDistInputs {
    env_override: Option<PathBuf>,
    current_exe: Option<PathBuf>,
    repo_root: PathBuf,
}

pub(crate) fn default_ui_dist_dir() -> PathBuf {
    resolve_ui_dist_dir(UiDistInputs {
        env_override: std::env::var_os("AGENTSTOW_UI_DIST").map(PathBuf::from),
        current_exe: std::env::current_exe().ok(),
        repo_root: repo_root_from_manifest_dir(),
    })
}

fn resolve_ui_dist_dir(inputs: UiDistInputs) -> PathBuf {
    if let Some(override_dir) = inputs.env_override {
        return override_dir;
    }

    if let Some(exe) = inputs.current_exe {
        if let Some(exe_dir) = exe.parent() {
            // Common dev case: target/debug/agentstow -> <repo>/web/dist.
            for ancestor in exe_dir.ancestors() {
                let candidate = ancestor.join("web/dist");
                if candidate.join("index.html").is_file() {
                    return candidate;
                }
            }
        }
    }

    // Dev fallback: resolve relative to repository root.
    inputs.repo_root.join("web/dist")
}

fn repo_root_from_manifest_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("agentstow-server crate should live under crates/<name>")
        .to_path_buf()
}

#[cfg(test)]
pub(crate) fn resolve_ui_dist_dir_for_test(
    env_override: Option<PathBuf>,
    current_exe: Option<PathBuf>,
    repo_root: PathBuf,
) -> PathBuf {
    resolve_ui_dist_dir(UiDistInputs {
        env_override,
        current_exe,
        repo_root,
    })
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
        "<!doctype html><html lang=\"zh-CN\"><head><meta charset=\"utf-8\"><title>AgentStow Web Dist Missing</title></head><body><h1>web/dist 尚未构建</h1><p>缺少前端入口文件：<code>{}</code></p><p>如果你在源码仓库内运行：进入 <code>web/</code> 后执行 <code>bun install</code> 与 <code>bun run build</code>。</p><p>如果你在任意目录运行已编译的二进制：请设置 <code>AGENTSTOW_UI_DIST</code> 指向包含 <code>index.html</code> 的 dist 目录。</p></body></html>",
        normalize_for_display(index_path)
    )
}
