use pretty_assertions::assert_eq;

use super::*;

#[test]
fn validate_json_should_fail_for_invalid_json() {
    let artifact = agentstow_manifest::ArtifactDef {
        kind: agentstow_core::ArtifactKind::File,
        source: "irrelevant".into(),
        template: false,
        validate_as: agentstow_core::ValidateAs::Json,
    };
    let err = Validator::validate_rendered_file(&artifact, br#"{ invalid }"#).unwrap_err();
    assert_eq!(err.exit_code(), agentstow_core::ExitCode::ValidationFailed);
}

#[test]
fn validate_toml_should_fail_for_non_utf8_bytes() {
    let artifact = agentstow_manifest::ArtifactDef {
        kind: agentstow_core::ArtifactKind::File,
        source: "irrelevant".into(),
        template: false,
        validate_as: agentstow_core::ValidateAs::Toml,
    };
    let err = Validator::validate_rendered_file(&artifact, &[0xff, 0xfe]).unwrap_err();
    assert_eq!(err.exit_code(), agentstow_core::ExitCode::ValidationFailed);
}

#[test]
fn validate_shell_should_fail_for_nul_byte() {
    let artifact = agentstow_manifest::ArtifactDef {
        kind: agentstow_core::ArtifactKind::File,
        source: "irrelevant".into(),
        template: false,
        validate_as: agentstow_core::ValidateAs::Shell,
    };
    let err = Validator::validate_rendered_file(&artifact, b"echo hi\0").unwrap_err();
    assert_eq!(err.exit_code(), agentstow_core::ExitCode::ValidationFailed);
}
