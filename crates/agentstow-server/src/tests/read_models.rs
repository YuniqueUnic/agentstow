use assert_fs::prelude::*;
use pretty_assertions::assert_eq;
use serial_test::serial;

use super::fixtures::{
    block_on, test_server, upsert_link_instance, with_test_home, write_http_mcp_workspace,
    write_prd_workspace,
};

#[test]
#[serial]
fn api_workspace_summary_should_expose_prd_read_model() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_prd_workspace(&temp);

    with_test_home(&temp, || {
        block_on(async {
            let server = test_server(&temp, temp.child("missing-dist").path().to_path_buf());
            let resp = server.get("/api/workspace-summary").await;

            resp.assert_status_ok();
            let body: serde_json::Value = resp.json();
            assert_eq!(body["counts"]["profile_count"], serde_json::json!(2));
            assert_eq!(body["counts"]["artifact_count"], serde_json::json!(2));
            assert_eq!(body["counts"]["target_count"], serde_json::json!(3));
            assert_eq!(body["counts"]["env_set_count"], serde_json::json!(1));
            assert_eq!(body["counts"]["script_count"], serde_json::json!(1));
            assert_eq!(body["counts"]["mcp_server_count"], serde_json::json!(1));
            assert_eq!(body["mcp_servers"][0]["command"], serde_json::json!("npx"));
            assert_eq!(
                body["mcp_servers"][0]["args"],
                serde_json::json!(["-y", "@modelcontextprotocol/server-filesystem", "."])
            );
            assert_eq!(body["mcp_servers"][0]["url"], serde_json::Value::Null);
            assert_eq!(body["mcp_servers"][0]["headers"], serde_json::json!([]));
            assert_eq!(
                body["mcp_servers"][0]["env_bindings"][0]["key"],
                serde_json::json!("OPENAI_API_KEY")
            );
            assert_eq!(
                body["mcp_servers"][0]["env_bindings"][0]["binding"],
                serde_json::json!("env:OPENAI_API_KEY")
            );
            assert_eq!(
                body["mcp_servers"][0]["env_bindings"][0]["binding_kind"],
                serde_json::json!("env")
            );
            assert_eq!(
                body["mcp_servers"][0]["env_bindings"][0]["rendered_placeholder"],
                serde_json::json!("${OPENAI_API_KEY}")
            );
            assert_eq!(
                body["mcp_servers"][0]["env_bindings"][0]["available"],
                serde_json::json!(false)
            );
            assert_eq!(body["env_sets"][0]["missing_count"], serde_json::json!(1));
            assert_eq!(
                body["issues"][0]["code"],
                serde_json::json!("target_profile_missing")
            );
        });
    });
}

#[test]
#[serial]
fn api_workspace_summary_should_expose_http_mcp_headers() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_http_mcp_workspace(&temp);

    with_test_home(&temp, || {
        block_on(async {
            let server = test_server(&temp, temp.child("missing-dist").path().to_path_buf());
            let resp = server.get("/api/workspace-summary").await;

            resp.assert_status_ok();
            let body: serde_json::Value = resp.json();
            assert_eq!(body["counts"]["mcp_server_count"], serde_json::json!(1));
            assert_eq!(body["mcp_servers"][0]["id"], serde_json::json!("remote"));
            assert_eq!(body["mcp_servers"][0]["command"], serde_json::Value::Null);
            assert_eq!(body["mcp_servers"][0]["args"], serde_json::json!([]));
            assert_eq!(
                body["mcp_servers"][0]["url"],
                serde_json::json!("https://example.com/mcp")
            );
            assert_eq!(
                body["mcp_servers"][0]["headers"],
                serde_json::json!([
                    { "key": "Authorization", "value": "Bearer demo-token" },
                    { "key": "X-Workspace", "value": "agentstow" }
                ])
            );
        });
    });
}

#[test]
#[serial]
fn api_mcp_validate_render_and_test_should_close_the_loop() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_http_mcp_workspace(&temp);

    block_on(async {
        let server = test_server(&temp, temp.child("missing-dist").path().to_path_buf());

        let validate = server.post("/api/mcp/remote/validate").await;
        validate.assert_status_ok();
        let validate_body: serde_json::Value = validate.json();
        assert_eq!(validate_body["server_id"], serde_json::json!("remote"));
        assert_eq!(validate_body["ok"], serde_json::json!(true));
        assert_eq!(
            validate_body["issues"][0]["code"],
            serde_json::json!("mcp_env_unavailable")
        );

        let render = server.get("/api/mcp/remote/render").await;
        render.assert_status_ok();
        let render_body: serde_json::Value = render.json();
        assert_eq!(render_body["server_id"], serde_json::json!("remote"));
        assert_eq!(render_body["transport_kind"], serde_json::json!("http"));
        assert!(
            render_body["launcher_preview"]
                .as_str()
                .unwrap()
                .contains("GET https://example.com/mcp")
        );
        assert!(
            render_body["config_json"]
                .as_str()
                .unwrap()
                .contains("\"remote\"")
        );

        let test = server.post("/api/mcp/remote/test").await;
        test.assert_status_ok();
        let test_body: serde_json::Value = test.json();
        assert_eq!(test_body["server_id"], serde_json::json!("remote"));
        assert_eq!(test_body["ok"], serde_json::json!(false));
        assert!(
            test_body["checks"]
                .as_array()
                .unwrap()
                .iter()
                .any(|check| check["code"] == "env:OPENAI_API_KEY" && check["status"] == "error")
        );
    });
}

#[test]
#[serial]
fn api_artifact_detail_should_include_targets_profiles_and_issues() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_prd_workspace(&temp);
    temp.child("proj").create_dir_all().unwrap();
    temp.child("proj/AGENTS.md")
        .write_str("Hello AgentStow from cn!")
        .unwrap();

    with_test_home(&temp, || {
        upsert_link_instance(&agentstow_state::LinkInstanceRecord {
            workspace_root: temp.path().to_path_buf(),
            artifact_id: agentstow_core::ArtifactId::new_unchecked("hello"),
            profile: agentstow_core::ProfileName::new_unchecked("derived"),
            target_path: temp.child("proj/AGENTS.md").path().to_path_buf(),
            method: agentstow_core::InstallMethod::Copy,
            rendered_path: None,
            blake3: Some("abc123".to_string()),
            updated_at: time::OffsetDateTime::UNIX_EPOCH,
        });

        block_on(async {
            let server = test_server(&temp, temp.child("missing-dist").path().to_path_buf());
            let resp = server.get("/api/artifacts/hello").await;

            resp.assert_status_ok();
            let body: serde_json::Value = resp.json();
            assert_eq!(body["artifact"]["id"], serde_json::json!("hello"));
            assert_eq!(body["targets"].as_array().unwrap().len(), 2);
            assert_eq!(body["profiles"].as_array().unwrap().len(), 1);
            assert_eq!(
                body["issues"][0]["code"],
                serde_json::json!("target_profile_missing")
            );
        });
    });
}

#[test]
#[serial]
fn api_profile_detail_should_include_merged_vars_and_artifacts() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_prd_workspace(&temp);

    with_test_home(&temp, || {
        block_on(async {
            let server = test_server(&temp, temp.child("missing-dist").path().to_path_buf());
            let resp = server.get("/api/profiles/derived").await;

            resp.assert_status_ok();
            let body: serde_json::Value = resp.json();
            assert_eq!(body["profile"]["id"], serde_json::json!("derived"));
            assert_eq!(body["syntax_mode"], serde_json::json!("inline"));
            assert_eq!(body["targets"].as_array().unwrap().len(), 1);
            assert_eq!(body["artifacts"][0]["id"], serde_json::json!("hello"));
            let declared_keys: Vec<_> = body["declared_vars"]
                .as_array()
                .unwrap()
                .iter()
                .map(|item| item["key"].as_str().unwrap())
                .collect();
            assert_eq!(declared_keys, vec!["region"]);
            let merged_keys: Vec<_> = body["merged_vars"]
                .as_array()
                .unwrap()
                .iter()
                .map(|item| item["key"].as_str().unwrap())
                .collect();
            assert!(merged_keys.contains(&"name"));
            assert!(merged_keys.contains(&"region"));
        });
    });
}

#[test]
#[serial]
fn api_impact_should_filter_by_artifact_and_include_link_status() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_prd_workspace(&temp);
    temp.child("proj").create_dir_all().unwrap();
    temp.child("proj/AGENTS.md")
        .write_str("Hello AgentStow from cn!")
        .unwrap();

    with_test_home(&temp, || {
        upsert_link_instance(&agentstow_state::LinkInstanceRecord {
            workspace_root: temp.path().to_path_buf(),
            artifact_id: agentstow_core::ArtifactId::new_unchecked("hello"),
            profile: agentstow_core::ProfileName::new_unchecked("derived"),
            target_path: temp.child("proj/AGENTS.md").path().to_path_buf(),
            method: agentstow_core::InstallMethod::Copy,
            rendered_path: None,
            blake3: Some("abc123".to_string()),
            updated_at: time::OffsetDateTime::UNIX_EPOCH,
        });

        block_on(async {
            let server = test_server(&temp, temp.child("missing-dist").path().to_path_buf());
            let resp = server
                .get("/api/impact")
                .add_query_param("artifact", "hello")
                .await;

            resp.assert_status_ok();
            let body: serde_json::Value = resp.json();
            assert_eq!(body["subject_kind"], serde_json::json!("artifact"));
            assert_eq!(body["affected_targets"].as_array().unwrap().len(), 2);
            assert_eq!(body["affected_artifacts"].as_array().unwrap().len(), 1);
            assert_eq!(body["link_status"].as_array().unwrap().len(), 1);
        });
    });
}
