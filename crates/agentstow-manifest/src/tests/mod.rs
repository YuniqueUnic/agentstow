use super::*;
use agentstow_core::ProfileName;
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
