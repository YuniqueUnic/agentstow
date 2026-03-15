use assert_fs::prelude::*;
use pretty_assertions::assert_eq;
use serial_test::serial;

use super::fixtures::{
    block_on, test_server, upsert_link_instance, with_test_home, write_minimal_workspace,
    write_prd_workspace,
};

#[test]
#[serial]
fn api_links_should_serialize_link_records_with_shared_dto() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_minimal_workspace(&temp);
    temp.child("proj/hello.txt")
        .write_str("Hello Server!")
        .unwrap();
    temp.child("cache").create_dir_all().unwrap();
    temp.child("home").create_dir_all().unwrap();

    temp_env::with_var("AGENTSTOW_HOME", Some(temp.child("home").path()), || {
        upsert_link_instance(&agentstow_state::LinkInstanceRecord {
            workspace_root: temp.path().to_path_buf(),
            artifact_id: agentstow_core::ArtifactId::new_unchecked("hello"),
            profile: agentstow_core::ProfileName::new_unchecked("base"),
            target_path: temp.child("proj/hello.txt").path().to_path_buf(),
            method: agentstow_core::InstallMethod::Copy,
            rendered_path: Some(temp.child("cache/hello.txt").path().to_path_buf()),
            blake3: Some("deadbeef".to_string()),
            updated_at: time::OffsetDateTime::UNIX_EPOCH,
        });

        block_on(async {
            let server = test_server(&temp, temp.child("missing-dist").path().to_path_buf());
            let resp = server.get("/api/links").await;

            resp.assert_status_ok();
            let body: serde_json::Value = resp.json();
            assert_eq!(body.as_array().unwrap().len(), 1);
            assert_eq!(body[0]["artifact_id"], serde_json::json!("hello"));
            assert_eq!(body[0]["method"], serde_json::json!("copy"));
            assert_eq!(body[0]["blake3"], serde_json::json!("deadbeef"));
            assert_eq!(
                body[0]["updated_at"],
                serde_json::json!("1970-01-01T00:00:00Z")
            );
        });
    });
}

#[test]
#[serial]
fn api_links_plan_apply_and_repair_should_work() {
    let temp = assert_fs::TempDir::new().unwrap();
    write_prd_workspace(&temp);

    with_test_home(&temp, || {
        block_on(async {
            let server = test_server(&temp, temp.child("missing-dist").path().to_path_buf());

            let resp = server
                .post("/api/links/plan")
                .json(&serde_json::json!({
                    "targets": [],
                    "default_profile": "base",
                }))
                .await;
            resp.assert_status_ok();
            let planned: serde_json::Value = resp.json();
            assert_eq!(planned["items"].as_array().unwrap().len(), 3);
            assert_eq!(planned["items"][0]["action"], serde_json::json!("planned"));

            let resp = server
                .post("/api/links/apply")
                .json(&serde_json::json!({
                    "targets": [],
                    "default_profile": "base",
                    "force": false,
                }))
                .await;
            resp.assert_status_ok();
            let applied: serde_json::Value = resp.json();
            assert_eq!(applied["items"].as_array().unwrap().len(), 3);
            assert!(temp.child("proj/AGENTS.md").path().is_file());
            assert!(temp.child("proj/adhoc.md").path().is_file());
            assert!(temp.child("proj/.agents/skills/rule.md").path().is_file());

            let resp = server
                .post("/api/links/repair")
                .json(&serde_json::json!({
                    "targets": [],
                    "default_profile": "base",
                    "force": false,
                }))
                .await;
            resp.assert_status_ok();
            let repaired: serde_json::Value = resp.json();
            assert_eq!(repaired["items"].as_array().unwrap().len(), 3);
        });
    });
}

#[test]
#[serial]
fn api_link_status_should_report_copy_dir_as_healthy_when_tree_matches() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("artifacts/skills").create_dir_all().unwrap();
    temp.child("artifacts/skills/rule.md")
        .write_str("hello")
        .unwrap();
    temp.child("proj/.agents/skills").create_dir_all().unwrap();
    temp.child("proj/.agents/skills/rule.md")
        .write_str("hello")
        .unwrap();
    temp.child("home").create_dir_all().unwrap();
    temp.child("agentstow.toml")
        .write_str(
            r#"
[profiles.base]
vars = { name = "demo" }

[artifacts.skills]
kind = "dir"
source = "artifacts/skills"

[targets.skills]
artifact = "skills"
profile = "base"
target_path = "proj/.agents/skills"
method = "copy"
"#,
        )
        .unwrap();

    temp_env::with_var("AGENTSTOW_HOME", Some(temp.child("home").path()), || {
        upsert_link_instance(&agentstow_state::LinkInstanceRecord {
            workspace_root: temp.path().to_path_buf(),
            artifact_id: agentstow_core::ArtifactId::new_unchecked("skills"),
            profile: agentstow_core::ProfileName::new_unchecked("base"),
            target_path: temp.child("proj/.agents/skills").path().to_path_buf(),
            method: agentstow_core::InstallMethod::Copy,
            rendered_path: Some(temp.child("artifacts/skills").path().to_path_buf()),
            blake3: None,
            updated_at: time::OffsetDateTime::now_utc(),
        });

        block_on(async {
            let server = test_server(&temp, temp.child("missing-dist").path().to_path_buf());
            let resp = server.get("/api/link-status").await;

            resp.assert_status_ok();
            let body: serde_json::Value = resp.json();
            assert_eq!(body.as_array().unwrap().len(), 1);
            assert_eq!(body[0]["ok"], serde_json::json!(true));
            assert_eq!(body[0]["message"], serde_json::json!("healthy"));
            assert_eq!(body[0]["method"], serde_json::json!("copy"));
        });
    });
}
