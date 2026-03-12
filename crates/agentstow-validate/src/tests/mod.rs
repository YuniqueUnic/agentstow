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
