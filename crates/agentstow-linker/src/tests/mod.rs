use assert_fs::prelude::*;
use pretty_assertions::assert_eq;

use super::*;

#[test]
fn copy_file_should_write_bytes() {
    let temp = assert_fs::TempDir::new().unwrap();
    let store = RenderStore::new(temp.child("cache").path().to_path_buf(), temp.path());

    let job = LinkJob {
        target: TargetName::new_unchecked("t"),
        artifact_id: ArtifactId::new_unchecked("a"),
        profile: ProfileName::new_unchecked("p"),
        artifact_kind: ArtifactKind::File,
        method: InstallMethod::Copy,
        target_path: temp.child("out.txt").path().to_path_buf(),
        desired: InstallSource::FileBytes(b"hello".to_vec()),
    };

    let plan = apply_job(&job, &store, ApplyOptions { force: false }).unwrap();
    assert_eq!(plan.method, InstallMethod::Copy);

    let s = std::fs::read_to_string(temp.child("out.txt").path()).unwrap();
    assert_eq!(s, "hello");
}

#[test]
fn symlink_file_should_point_to_render_store() {
    let temp = assert_fs::TempDir::new().unwrap();
    let store = RenderStore::new(temp.child("cache").path().to_path_buf(), temp.path());

    let job = LinkJob {
        target: TargetName::new_unchecked("t"),
        artifact_id: ArtifactId::new_unchecked("a"),
        profile: ProfileName::new_unchecked("p"),
        artifact_kind: ArtifactKind::File,
        method: InstallMethod::Symlink,
        target_path: temp.child("out.txt").path().to_path_buf(),
        desired: InstallSource::FileBytes(b"hello".to_vec()),
    };

    let plan = apply_job(&job, &store, ApplyOptions { force: false }).unwrap();
    assert_eq!(plan.method, InstallMethod::Symlink);

    let rendered_path = store.rendered_file_path(
        &ArtifactId::new_unchecked("a"),
        &ProfileName::new_unchecked("p"),
    );
    let target = temp.child("out.txt").path().to_path_buf();
    assert!(is_correct_symlink(&target, &rendered_path).unwrap());
}
