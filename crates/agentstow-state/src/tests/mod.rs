use assert_fs::prelude::*;
use pretty_assertions::assert_eq;

use super::*;

#[test]
fn upsert_and_list_should_work() {
    let temp = assert_fs::TempDir::new().unwrap();
    let home = temp.child("home");
    home.create_dir_all().unwrap();

    temp_env::with_var("AGENTSTOW_HOME", Some(home.path()), || {
        let dirs = AgentStowDirs::from_env().unwrap();
        let db = StateDb::open(&dirs).unwrap();

        let record = LinkInstanceRecord {
            workspace_root: temp.child("ws").path().to_path_buf(),
            artifact_id: ArtifactId::new_unchecked("a"),
            profile: ProfileName::new_unchecked("p"),
            target_path: temp.child("proj/AGENTS.md").path().to_path_buf(),
            method: InstallMethod::Symlink,
            rendered_path: Some(temp.child("cache/rendered").path().to_path_buf()),
            blake3: Some("deadbeef".to_string()),
            updated_at: OffsetDateTime::now_utc(),
        };
        db.upsert_link_instance(&record).unwrap();

        let listed = db.list_link_instances(&record.workspace_root).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].artifact_id.as_str(), "a");
    });
}

#[test]
fn schema_version_mismatch_should_reset_db() {
    let temp = assert_fs::TempDir::new().unwrap();
    let home = temp.child("home");
    home.create_dir_all().unwrap();

    temp_env::with_var("AGENTSTOW_HOME", Some(home.path()), || {
        let dirs = AgentStowDirs::from_env().unwrap();

        // 先写入一条记录
        {
            let db = StateDb::open(&dirs).unwrap();
            let record = LinkInstanceRecord {
                workspace_root: temp.child("ws").path().to_path_buf(),
                artifact_id: ArtifactId::new_unchecked("a"),
                profile: ProfileName::new_unchecked("p"),
                target_path: temp.child("proj/AGENTS.md").path().to_path_buf(),
                method: InstallMethod::Symlink,
                rendered_path: Some(temp.child("cache/rendered").path().to_path_buf()),
                blake3: Some("deadbeef".to_string()),
                updated_at: OffsetDateTime::now_utc(),
            };
            db.upsert_link_instance(&record).unwrap();

            // 模拟旧版本/未知版本
            db.conn.execute_batch("PRAGMA user_version = 999;").unwrap();
        }

        // reopen：应触发破坏性重建（清空旧数据）
        let db = StateDb::open(&dirs).unwrap();
        let records = db.list_link_instances(temp.child("ws").path()).unwrap();
        assert_eq!(records.len(), 0);

        let user_version: i32 = db
            .conn
            .query_row("PRAGMA user_version;", [], |row| row.get(0))
            .unwrap();
        assert_eq!(user_version, super::SCHEMA_VERSION);
    });
}

#[test]
fn upsert_should_replace_existing_row_for_same_target() {
    let temp = assert_fs::TempDir::new().unwrap();
    let home = temp.child("home");
    home.create_dir_all().unwrap();

    temp_env::with_var("AGENTSTOW_HOME", Some(home.path()), || {
        let dirs = AgentStowDirs::from_env().unwrap();
        let db = StateDb::open(&dirs).unwrap();
        let target_path = temp.child("proj/AGENTS.md").path().to_path_buf();

        db.upsert_link_instance(&LinkInstanceRecord {
            workspace_root: temp.child("ws").path().to_path_buf(),
            artifact_id: ArtifactId::new_unchecked("a"),
            profile: ProfileName::new_unchecked("p"),
            target_path: target_path.clone(),
            method: InstallMethod::Symlink,
            rendered_path: Some(temp.child("cache/v1").path().to_path_buf()),
            blake3: Some("one".to_string()),
            updated_at: OffsetDateTime::now_utc(),
        })
        .unwrap();

        db.upsert_link_instance(&LinkInstanceRecord {
            workspace_root: temp.child("ws").path().to_path_buf(),
            artifact_id: ArtifactId::new_unchecked("b"),
            profile: ProfileName::new_unchecked("p2"),
            target_path: target_path.clone(),
            method: InstallMethod::Copy,
            rendered_path: None,
            blake3: Some("two".to_string()),
            updated_at: OffsetDateTime::now_utc(),
        })
        .unwrap();

        let listed = db.list_link_instances(temp.child("ws").path()).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].artifact_id.as_str(), "b");
        assert_eq!(listed[0].profile.as_str(), "p2");
        assert_eq!(listed[0].method, InstallMethod::Copy);
    });
}

#[test]
fn list_should_fallback_for_unknown_method_and_bad_timestamp() {
    let temp = assert_fs::TempDir::new().unwrap();
    let home = temp.child("home");
    home.create_dir_all().unwrap();

    temp_env::with_var("AGENTSTOW_HOME", Some(home.path()), || {
        let dirs = AgentStowDirs::from_env().unwrap();
        let db = StateDb::open(&dirs).unwrap();
        let workspace_root = temp.child("ws").path().to_path_buf();
        let target_path = temp.child("proj/AGENTS.md").path().to_path_buf();

        db.conn
            .execute(
                r#"
INSERT INTO link_instances (
  workspace_root, target_path, artifact_id, profile, method, rendered_path, blake3, updated_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
"#,
                rusqlite::params![
                    workspace_root.to_string_lossy(),
                    target_path.to_string_lossy(),
                    "a",
                    "p",
                    "WeirdMethod",
                    Option::<String>::None,
                    Option::<String>::None,
                    "not-a-timestamp",
                ],
            )
            .unwrap();

        let listed = db.list_link_instances(&workspace_root).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].method, InstallMethod::Symlink);
        assert_eq!(listed[0].updated_at, OffsetDateTime::UNIX_EPOCH);
    });
}
