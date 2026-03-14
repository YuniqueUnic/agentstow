use crate::export_bindings;

#[test]
fn export_bindings_should_succeed() {
    let dir = tempfile::tempdir().expect("create temp dir failed");
    temp_env::with_var(
        "TS_RS_EXPORT_DIR",
        Some(dir.path().to_string_lossy().as_ref()),
        || {
            export_bindings().expect("ts-rs 导出 bindings 失败");
        },
    );
}
