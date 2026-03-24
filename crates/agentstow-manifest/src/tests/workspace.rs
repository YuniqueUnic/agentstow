use crate::{Manifest, init_workspace_skeleton, probe_workspace_path};
use agentstow_core::ProfileName;
use assert_fs::prelude::*;
use pretty_assertions::assert_eq;

#[test]
fn init_workspace_skeleton_should_create_manifest_and_sample_artifact() {
    let temp = assert_fs::TempDir::new().unwrap();
    let ws = temp.child("ws");

    let out = init_workspace_skeleton(ws.path()).unwrap();
    assert!(out.created);
    assert_eq!(out.workspace_root, ws.path().to_path_buf());
    assert!(out.manifest_path.exists());
    assert!(ws.child("artifacts/hello.txt.tera").path().exists());

    let manifest = Manifest::load_from_dir(ws.path()).unwrap();
    assert!(
        manifest
            .artifacts
            .contains_key(&agentstow_core::ArtifactId::new_unchecked("hello"))
    );
    assert!(
        manifest
            .profiles
            .contains_key(&ProfileName::new_unchecked("base"))
    );
    let source = std::fs::read_to_string(&out.manifest_path).unwrap();
    assert!(source.contains("name = \"AgentStow\""));
    assert!(!source.contains("vars ="));
}

#[test]
fn probe_workspace_path_should_accept_manifest_file_path() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("agentstow.toml")
        .write_str(
            r#"
[profiles.base]
name = "AgentStow"
"#,
        )
        .unwrap();

    let probe = probe_workspace_path(temp.child("agentstow.toml").path()).unwrap();
    assert!(probe.exists);
    assert!(!probe.is_directory);
    assert!(probe.manifest_present);
    assert!(probe.selectable);
    assert!(!probe.initializable);
    let expected_root = fs_err::canonicalize(temp.path()).unwrap();
    assert_eq!(probe.manifest_path, expected_root.join("agentstow.toml"));
    assert_eq!(probe.resolved_workspace_root, expected_root);
}

#[test]
fn probe_workspace_path_should_report_missing_workspace_candidate() {
    let temp = assert_fs::TempDir::new().unwrap();
    let probe = probe_workspace_path(temp.child("missing-workspace").path()).unwrap();

    assert!(!probe.exists);
    assert!(!probe.selectable);
    assert!(probe.initializable);
    assert_eq!(
        probe.manifest_path,
        temp.child("missing-workspace/agentstow.toml").path()
    );
    assert!(
        probe
            .reason
            .as_deref()
            .unwrap()
            .contains("可直接初始化 workspace")
    );
}
