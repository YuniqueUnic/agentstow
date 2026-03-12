use super::*;

#[test]
fn secret_binding_env_missing_should_error() {
    let binding = SecretBinding::Env {
        var: "AGENTSTOW__TEST_MISSING".to_string(),
    };
    let err = binding.resolve().unwrap_err();
    assert_eq!(err.exit_code(), ExitCode::ValidationFailed);
}
