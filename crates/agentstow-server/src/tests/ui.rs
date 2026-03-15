use assert_fs::prelude::*;
use axum::http::StatusCode;
use pretty_assertions::assert_eq;

use crate::{resolve_ui_dist_dir_for_test, ui_dist_missing_page};

use super::fixtures::{test_server, write_minimal_workspace, write_ui_dist};

#[tokio::test]
async fn ui_root_should_serve_built_index_html() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_minimal_workspace(&temp);
    let server = test_server(&temp, write_ui_dist(&temp));

    let resp = server.get("/").await;

    resp.assert_status_ok();
    resp.assert_text_contains("agentstow-web");
}

#[tokio::test]
async fn ui_non_api_path_should_fallback_to_index_html() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_minimal_workspace(&temp);
    let server = test_server(&temp, write_ui_dist(&temp));

    let resp = server.get("/links/overview").await;

    resp.assert_status_ok();
    resp.assert_text_contains("agentstow-web");
}

#[tokio::test]
async fn ui_assets_should_serve_built_static_files() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_minimal_workspace(&temp);
    let server = test_server(&temp, write_ui_dist(&temp));

    let resp = server.get("/assets/app.js").await;

    resp.assert_status_ok();
    resp.assert_text_contains("agentstow-web");
}

#[tokio::test]
async fn ui_should_return_helpful_message_when_dist_is_missing() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_minimal_workspace(&temp);
    let missing_dist = temp.child("missing-dist").path().to_path_buf();
    let server = test_server(&temp, missing_dist.clone());

    let resp = server.get("/").await;

    resp.assert_status(StatusCode::SERVICE_UNAVAILABLE);
    resp.assert_text(ui_dist_missing_page(&missing_dist.join("index.html")));
}

#[test]
fn ui_dist_resolver_should_honor_env_override() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("override-dist/index.html")
        .write_str("<!doctype html>")
        .unwrap();
    let override_dir = temp.child("override-dist").path().to_path_buf();

    let resolved = resolve_ui_dist_dir_for_test(
        Some(override_dir.clone()),
        Some(
            temp.child("repo/target/debug/agentstow")
                .path()
                .to_path_buf(),
        ),
        temp.child("repo").path().to_path_buf(),
    );

    assert_eq!(resolved, override_dir);
}

#[test]
fn ui_dist_resolver_should_find_web_dist_from_current_exe_ancestors() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("repo/web/dist/index.html")
        .write_str("<!doctype html>")
        .unwrap();

    let exe_path = temp
        .child("repo/target/debug/agentstow")
        .path()
        .to_path_buf();

    let resolved = resolve_ui_dist_dir_for_test(
        None,
        Some(exe_path),
        temp.child("repo").path().to_path_buf(),
    );

    assert_eq!(resolved, temp.child("repo/web/dist").path().to_path_buf());
}

#[test]
fn ui_dist_resolver_should_fallback_to_repo_root_web_dist() {
    let temp = assert_fs::TempDir::new().unwrap();
    let repo_root = temp.child("repo");
    repo_root.create_dir_all().unwrap();

    let resolved = resolve_ui_dist_dir_for_test(
        None,
        Some(temp.child("bin/agentstow").path().to_path_buf()),
        repo_root.path().to_path_buf(),
    );

    assert_eq!(resolved, repo_root.child("web/dist").path().to_path_buf());
}
