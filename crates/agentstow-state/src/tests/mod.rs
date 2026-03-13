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
