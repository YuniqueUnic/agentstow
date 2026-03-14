use super::*;
use agentstow_core::ProfileName;
use assert_fs::prelude::*;
use pretty_assertions::assert_eq;

#[test]
fn profile_merge_order_later_extends_wins_and_self_wins() {
    let base = Profile {
        extends: vec![],
        vars: serde_json::json!({ "k": 1, "base": true })
            .as_object()
            .unwrap()
            .clone(),
    };
    let work = Profile {
        extends: vec![],
        vars: serde_json::json!({ "k": 2, "work": true })
            .as_object()
            .unwrap()
            .clone(),
    };
    let child = Profile {
        extends: vec![
            ProfileName::new_unchecked("base"),
            ProfileName::new_unchecked("work"),
        ],
        vars: serde_json::json!({ "k": 3, "child": true })
            .as_object()
            .unwrap()
            .clone(),
    };

    let mut profiles = BTreeMap::new();
    profiles.insert(ProfileName::new_unchecked("base"), base);
    profiles.insert(ProfileName::new_unchecked("work"), work);
    profiles.insert(ProfileName::new_unchecked("child"), child);

    let merged = profiles
        .get(&ProfileName::new_unchecked("child"))
        .unwrap()
        .merged_vars(&profiles)
        .unwrap();

    assert_eq!(merged.get("k").unwrap(), &serde_json::json!(3));
    assert_eq!(merged.get("base").unwrap(), &serde_json::json!(true));
    assert_eq!(merged.get("work").unwrap(), &serde_json::json!(true));
    assert_eq!(merged.get("child").unwrap(), &serde_json::json!(true));
}

#[test]
fn profile_cycle_should_error() {
    let a = Profile {
        extends: vec![ProfileName::new_unchecked("b")],
        vars: serde_json::Map::new(),
    };
    let b = Profile {
        extends: vec![ProfileName::new_unchecked("a")],
        vars: serde_json::Map::new(),
    };
    let mut profiles = BTreeMap::new();
    profiles.insert(ProfileName::new_unchecked("a"), a);
    profiles.insert(ProfileName::new_unchecked("b"), b);

    let err = profiles
        .get(&ProfileName::new_unchecked("a"))
        .unwrap()
        .merged_vars(&profiles)
        .unwrap_err();

    assert_eq!(err.exit_code(), agentstow_core::ExitCode::InvalidConfig);
}

#[test]
fn find_from_should_error_when_manifest_is_missing() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("nested/work").create_dir_all().unwrap();

    let err = Manifest::find_from(temp.child("nested/work").path()).unwrap_err();
    assert_eq!(err.exit_code(), agentstow_core::ExitCode::InvalidConfig);
    assert!(err.to_string().contains("未找到 agentstow.toml"));
}

#[test]
fn load_should_error_when_target_references_missing_artifact() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("agentstow.toml")
        .write_str(
            r#"
[profiles.base]
vars = {}

[targets.missing]
artifact = "ghost"
profile = "base"
target_path = "out.txt"
method = "copy"
"#,
        )
        .unwrap();

    let err = Manifest::load_from_path(temp.child("agentstow.toml").path()).unwrap_err();
    assert_eq!(err.exit_code(), agentstow_core::ExitCode::InvalidConfig);
    assert!(err.to_string().contains("target 引用不存在的 artifact"));
}

#[test]
fn load_should_error_when_profile_extends_missing_profile() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("agentstow.toml")
        .write_str(
            r#"
[profiles.base]
extends = ["ghost"]
vars = {}
"#,
        )
        .unwrap();

    let err = Manifest::load_from_path(temp.child("agentstow.toml").path()).unwrap_err();
    assert_eq!(err.exit_code(), agentstow_core::ExitCode::InvalidConfig);
    assert!(err.to_string().contains("profile extends 不存在"));
}

#[test]
fn load_should_error_when_profiles_have_cycle() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("agentstow.toml")
        .write_str(
            r#"
[profiles.a]
extends = ["b"]
vars = {}

[profiles.b]
extends = ["a"]
vars = {}
"#,
        )
        .unwrap();

    let err = Manifest::load_from_path(temp.child("agentstow.toml").path()).unwrap_err();
    assert_eq!(err.exit_code(), agentstow_core::ExitCode::InvalidConfig);
    assert!(err.to_string().contains("循环引用"));
}

#[test]
fn load_should_error_when_target_references_missing_profile() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("agentstow.toml")
        .write_str(
            r#"
[artifacts.hello]
kind = "file"
source = "hello.txt"
template = false
validate_as = "none"

[targets.out]
artifact = "hello"
profile = "ghost"
target_path = "out.txt"
method = "copy"
"#,
        )
        .unwrap();

    let err = Manifest::load_from_path(temp.child("agentstow.toml").path()).unwrap_err();
    assert_eq!(err.exit_code(), agentstow_core::ExitCode::InvalidConfig);
    assert!(err.to_string().contains("target 引用不存在的 profile"));
}

#[test]
fn load_should_error_when_target_path_overlaps_artifact_source() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("agentstow.toml")
        .write_str(
            r#"
[artifacts.skills]
kind = "dir"
source = "artifacts/skills"

[targets.bad]
artifact = "skills"
target_path = "artifacts/skills/project-output"
method = "copy"
"#,
        )
        .unwrap();

    let err = Manifest::load_from_path(temp.child("agentstow.toml").path()).unwrap_err();
    assert_eq!(err.exit_code(), agentstow_core::ExitCode::InvalidConfig);
    assert!(
        err.to_string()
            .contains("target 路径与 artifact source 重叠")
    );
}

#[test]
fn load_should_error_for_invalid_toml_syntax() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("agentstow.toml")
        .write_str("[profiles.base\nvars = {}")
        .unwrap();

    let err = Manifest::load_from_path(temp.child("agentstow.toml").path()).unwrap_err();
    assert_eq!(err.exit_code(), agentstow_core::ExitCode::InvalidConfig);
    assert!(err.to_string().contains("解析 manifest 失败"));
}

#[test]
fn profile_vars_should_error_when_profile_is_missing() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("agentstow.toml")
        .write_str(
            r#"
[profiles.base]
vars = {}
"#,
        )
        .unwrap();

    let manifest = Manifest::load_from_path(temp.child("agentstow.toml").path()).unwrap();
    let err = manifest
        .profile_vars(&ProfileName::new_unchecked("ghost"))
        .unwrap_err();
    assert_eq!(err.exit_code(), agentstow_core::ExitCode::InvalidConfig);
    assert!(err.to_string().contains("profile 不存在"));
}

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
}
