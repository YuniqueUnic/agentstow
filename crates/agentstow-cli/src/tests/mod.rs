#[test]
fn shell_kind_from_str_should_work() {
    let s: agentstow_core::ShellKind = "bash".parse().unwrap();
    assert!(matches!(s, agentstow_core::ShellKind::Bash));

    let s: agentstow_core::ShellKind = "powershell".parse().unwrap();
    assert!(matches!(s, agentstow_core::ShellKind::Powershell));

    let s: agentstow_core::ShellKind = "pwsh".parse().unwrap();
    assert!(matches!(s, agentstow_core::ShellKind::Powershell));

    let s: agentstow_core::ShellKind = "pwsh.exe".parse().unwrap();
    assert!(matches!(s, agentstow_core::ShellKind::Powershell));

    let s: agentstow_core::ShellKind = "cmd".parse().unwrap();
    assert!(matches!(s, agentstow_core::ShellKind::Cmd));

    let s: agentstow_core::ShellKind = "cmd.exe".parse().unwrap();
    assert!(matches!(s, agentstow_core::ShellKind::Cmd));
}
