use assert_fs::prelude::*;
use pretty_assertions::assert_eq;

use super::*;

#[test]
fn render_tera_template_should_work() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("artifacts").create_dir_all().unwrap();
    temp.child("artifacts/hello.txt.tera")
        .write_str("Hello {{ name }}!")
        .unwrap();

    temp.child("agentstow.toml")
        .write_str(
            r#"
[profiles.base]
vars = { name = "AgentStow" }

[artifacts.hello]
kind = "file"
source = "artifacts/hello.txt.tera"
template = true
validate_as = "none"
"#,
        )
        .unwrap();

    let manifest = Manifest::load_from_path(&temp.child("agentstow.toml").path()).unwrap();
    let out = Renderer::render_file(
        &manifest,
        &ArtifactId::new_unchecked("hello"),
        &ProfileName::new_unchecked("base"),
    )
    .unwrap();

    assert_eq!(String::from_utf8(out.bytes).unwrap(), "Hello AgentStow!");
}
