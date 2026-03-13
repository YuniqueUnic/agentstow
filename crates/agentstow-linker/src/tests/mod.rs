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
fn copy_file_force_should_overwrite_existing_file() {
    let temp = assert_fs::TempDir::new().unwrap();
    let store = RenderStore::new(temp.child("cache").path().to_path_buf(), temp.path());

    let target_path = temp.child("out.txt").path().to_path_buf();

    let job = LinkJob {
        target: TargetName::new_unchecked("t"),
        artifact_id: ArtifactId::new_unchecked("a"),
        profile: ProfileName::new_unchecked("p"),
        artifact_kind: ArtifactKind::File,
        method: InstallMethod::Copy,
        target_path: target_path.clone(),
        desired: InstallSource::FileBytes(b"hello".to_vec()),
    };

    apply_job(&job, &store, ApplyOptions { force: false }).unwrap();

    let job2 = LinkJob {
        desired: InstallSource::FileBytes(b"world".to_vec()),
        ..job
    };
    apply_job(&job2, &store, ApplyOptions { force: true }).unwrap();

    let s = std::fs::read_to_string(&target_path).unwrap();
    assert_eq!(s, "world");
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

#[test]
fn symlink_file_apply_should_be_idempotent() {
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

    apply_job(&job, &store, ApplyOptions { force: false }).unwrap();
    apply_job(&job, &store, ApplyOptions { force: false }).unwrap();

    let rendered_path = store.rendered_file_path(
        &ArtifactId::new_unchecked("a"),
        &ProfileName::new_unchecked("p"),
    );
    let target = temp.child("out.txt").path().to_path_buf();
    assert!(is_correct_symlink(&target, &rendered_path).unwrap());
}

#[test]
fn symlink_file_force_should_replace_existing_file() {
    let temp = assert_fs::TempDir::new().unwrap();
    let store = RenderStore::new(temp.child("cache").path().to_path_buf(), temp.path());

    temp.child("out.txt").write_str("old").unwrap();

    let job = LinkJob {
        target: TargetName::new_unchecked("t"),
        artifact_id: ArtifactId::new_unchecked("a"),
        profile: ProfileName::new_unchecked("p"),
        artifact_kind: ArtifactKind::File,
        method: InstallMethod::Symlink,
        target_path: temp.child("out.txt").path().to_path_buf(),
        desired: InstallSource::FileBytes(b"hello".to_vec()),
    };

    apply_job(&job, &store, ApplyOptions { force: true }).unwrap();

    let rendered_path = store.rendered_file_path(
        &ArtifactId::new_unchecked("a"),
        &ProfileName::new_unchecked("p"),
    );
    let target = temp.child("out.txt").path().to_path_buf();
    assert!(is_correct_symlink(&target, &rendered_path).unwrap());
}

#[test]
fn copy_dir_force_should_overwrite_existing_dir() {
    let temp = assert_fs::TempDir::new().unwrap();
    let store = RenderStore::new(temp.child("cache").path().to_path_buf(), temp.path());

    temp.child("src").create_dir_all().unwrap();
    temp.child("src/a.txt").write_str("v1").unwrap();

    let job = LinkJob {
        target: TargetName::new_unchecked("t"),
        artifact_id: ArtifactId::new_unchecked("a"),
        profile: ProfileName::new_unchecked("p"),
        artifact_kind: ArtifactKind::Dir,
        method: InstallMethod::Copy,
        target_path: temp.child("dst").path().to_path_buf(),
        desired: InstallSource::Path(temp.child("src").path().to_path_buf()),
    };

    apply_job(&job, &store, ApplyOptions { force: false }).unwrap();
    assert_eq!(
        std::fs::read_to_string(temp.child("dst/a.txt").path()).unwrap(),
        "v1"
    );

    temp.child("src/a.txt").write_str("v2").unwrap();
    apply_job(&job, &store, ApplyOptions { force: true }).unwrap();
    assert_eq!(
        std::fs::read_to_string(temp.child("dst/a.txt").path()).unwrap(),
        "v2"
    );
}

#[cfg(windows)]
#[test]
fn junction_health_check_should_work() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("src").create_dir_all().unwrap();
    temp.child("dst").create_dir_all().unwrap();

    let src = temp.child("src").path().to_path_buf();
    let junction_path = temp.child("dst/j").path().to_path_buf();
    junction::create(&src, &junction_path).unwrap();

    assert!(check_junction(&junction_path, &src).unwrap());
}
