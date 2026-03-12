use super::*;

#[test]
fn emit_bash_should_quote_single_quotes() {
    let vars = vec![("KEY".to_string(), "a'b".to_string())];
    let out = Env::emit_shell(agentstow_core::ShellKind::Bash, &vars).unwrap();
    assert!(out.contains("export KEY='a'\"'\"'b'"));
}
