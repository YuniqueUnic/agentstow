use crate::export_bindings;

#[test]
fn export_bindings_should_succeed() {
    export_bindings().expect("ts-rs 导出 bindings 失败");
}
