use assert_fs::prelude::*;
use pretty_assertions::assert_eq;

use super::fixtures::{test_server, write_minimal_workspace};

#[tokio::test]
async fn api_render_should_validate_rendered_output() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("artifacts").create_dir_all().unwrap();
    temp.child("artifacts/bad.json.tera")
        .write_str("{ invalid: {{ value }} }")
        .unwrap();
    temp.child("agentstow.toml")
        .write_str(
            r#"
[profiles.base]
vars = { value = 1 }

[artifacts.bad]
kind = "file"
source = "artifacts/bad.json.tera"
template = true
validate_as = "json"
"#,
        )
        .unwrap();

    let server = test_server(&temp, temp.child("missing-dist").path().to_path_buf());
    let resp = server
        .get("/api/render")
        .add_query_param("artifact", "bad")
        .add_query_param("profile", "base")
        .await;

    resp.assert_status_bad_request();
    let body: serde_json::Value = resp.json();
    assert!(body["message"].as_str().unwrap().contains("校验失败"));
}

#[tokio::test]
async fn api_artifact_source_should_read_and_update_file_artifact() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_minimal_workspace(&temp);

    let server = test_server(&temp, temp.child("missing-dist").path().to_path_buf());

    let resp = server.get("/api/artifacts/hello/source").await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["artifact_id"], serde_json::json!("hello"));
    assert_eq!(body["content"], serde_json::json!("Hello {{ name }}!"));

    let resp = server
        .put("/api/artifacts/hello/source")
        .json(&serde_json::json!({ "content": "Hi {{ name }}!" }))
        .await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["content"], serde_json::json!("Hi {{ name }}!"));

    let updated = std::fs::read_to_string(temp.child("artifacts/hello.txt.tera").path()).unwrap();
    assert_eq!(updated, "Hi {{ name }}!");

    let watch_resp = server.get("/api/watch-status").await;
    watch_resp.assert_status_ok();
    let watch_body: serde_json::Value = watch_resp.json();
    assert_eq!(watch_body["revision"], serde_json::json!(1));
    assert_eq!(
        watch_body["recent_events"][0]["summary"],
        serde_json::json!("save artifacts/hello.txt.tera")
    );
}

#[tokio::test]
async fn api_render_should_return_json_error_for_invalid_artifact_id() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_minimal_workspace(&temp);

    let server = test_server(&temp, temp.child("missing-dist").path().to_path_buf());
    let resp = server
        .get("/api/render")
        .add_query_param("artifact", "bad/id")
        .add_query_param("profile", "base")
        .await;

    resp.assert_status_bad_request();
    let body: serde_json::Value = resp.json();
    assert!(body["message"].as_str().unwrap().contains("参数错误"));
}

#[tokio::test]
async fn api_render_should_return_json_error_for_missing_profile() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_minimal_workspace(&temp);

    let server = test_server(&temp, temp.child("missing-dist").path().to_path_buf());
    let resp = server
        .get("/api/render")
        .add_query_param("artifact", "hello")
        .add_query_param("profile", "ghost")
        .await;

    resp.assert_status_bad_request();
    let body: serde_json::Value = resp.json();
    assert!(body["message"].as_str().unwrap().contains("profile 不存在"));
}
