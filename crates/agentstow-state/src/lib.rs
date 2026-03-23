use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use agentstow_core::{
    AgentStowDirs, AgentStowError, ArtifactId, InstallMethod, ProfileName, Result,
    normalize_for_display,
};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tracing::instrument;

const SCHEMA_VERSION: i32 = 1;

#[derive(Debug)]
pub struct StateDb {
    db_path: PathBuf,
    conn: Connection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkInstanceRecord {
    pub workspace_root: PathBuf,
    pub artifact_id: ArtifactId,
    pub profile: ProfileName,
    pub target_path: PathBuf,
    pub method: InstallMethod,
    pub rendered_path: Option<PathBuf>,
    pub blake3: Option<String>,
    pub updated_at: OffsetDateTime,
}

impl StateDb {
    pub fn open(dirs: &AgentStowDirs) -> Result<Self> {
        dirs.ensure_dirs()?;
        let db_path = dirs.data_dir.join("agentstow.db");
        let conn = Connection::open(&db_path).map_err(|e| AgentStowError::State {
            message: format!(
                "打开 sqlite 失败：{e}; path={}",
                normalize_for_display(&db_path)
            )
            .into(),
        })?;
        conn.busy_timeout(Duration::from_secs(5))
            .map_err(|e| AgentStowError::State {
                message: format!("配置 sqlite busy timeout 失败：{e}").into(),
            })?;
        let db = Self { db_path, conn };
        db.init()?;
        Ok(db)
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    fn init(&self) -> Result<()> {
        self.conn
            .execute_batch(
                r#"
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
"#,
            )
            .map_err(|e| AgentStowError::State {
                message: format!("初始化 sqlite pragma 失败：{e}").into(),
            })?;

        let user_version = self
            .conn
            .query_row("PRAGMA user_version;", [], |row| row.get::<_, i32>(0))
            .unwrap_or(0);

        // 开发期策略：不做迁移，schema 变更允许直接删库重建。
        // 为避免旧 schema 残留导致运行时错误：当 user_version 不匹配时，直接 drop + recreate。
        let schema_sql: String = if user_version == SCHEMA_VERSION {
            r#"
BEGIN;

CREATE TABLE IF NOT EXISTS link_instances (
  workspace_root TEXT NOT NULL,
  target_path    TEXT NOT NULL,
  artifact_id    TEXT NOT NULL,
  profile        TEXT NOT NULL,
  method         TEXT NOT NULL,
  rendered_path  TEXT,
  blake3         TEXT,
  updated_at     TEXT NOT NULL,

  PRIMARY KEY (workspace_root, target_path)
);

CREATE INDEX IF NOT EXISTS idx_link_instances_artifact
  ON link_instances (workspace_root, artifact_id);

COMMIT;
"#
            .to_string()
        } else {
            // 破坏性重建：不保留旧数据
            format!(
                r#"
BEGIN;

DROP TABLE IF EXISTS link_instances;
DROP INDEX IF EXISTS idx_link_instances_artifact;

CREATE TABLE link_instances (
  workspace_root TEXT NOT NULL,
  target_path    TEXT NOT NULL,
  artifact_id    TEXT NOT NULL,
  profile        TEXT NOT NULL,
  method         TEXT NOT NULL,
  rendered_path  TEXT,
  blake3         TEXT,
  updated_at     TEXT NOT NULL,

  PRIMARY KEY (workspace_root, target_path)
);

CREATE INDEX idx_link_instances_artifact
  ON link_instances (workspace_root, artifact_id);

PRAGMA user_version = {SCHEMA_VERSION};
COMMIT;
"#
            )
        };

        self.conn
            .execute_batch(&schema_sql)
            .map_err(|e| AgentStowError::State {
                message: format!("初始化 sqlite schema 失败：{e}").into(),
            })?;
        Ok(())
    }

    #[instrument(skip_all, fields(workspace_root=%normalize_for_display(&record.workspace_root), target_path=%normalize_for_display(&record.target_path)))]
    pub fn upsert_link_instance(&self, record: &LinkInstanceRecord) -> Result<()> {
        self.prune_overlapping_link_instances(record)?;
        self.conn
            .execute(
                r#"
INSERT INTO link_instances (
  workspace_root,
  target_path,
  artifact_id,
  profile,
  method,
  rendered_path,
  blake3,
  updated_at
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
ON CONFLICT(workspace_root, target_path) DO UPDATE SET
  artifact_id = excluded.artifact_id,
  profile = excluded.profile,
  method = excluded.method,
  rendered_path = excluded.rendered_path,
  blake3 = excluded.blake3,
  updated_at = excluded.updated_at
"#,
                params![
                    record.workspace_root.to_string_lossy(),
                    record.target_path.to_string_lossy(),
                    record.artifact_id.as_str(),
                    record.profile.as_str(),
                    format!("{:?}", record.method),
                    record
                        .rendered_path
                        .as_ref()
                        .map(|p| p.to_string_lossy().to_string()),
                    record.blake3.as_deref(),
                    record
                        .updated_at
                        .format(&time::format_description::well_known::Rfc3339)
                        .unwrap_or_default(),
                ],
            )
            .map_err(|e| AgentStowError::State {
                message: format!("写入 link instance 失败：{e}").into(),
            })?;
        Ok(())
    }

    fn prune_overlapping_link_instances(&self, record: &LinkInstanceRecord) -> Result<()> {
        let existing = self.list_link_instances(&record.workspace_root)?;
        for item in existing {
            if item.target_path == record.target_path {
                continue;
            }
            if paths_overlap(&item.target_path, &record.target_path) {
                self.conn
                    .execute(
                        "DELETE FROM link_instances WHERE workspace_root = ?1 AND target_path = ?2",
                        params![
                            record.workspace_root.to_string_lossy(),
                            item.target_path.to_string_lossy()
                        ],
                    )
                    .map_err(|e| AgentStowError::State {
                        message: format!("删除重叠 link instance 失败：{e}").into(),
                    })?;
            }
        }
        Ok(())
    }

    pub fn list_link_instances(&self, workspace_root: &Path) -> Result<Vec<LinkInstanceRecord>> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
SELECT workspace_root, target_path, artifact_id, profile, method, rendered_path, blake3, updated_at
FROM link_instances
WHERE workspace_root = ?1
ORDER BY target_path
"#,
            )
            .map_err(|e| AgentStowError::State {
                message: format!("prepare 失败：{e}").into(),
            })?;

        let rows = stmt
            .query_map([workspace_root.to_string_lossy().to_string()], |row| {
                let workspace_root: String = row.get(0)?;
                let target_path: String = row.get(1)?;
                let artifact_id: String = row.get(2)?;
                let profile: String = row.get(3)?;
                let method: String = row.get(4)?;
                let rendered_path: Option<String> = row.get(5)?;
                let blake3: Option<String> = row.get(6)?;
                let updated_at: String = row.get(7)?;

                let updated_at = OffsetDateTime::parse(
                    &updated_at,
                    &time::format_description::well_known::Rfc3339,
                )
                .unwrap_or(OffsetDateTime::UNIX_EPOCH);

                Ok(LinkInstanceRecord {
                    workspace_root: PathBuf::from(workspace_root),
                    artifact_id: ArtifactId::new_unchecked(artifact_id),
                    profile: ProfileName::new_unchecked(profile),
                    target_path: PathBuf::from(target_path),
                    method: parse_install_method(&method),
                    rendered_path: rendered_path.map(PathBuf::from),
                    blake3,
                    updated_at,
                })
            })
            .map_err(|e| AgentStowError::State {
                message: format!("query_map 失败：{e}").into(),
            })?;

        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(|e| AgentStowError::State {
                message: format!("row parse 失败：{e}").into(),
            })?);
        }
        Ok(out)
    }

    pub fn prune_link_instances_not_in(
        &self,
        workspace_root: &Path,
        keep_target_paths: &HashSet<PathBuf>,
    ) -> Result<()> {
        let existing = self.list_link_instances(workspace_root)?;
        for item in existing {
            if keep_target_paths.contains(&item.target_path) {
                continue;
            }
            self.conn
                .execute(
                    "DELETE FROM link_instances WHERE workspace_root = ?1 AND target_path = ?2",
                    params![
                        workspace_root.to_string_lossy(),
                        item.target_path.to_string_lossy(),
                    ],
                )
                .map_err(|e| AgentStowError::State {
                    message: format!("删除过期 link instance 失败：{e}").into(),
                })?;
        }
        Ok(())
    }
}

fn parse_install_method(s: &str) -> InstallMethod {
    match s {
        "Symlink" => InstallMethod::Symlink,
        "Junction" => InstallMethod::Junction,
        "Copy" => InstallMethod::Copy,
        _ => InstallMethod::Symlink,
    }
}

fn paths_overlap(left: &Path, right: &Path) -> bool {
    is_prefix_path(left, right) || is_prefix_path(right, left)
}

fn is_prefix_path(prefix: &Path, path: &Path) -> bool {
    let mut prefix_components = prefix.components();
    let mut path_components = path.components();
    loop {
        match (prefix_components.next(), path_components.next()) {
            (None, _) => return true,
            (Some(_), None) => return false,
            (Some(left), Some(right)) if left == right => continue,
            _ => return false,
        }
    }
}

#[cfg(test)]
mod tests;
