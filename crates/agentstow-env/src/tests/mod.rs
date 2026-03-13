use super::*;

#[test]
fn emit_bash_should_quote_single_quotes() {
    let vars = vec![("KEY".to_string(), "a'b".to_string())];
    let out = Env::emit_shell(agentstow_core::ShellKind::Bash, &vars).unwrap();
    assert!(out.contains("export KEY='a'\"'\"'b'"));
}

#[test]
fn resolve_env_set_should_reject_invalid_shell_key() {
    let env_set = agentstow_manifest::EnvSet {
        vars: vec![agentstow_manifest::EnvVarDef {
            key: "BAD-KEY".to_string(),
            binding: agentstow_core::SecretBinding::Literal {
                value: "x".to_string(),
            },
        }],
    };

    let err = Env::resolve_env_set(&env_set).unwrap_err();
    assert_eq!(err.exit_code(), agentstow_core::ExitCode::InvalidConfig);
    assert!(err.to_string().contains("env key 非法"));
}

#[test]
fn emit_shell_should_preserve_newlines_inside_value() {
    let vars = vec![("KEY".to_string(), "line1\nline2".to_string())];
    let out = Env::emit_shell(agentstow_core::ShellKind::Bash, &vars).unwrap();
    assert!(out.contains("export KEY='line1\nline2'"));
}
