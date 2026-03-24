use super::{artifact_id, job_profile};
use crate::{
    ApplyOptions, InstallSource, RenderStore, apply_job, build_link_instance_record,
    build_link_job_from_manifest, check_link_job_health, check_link_record_health,
};
use agentstow_core::{ArtifactKind, InstallMethod};
use agentstow_manifest::Manifest;
use assert_fs::prelude::*;
use pretty_assertions::assert_eq;
use time::OffsetDateTime;

#[test]
fn build_link_job_from_manifest_should_render_template_dir_into_render_store() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("templates/agents/skills/shared-guidance")
        .create_dir_all()
        .unwrap();
    temp.child("templates/agents/worker.toml.tera")
        .write_str("name = \"{{ project_name }}\"\n")
        .unwrap();
    temp.child("templates/agents/skills/shared-guidance/SKILL.md")
        .write_str("shared guidance")
        .unwrap();
    temp.child("agentstow.toml")
        .write_str(
            r#"
[profiles.base]
vars = { project_name = "demo" }

[artifacts.agents_dir]
kind = "dir"
source = "templates/agents"
template = true

[targets.agents_dir]
artifact = "agents_dir"
profile = "base"
target_path = "proj/.agents"
method = "symlink"
"#,
        )
        .unwrap();

    let manifest = Manifest::load_from_path(temp.child("agentstow.toml").path()).unwrap();
    let render_store = RenderStore::new(temp.child("cache").path().to_path_buf(), temp.path());
    let (target_name, target) = manifest.targets.iter().next().unwrap();

    let job = build_link_job_from_manifest(
        &manifest,
        target_name,
        target,
        &job_profile("base"),
        &render_store,
    )
    .unwrap();

    assert_eq!(job.artifact_kind, ArtifactKind::Dir);
    let InstallSource::Path(rendered_dir) = &job.desired else {
        panic!("expected rendered dir path");
    };
    assert_eq!(
        std::fs::read_to_string(rendered_dir.join("worker.toml")).unwrap(),
        "name = \"demo\"\n"
    );
    assert_eq!(
        std::fs::read_to_string(rendered_dir.join("skills/shared-guidance/SKILL.md")).unwrap(),
        "shared guidance"
    );
}

#[test]
fn build_link_instance_record_should_preserve_rendered_path_for_template_dir_copy() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("templates/agents").create_dir_all().unwrap();
    temp.child("templates/agents/worker.toml.tera")
        .write_str("name = \"{{ project_name }}\"\n")
        .unwrap();
    temp.child("agentstow.toml")
        .write_str(
            r#"
[profiles.base]
vars = { project_name = "demo" }

[artifacts.agents_dir]
kind = "dir"
source = "templates/agents"
template = true

[targets.agents_dir]
artifact = "agents_dir"
profile = "base"
target_path = "proj/.agents"
method = "copy"
"#,
        )
        .unwrap();

    let manifest = Manifest::load_from_path(temp.child("agentstow.toml").path()).unwrap();
    let render_store = RenderStore::new(temp.child("cache").path().to_path_buf(), temp.path());
    let (target_name, target) = manifest.targets.iter().next().unwrap();
    let job = build_link_job_from_manifest(
        &manifest,
        target_name,
        target,
        &job_profile("base"),
        &render_store,
    )
    .unwrap();

    let record =
        build_link_instance_record(&manifest, &job, &render_store, OffsetDateTime::UNIX_EPOCH)
            .unwrap();

    assert_eq!(record.method, InstallMethod::Copy);
    assert_eq!(
        record.rendered_path,
        Some(render_store.rendered_dir_path(&artifact_id("agents_dir"), &job_profile("base")))
    );
}

#[test]
fn check_link_job_health_should_report_rendered_dir_symlink_as_healthy() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("templates/agents/skills/shared-guidance")
        .create_dir_all()
        .unwrap();
    temp.child("templates/agents/worker.toml.tera")
        .write_str("name = \"{{ project_name }}\"\n")
        .unwrap();
    temp.child("templates/agents/skills/shared-guidance/SKILL.md")
        .write_str("shared guidance")
        .unwrap();
    temp.child("agentstow.toml")
        .write_str(
            r#"
[profiles.base]
vars = { project_name = "demo" }

[artifacts.agents_dir]
kind = "dir"
source = "templates/agents"
template = true

[targets.agents_dir]
artifact = "agents_dir"
profile = "base"
target_path = "proj/.agents"
method = "symlink"
"#,
        )
        .unwrap();

    let manifest = Manifest::load_from_path(temp.child("agentstow.toml").path()).unwrap();
    let store = RenderStore::new(temp.child("cache").path().to_path_buf(), temp.path());
    let (target_name, target) = manifest.targets.iter().next().unwrap();
    let job =
        build_link_job_from_manifest(&manifest, target_name, target, &job_profile("base"), &store)
            .unwrap();

    assert!(!check_link_job_health(&job, &store).unwrap());

    apply_job(&job, &store, ApplyOptions { force: false }).unwrap();
    assert!(check_link_job_health(&job, &store).unwrap());
}

#[test]
fn check_link_record_health_should_use_rendered_path_for_template_dir_copy() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("templates/agents/skills/shared-guidance")
        .create_dir_all()
        .unwrap();
    temp.child("templates/agents/worker.toml.tera")
        .write_str("name = \"{{ project_name }}\"\n")
        .unwrap();
    temp.child("templates/agents/skills/shared-guidance/SKILL.md.tera")
        .write_str("hello {{ project_name }}")
        .unwrap();
    temp.child("agentstow.toml")
        .write_str(
            r#"
[profiles.base]
vars = { project_name = "demo" }

[artifacts.agents_dir]
kind = "dir"
source = "templates/agents"
template = true

[targets.agents_dir]
artifact = "agents_dir"
profile = "base"
target_path = "proj/.agents"
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

    std::fs::remove_file(temp.child("proj/.agents/worker.toml").path()).unwrap();
    assert!(!check_link_record_health(&manifest, &record).unwrap());
}
