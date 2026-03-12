#[test]
fn shell_kind_from_str_should_work() {
    let s: agentstow_core::ShellKind = "bash".parse().unwrap();
    assert!(matches!(s, agentstow_core::ShellKind::Bash));
}
