use crate::health::is_correct_symlink;
use agentstow_core::{ArtifactId, ArtifactKind, InstallMethod, ProfileName, TargetName};
use agentstow_manifest::Manifest;
use assert_fs::prelude::*;
use pretty_assertions::assert_eq;
use std::path::PathBuf;
use time::OffsetDateTime;

use super::*;

mod rendered_dir;

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
fn symlink_file_health_should_handle_relative_parent_segments() {
    let temp = assert_fs::TempDir::new().unwrap();
    let store = RenderStore::new(temp.child("cache").path().to_path_buf(), temp.path());
    temp.child("nested/deeper").create_dir_all().unwrap();

    let job = LinkJob {
        target: TargetName::new_unchecked("t"),
        artifact_id: ArtifactId::new_unchecked("a"),
        profile: ProfileName::new_unchecked("p"),
        artifact_kind: ArtifactKind::File,
        method: InstallMethod::Symlink,
        target_path: temp.child("nested/deeper/out.txt").path().to_path_buf(),
        desired: InstallSource::FileBytes(b"hello".to_vec()),
    };

    apply_job(&job, &store, ApplyOptions { force: false }).unwrap();

    let rendered_path = store.rendered_file_path(
        &ArtifactId::new_unchecked("a"),
        &ProfileName::new_unchecked("p"),
    );
    let target = temp.child("nested/deeper/out.txt").path().to_path_buf();
    assert!(check_symlink(&target, &rendered_path).unwrap());
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
fn check_symlink_should_reject_indirect_symlink_target() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("cache/rendered").create_dir_all().unwrap();
    temp.child("cache/rendered/config.toml")
        .write_str("hello")
        .unwrap();
    temp.child("links").create_dir_all().unwrap();

    #[cfg(unix)]
    std::os::unix::fs::symlink(
        "../cache/rendered/config.toml",
        temp.child("links/direct.txt").path(),
    )
    .unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(
        temp.child("cache/rendered/config.toml").path(),
        temp.child("links/direct.txt").path(),
    )
    .unwrap();

    #[cfg(unix)]
    std::os::unix::fs::symlink("../links/direct.txt", temp.child("out.txt").path()).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(
        temp.child("links/direct.txt").path(),
        temp.child("out.txt").path(),
    )
    .unwrap();

    assert!(
        !check_symlink(
            temp.child("out.txt").path(),
            temp.child("cache/rendered/config.toml").path(),
        )
        .unwrap()
    );
}

#[test]
fn symlink_dir_force_should_replace_indirect_symlink_with_direct_target() {
    let temp = assert_fs::TempDir::new().unwrap();
    let store = RenderStore::new(temp.child("cache").path().to_path_buf(), temp.path());
    temp.child("source-dir/sub").create_dir_all().unwrap();
    temp.child("source-dir/sub/rule.md")
        .write_str("hello")
        .unwrap();
    temp.child("existing").create_dir_all().unwrap();
    temp.child("proj").create_dir_all().unwrap();

    #[cfg(unix)]
    std::os::unix::fs::symlink("../source-dir", temp.child("existing/direct").path()).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(
        temp.child("source-dir").path(),
        temp.child("existing/direct").path(),
    )
    .unwrap();

    #[cfg(unix)]
    std::os::unix::fs::symlink("../existing/direct", temp.child("proj/.agents").path()).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(
        temp.child("existing/direct").path(),
        temp.child("proj/.agents").path(),
    )
    .unwrap();

    let job = LinkJob {
        target: TargetName::new_unchecked("t"),
        artifact_id: ArtifactId::new_unchecked("agents_dir"),
        profile: ProfileName::new_unchecked("base"),
        artifact_kind: ArtifactKind::Dir,
        method: InstallMethod::Symlink,
        target_path: temp.child("proj/.agents").path().to_path_buf(),
        desired: InstallSource::Path(temp.child("source-dir").path().to_path_buf()),
    };

    apply_job(&job, &store, ApplyOptions { force: true }).unwrap();

    assert!(check_symlink(&job.target_path, temp.child("source-dir").path()).unwrap());
    assert_ne!(
        fs_err::read_link(&job.target_path).unwrap(),
        PathBuf::from("../existing/direct")
    );
}

fn artifact_id(id: &str) -> ArtifactId {
    ArtifactId::new_unchecked(id)
}

fn job_profile(profile: &str) -> ProfileName {
    ProfileName::new_unchecked(profile)
}

#[test]
fn check_link_job_health_should_report_copy_file_drift() {
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

    assert!(!check_link_job_health(&job, &store).unwrap());

    apply_job(&job, &store, ApplyOptions { force: false }).unwrap();
    assert!(check_link_job_health(&job, &store).unwrap());

    temp.child("out.txt").write_str("drift").unwrap();
    assert!(!check_link_job_health(&job, &store).unwrap());
}

#[test]
fn check_link_record_health_should_report_template_copy_file_as_healthy() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("templates").create_dir_all().unwrap();
    temp.child("templates/AGENTS.md.tera")
        .write_str("# {{ project_name }}\n")
        .unwrap();
    temp.child("agentstow.toml")
        .write_str(
            r#"
[profiles.base]
vars = { project_name = "demo" }

[artifacts.agents]
kind = "file"
source = "templates/AGENTS.md.tera"
template = true
validate_as = "none"

[targets.agents]
artifact = "agents"
profile = "base"
target_path = "proj/AGENTS.md"
method = "copy"
"#,
        )
        .unwrap();

    let manifest = Manifest::load_from_path(temp.child("agentstow.toml").path()).unwrap();
    let store = RenderStore::new(temp.child("cache").path().to_path_buf(), temp.path());
    let (target_name, target) = manifest.targets.iter().next().unwrap();
    let job =
        build_link_job_from_manifest(&manifest, target_name, target, &job_profile("base"), &store)
            .unwrap();

    apply_job(&job, &store, ApplyOptions { force: false }).unwrap();
    let record =
        build_link_instance_record(&manifest, &job, &store, OffsetDateTime::UNIX_EPOCH).unwrap();

    assert!(check_link_record_health(&manifest, &record).unwrap());
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

#[test]
fn copy_dir_apply_should_be_idempotent_when_target_is_healthy() {
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
    apply_job(&job, &store, ApplyOptions { force: false }).unwrap();

    assert_eq!(
        std::fs::read_to_string(temp.child("dst/a.txt").path()).unwrap(),
        "v1"
    );
}

#[test]
fn check_copy_dir_should_fail_when_target_has_extra_file() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("src").create_dir_all().unwrap();
    temp.child("src/a.txt").write_str("ok").unwrap();
    temp.child("dst").create_dir_all().unwrap();
    temp.child("dst/a.txt").write_str("ok").unwrap();
    temp.child("dst/extra.txt").write_str("noise").unwrap();

    let ok = check_copy_dir(temp.child("dst").path(), temp.child("src").path()).unwrap();
    assert!(!ok);
}

#[test]
fn check_copy_dir_should_fail_when_nested_content_differs() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("src/nested").create_dir_all().unwrap();
    temp.child("src/nested/a.txt").write_str("ok").unwrap();
    temp.child("dst/nested").create_dir_all().unwrap();
    temp.child("dst/nested/a.txt").write_str("drift").unwrap();

    let ok = check_copy_dir(temp.child("dst").path(), temp.child("src").path()).unwrap();
    assert!(!ok);
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
