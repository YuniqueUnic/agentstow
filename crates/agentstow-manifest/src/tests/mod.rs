use super::*;
use agentstow_core::ProfileName;
use assert_fs::prelude::*;
use pretty_assertions::assert_eq;
use std::path::PathBuf;

#[test]
fn profile_merge_order_later_extends_wins_and_self_wins() {
    let base = Profile {
        extends: vec![],
        vars: serde_json::json!({ "k": 1, "base": true })
            .as_object()
            .unwrap()
            .clone(),
        var_syntax: ProfileVarSyntaxMode::Inline,
    };
    let work = Profile {
        extends: vec![],
        vars: serde_json::json!({ "k": 2, "work": true })
            .as_object()
            .unwrap()
            .clone(),
        var_syntax: ProfileVarSyntaxMode::Inline,
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
        var_syntax: ProfileVarSyntaxMode::Inline,
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
        var_syntax: ProfileVarSyntaxMode::Inline,
    };
    let b = Profile {
        extends: vec![ProfileName::new_unchecked("a")],
        vars: serde_json::Map::new(),
        var_syntax: ProfileVarSyntaxMode::Inline,
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
fn load_should_support_inline_profile_vars() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("agentstow.toml")
        .write_str(
            r#"
[profiles.base]
name = "AgentStow"
region = "cn"
"#,
        )
        .unwrap();

    let manifest = Manifest::load_from_path(temp.child("agentstow.toml").path()).unwrap();
    let profile = manifest
        .profiles
        .get(&ProfileName::new_unchecked("base"))
        .unwrap();

    assert_eq!(profile.var_syntax_mode(), ProfileVarSyntaxMode::Inline);
    assert_eq!(
        profile.declared_vars().get("name").unwrap(),
        &serde_json::json!("AgentStow")
    );
    assert_eq!(
        profile.declared_vars().get("region").unwrap(),
        &serde_json::json!("cn")
    );
}

#[test]
fn load_should_support_vars_object_profile_vars() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("agentstow.toml")
        .write_str(
            r#"
[profiles.base]
vars = { name = "AgentStow", region = "global" }
"#,
        )
        .unwrap();

    let manifest = Manifest::load_from_path(temp.child("agentstow.toml").path()).unwrap();
    let profile = manifest
        .profiles
        .get(&ProfileName::new_unchecked("base"))
        .unwrap();

    assert_eq!(profile.var_syntax_mode(), ProfileVarSyntaxMode::VarsObject);
    assert_eq!(
        profile.declared_vars().get("name").unwrap(),
        &serde_json::json!("AgentStow")
    );
    assert_eq!(
        profile.declared_vars().get("region").unwrap(),
        &serde_json::json!("global")
    );
}

#[test]
fn load_should_support_mixed_profile_vars_when_keys_do_not_overlap() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("agentstow.toml")
        .write_str(
            r#"
[profiles.base]
vars = { name = "AgentStow" }
region = "cn"
"#,
        )
        .unwrap();

    let manifest = Manifest::load_from_path(temp.child("agentstow.toml").path()).unwrap();
    let profile = manifest
        .profiles
        .get(&ProfileName::new_unchecked("base"))
        .unwrap();

    assert_eq!(profile.var_syntax_mode(), ProfileVarSyntaxMode::Mixed);
    assert_eq!(
        profile.declared_vars().get("name").unwrap(),
        &serde_json::json!("AgentStow")
    );
    assert_eq!(
        profile.declared_vars().get("region").unwrap(),
        &serde_json::json!("cn")
    );
}

#[test]
fn load_should_error_when_profile_vars_are_declared_twice() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("agentstow.toml")
        .write_str(
            r#"
[profiles.base]
vars = { name = "AgentStow" }
name = "Shadow"
"#,
        )
        .unwrap();

    let err = Manifest::load_from_path(temp.child("agentstow.toml").path()).unwrap_err();
    assert_eq!(err.exit_code(), agentstow_core::ExitCode::InvalidConfig);
    assert!(
        err.to_string()
            .contains("同时出现在 `vars` 和 profile 顶层")
    );
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
fn load_should_parse_env_file_inline_vars_file_contexts_and_mcp_imports() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("mcps.json")
        .write_str(
            r#"{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "."]
    }
  }
}"#,
        )
        .unwrap();
    temp.child("agentstow.toml")
        .write_str(
            r#"
[profiles.base]
vars = { name = "AgentStow" }

[env]
DIRECT_ENV = "from-inline"
DUPLICATE_ENV = "from-manifest"

[env.files]
paths = [".env"]

[env.emit.default]
vars = [
  { key = "OPENAI_API_KEY", binding = { kind = "env", var = "OPENAI_API_KEY" } }
]

[file.reference]
path = "reference.md"

[mcp_servers.file]
path = "mcps.json"

[mcp_servers.local]
[mcp_servers.local.transport]
kind = "http"
url = "https://example.com/mcp"

"#,
        )
        .unwrap();

    let manifest = Manifest::load_from_path(temp.child("agentstow.toml").path()).unwrap();
    assert_eq!(manifest.env.files.paths, vec![PathBuf::from(".env")]);
    assert_eq!(manifest.env.vars.get("DIRECT_ENV").unwrap(), "from-inline");
    assert!(manifest.env.emit.contains_key("default"));
    assert_eq!(
        manifest.env.vars.get("DUPLICATE_ENV").unwrap(),
        "from-manifest"
    );
    assert_eq!(
        manifest.file.get("reference").unwrap().path,
        PathBuf::from("reference.md")
    );
    assert!(manifest.mcp_servers.contains_key("filesystem"));
    assert!(manifest.mcp_servers.contains_key("local"));
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
fn load_should_error_when_targets_have_same_target_path() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("agentstow.toml")
        .write_str(
            r#"
[profiles.base]
vars = {}

[artifacts.one]
kind = "file"
source = "artifacts/one.txt"

[artifacts.two]
kind = "file"
source = "artifacts/two.txt"

[targets.one]
artifact = "one"
profile = "base"
target_path = "proj/shared.txt"
method = "copy"

[targets.two]
artifact = "two"
profile = "base"
target_path = "proj/shared.txt"
method = "copy"
"#,
        )
        .unwrap();

    let err = Manifest::load_from_path(temp.child("agentstow.toml").path()).unwrap_err();
    assert_eq!(err.exit_code(), agentstow_core::ExitCode::InvalidConfig);
    assert!(err.to_string().contains("targets target_path 发生重叠"));
    assert!(err.to_string().contains("proj/shared.txt"));
}

#[test]
fn load_should_error_when_targets_overlap_after_relative_path_normalization() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("agentstow.toml")
        .write_str(
            r#"
[profiles.base]
vars = {}

[artifacts.one]
kind = "file"
source = "artifacts/one.txt"

[artifacts.two]
kind = "file"
source = "artifacts/two.txt"

[targets.parent]
artifact = "one"
profile = "base"
target_path = "proj/.agents"
method = "copy"

[targets.child]
artifact = "two"
profile = "base"
target_path = "proj/tmp/../.agents/skills"
method = "copy"
"#,
        )
        .unwrap();

    let err = Manifest::load_from_path(temp.child("agentstow.toml").path()).unwrap_err();
    assert_eq!(err.exit_code(), agentstow_core::ExitCode::InvalidConfig);
    assert!(err.to_string().contains("targets target_path 发生重叠"));
    assert!(err.to_string().contains("proj/.agents"));
    assert!(err.to_string().contains("proj/.agents/skills"));
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
