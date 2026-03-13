use super::*;
use proptest::prelude::*;

#[test]
fn secret_binding_env_missing_should_error() {
    let binding = SecretBinding::Env {
        var: "AGENTSTOW__TEST_MISSING".to_string(),
    };
    let err = binding.resolve().unwrap_err();
    assert_eq!(err.exit_code(), ExitCode::ValidationFailed);
}

#[test]
fn ids_should_reject_path_separators() {
    let err = ArtifactId::parse("a/b").unwrap_err();
    assert_eq!(err.exit_code(), ExitCode::InvalidArgs);

    let err = ProfileName::parse("base\\work").unwrap_err();
    assert_eq!(err.exit_code(), ExitCode::InvalidArgs);

    let err = TargetName::parse("..").unwrap_err();
    // `..` 本身不是路径分隔符，但按当前规则不允许以避免歧义/误用
    assert_eq!(err.exit_code(), ExitCode::InvalidArgs);
}

proptest! {
    #[test]
    fn ids_should_accept_ascii_safe_characters(s in "[A-Za-z0-9_.-]{1,64}") {
        prop_assume!(s != "." && s != "..");

        let artifact = ArtifactId::parse(s.clone());
        let profile = ProfileName::parse(s.clone());
        let target = TargetName::parse(s);

        prop_assert!(artifact.is_ok());
        prop_assert!(profile.is_ok());
        prop_assert!(target.is_ok());
    }

    #[test]
    fn ids_should_reject_out_of_policy_strings(s in "[^A-Za-z0-9_.-][^\n]{0,70}|.{65,80}") {
        let artifact = ArtifactId::parse(s.clone());
        let profile = ProfileName::parse(s.clone());
        let target = TargetName::parse(s);

        prop_assert!(artifact.is_err());
        prop_assert!(profile.is_err());
        prop_assert!(target.is_err());
    }
}
