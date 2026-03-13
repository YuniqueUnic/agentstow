use super::*;

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
