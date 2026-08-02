//! `SegmentRepository` 的 SQLite 实现。

use crate::schema::{
    CREATE_SEGMENT_LABEL_HISTORY_TABLE, CREATE_SEGMENT_TABLE, CREATE_VIDEO_LIST_TABLE,
    CREATE_VIDEO_NOTE_TABLE, CREATE_VIDEO_TABLE,
};
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct SqliteSegmentRepository {
    connection: Option<Connection>,
}

impl SqliteSegmentRepository {
    pub fn open_default() -> els_types::AppResult<Self> {
        Self::open_path(default_database_path())
    }

    pub fn open_path(path: impl AsRef<Path>) -> els_types::AppResult<Self> {
        let connection = Connection::open(path).map_err(sqlite_error)?;
        ensure_schema(&connection)?;
        Ok(Self {
            connection: Some(connection),
        })
    }

    pub fn disabled() -> Self {
        Self { connection: None }
    }

    fn connection(&self) -> els_types::AppResult<&Connection> {
        self.connection
            .as_ref()
            .ok_or_else(|| els_types::AppError::Io("segment repository is unavailable".to_string()))
    }

    fn connection_mut(&mut self) -> els_types::AppResult<&mut Connection> {
        self.connection
            .as_mut()
            .ok_or_else(|| els_types::AppError::Io("segment repository is unavailable".to_string()))
    }
}

impl Default for SqliteSegmentRepository {
    fn default() -> Self {
        match Self::open_default() {
            Ok(repository) => repository,
            Err(err) => {
                eprintln!("Failed to initialize segment repository: {err}");
                Self::disabled()
            }
        }
    }
}

impl els_learning_core::SegmentRepository for SqliteSegmentRepository {
    fn ensure_video(
        &mut self,
        path: &str,
        title: &str,
        duration_secs: f64,
    ) -> els_types::AppResult<i64> {
        let connection = self.connection()?;
        connection
            .execute(
                "INSERT INTO video (path, title, duration) VALUES (?1, ?2, ?3)
                 ON CONFLICT(path) DO UPDATE SET
                    title = excluded.title,
                    duration = excluded.duration",
                params![path, title, duration_secs],
            )
            .map_err(sqlite_error)?;
        connection
            .query_row(
                "SELECT id FROM video WHERE path = ?1",
                params![path],
                |row| row.get(0),
            )
            .map_err(sqlite_error)
    }

    fn save(&mut self, segment: &els_learning_core::Segment) -> els_types::AppResult<i64> {
        let connection = self.connection()?;
        if let Some(id) = segment.id {
            connection
                .execute(
                    "UPDATE segment SET
                        start_time = ?1,
                        end_time = ?2,
                        repeat_count = ?3,
                        interval_seconds = ?4,
                        label = ?5
                     WHERE id = ?6 AND video_id = ?7",
                    params![
                        segment.range.start,
                        segment.range.end,
                        segment.repeat_count,
                        segment.interval_seconds,
                        &segment.label,
                        id,
                        segment.video_id,
                    ],
                )
                .map_err(sqlite_error)?;
            return Ok(id);
        }

        connection
            .execute(
                "INSERT INTO segment (
                    video_id, start_time, end_time, repeat_count,
                    interval_seconds, completed_loops, label
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    segment.video_id,
                    segment.range.start,
                    segment.range.end,
                    segment.repeat_count,
                    segment.interval_seconds,
                    segment.completed_loops,
                    &segment.label,
                ],
            )
            .map_err(sqlite_error)?;
        Ok(connection.last_insert_rowid())
    }

    fn find_by_video(
        &self,
        video_id: i64,
    ) -> els_types::AppResult<Vec<els_learning_core::Segment>> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT id, start_time, end_time, repeat_count,
                        interval_seconds, completed_loops, label
                 FROM segment
                 WHERE video_id = ?1
                 ORDER BY start_time ASC, end_time ASC, id ASC",
            )
            .map_err(sqlite_error)?;
        let rows = statement
            .query_map(params![video_id], |row| {
                Ok(els_learning_core::Segment {
                    id: Some(row.get(0)?),
                    video_id,
                    range: els_types::TimeRange {
                        start: row.get(1)?,
                        end: row.get(2)?,
                    },
                    repeat_count: row.get(3)?,
                    interval_seconds: row.get(4)?,
                    completed_loops: row.get(5)?,
                    label: row.get(6)?,
                })
            })
            .map_err(sqlite_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(sqlite_error)
    }

    fn delete(&mut self, segment_id: i64) -> els_types::AppResult<()> {
        self.connection()?
            .execute("DELETE FROM segment WHERE id = ?1", params![segment_id])
            .map_err(sqlite_error)?;
        Ok(())
    }

    fn set_labels(
        &mut self,
        segment_ids: &[i64],
        video_id: i64,
        label: &str,
    ) -> els_types::AppResult<()> {
        let transaction = self.connection_mut()?.transaction().map_err(sqlite_error)?;
        for segment_id in segment_ids {
            let changed = transaction
                .execute(
                    "UPDATE segment SET label = ?1 WHERE id = ?2 AND video_id = ?3",
                    params![label, segment_id, video_id],
                )
                .map_err(sqlite_error)?;
            if changed == 0 {
                return Err(els_types::AppError::NotFound);
            }
        }

        if !label.is_empty() {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
                .min(i64::MAX as u128) as i64;
            let latest: i64 = transaction
                .query_row(
                    "SELECT COALESCE(MAX(last_used_at), 0)
                     FROM segment_label_history WHERE video_id = ?1",
                    params![video_id],
                    |row| row.get(0),
                )
                .map_err(sqlite_error)?;
            let last_used_at = now.max(latest.saturating_add(1));
            transaction
                .execute(
                    "INSERT INTO segment_label_history (video_id, label, last_used_at)
                     VALUES (?1, ?2, ?3)
                     ON CONFLICT(video_id, label) DO UPDATE SET
                        last_used_at = excluded.last_used_at",
                    params![video_id, label, last_used_at],
                )
                .map_err(sqlite_error)?;
        }

        transaction.commit().map_err(sqlite_error)
    }

    fn find_recent_labels(&self, video_id: i64, limit: usize) -> els_types::AppResult<Vec<String>> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let mut statement = self
            .connection()?
            .prepare(
                "SELECT label FROM segment_label_history
                 WHERE video_id = ?1
                 ORDER BY last_used_at DESC, label ASC
                 LIMIT ?2",
            )
            .map_err(sqlite_error)?;
        let rows = statement
            .query_map(
                params![video_id, limit.min(i64::MAX as usize) as i64],
                |row| row.get(0),
            )
            .map_err(sqlite_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(sqlite_error)
    }

    fn increment_completed_loops(
        &mut self,
        segment_id: i64,
    ) -> els_types::AppResult<els_learning_core::Progress> {
        self.connection()?
            .query_row(
                "UPDATE segment
                 SET completed_loops = completed_loops + 1
                 WHERE id = ?1
                 RETURNING completed_loops, repeat_count",
                params![segment_id],
                |row| {
                    Ok(els_learning_core::Progress {
                        completed_loops: row.get(0)?,
                        total_loops: row.get(1)?,
                    })
                },
            )
            .map_err(sqlite_error)
    }

    fn increment_completed_loops_many(&mut self, segment_ids: &[i64]) -> els_types::AppResult<()> {
        if segment_ids.is_empty() {
            return Ok(());
        }
        let transaction = self.connection_mut()?.transaction().map_err(sqlite_error)?;
        for segment_id in segment_ids {
            let changed = transaction
                .execute(
                    "UPDATE segment SET completed_loops = completed_loops + 1 WHERE id = ?1",
                    params![segment_id],
                )
                .map_err(sqlite_error)?;
            if changed == 0 {
                return Err(els_types::AppError::NotFound);
            }
        }
        transaction.commit().map_err(sqlite_error)
    }
}

pub(crate) fn ensure_schema(connection: &Connection) -> els_types::AppResult<()> {
    connection
        .execute_batch(CREATE_VIDEO_TABLE)
        .map_err(sqlite_error)?;
    connection
        .execute_batch(CREATE_VIDEO_LIST_TABLE)
        .map_err(sqlite_error)?;
    migrate_video_list_status(connection)?;
    connection
        .execute_batch(CREATE_SEGMENT_TABLE)
        .map_err(sqlite_error)?;
    connection
        .execute_batch(CREATE_SEGMENT_LABEL_HISTORY_TABLE)
        .map_err(sqlite_error)?;
    connection
        .execute_batch(CREATE_VIDEO_NOTE_TABLE)
        .map_err(sqlite_error)?;

    for migration in [
        "ALTER TABLE video ADD COLUMN learning_status TEXT NOT NULL DEFAULT 'in_progress'",
        "ALTER TABLE video ADD COLUMN last_opened_at INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE video ADD COLUMN last_position REAL NOT NULL DEFAULT 0",
        "ALTER TABLE video ADD COLUMN list_id INTEGER",
        "ALTER TABLE segment ADD COLUMN interval_seconds INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE segment ADD COLUMN completed_loops INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE segment ADD COLUMN label TEXT NOT NULL DEFAULT ''",
    ] {
        if let Err(err) = connection.execute(migration, []) {
            if !err.to_string().contains("duplicate column name") {
                return Err(sqlite_error(err));
            }
        }
    }
    connection
        .execute(
            "INSERT OR IGNORE INTO segment_label_history (video_id, label, last_used_at)
             SELECT video_id, label, id FROM segment WHERE TRIM(label) <> ''",
            [],
        )
        .map_err(sqlite_error)?;
    Ok(())
}

fn migrate_video_list_status(connection: &Connection) -> els_types::AppResult<()> {
    let has_status_column = connection
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM pragma_table_info('video_list')
                WHERE name = 'learning_status'
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(sqlite_error)?;
    if has_status_column {
        return Ok(());
    }

    let transaction = connection.unchecked_transaction().map_err(sqlite_error)?;
    transaction
        .execute_batch("ALTER TABLE video_list RENAME TO video_list_legacy;")
        .map_err(sqlite_error)?;
    transaction
        .execute_batch(CREATE_VIDEO_LIST_TABLE)
        .map_err(sqlite_error)?;
    transaction
        .execute(
            "INSERT INTO video_list (id, name, learning_status, created_at)
             SELECT id, name, 'in_progress', created_at FROM video_list_legacy",
            [],
        )
        .map_err(sqlite_error)?;
    transaction
        .execute_batch("DROP TABLE video_list_legacy;")
        .map_err(sqlite_error)?;
    transaction.commit().map_err(sqlite_error)
}

pub(crate) fn default_database_path() -> PathBuf {
    if let Ok(path) = std::env::var("ELS_DB_PATH") {
        return PathBuf::from(path);
    }
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("english-learning-studio.sqlite3")
}

pub(crate) fn sqlite_error(error: rusqlite::Error) -> els_types::AppError {
    els_types::AppError::Io(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::SqliteSegmentRepository;
    use els_learning_core::SegmentRepository;

    #[test]
    fn persists_lists_updates_and_deletes_segments() {
        let db_path = std::env::temp_dir().join(format!(
            "els-segments-{}-{}.sqlite3",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock")
                .as_nanos()
        ));
        let mut repository = SqliteSegmentRepository::open_path(&db_path).expect("open repository");
        let video_id = repository
            .ensure_video("/tmp/video.mp4", "video.mp4", 60.0)
            .expect("ensure video");
        let id = repository
            .save(&els_learning_core::Segment {
                id: None,
                video_id,
                range: els_types::TimeRange {
                    start: 10.0,
                    end: 20.0,
                },
                repeat_count: 3,
                interval_seconds: 2,
                completed_loops: 0,
                label: "场景 1".to_string(),
            })
            .expect("save segment");

        repository
            .set_labels(&[id], video_id, "对话")
            .expect("set first recent label");
        repository
            .set_labels(&[id], video_id, "场景 1")
            .expect("set latest recent label");
        assert_eq!(
            repository
                .find_recent_labels(video_id, 10)
                .expect("list recent labels"),
            vec!["场景 1".to_string(), "对话".to_string()]
        );
        repository
            .set_labels(&[id], video_id, "")
            .expect("clear label without clearing history");
        assert_eq!(
            repository
                .find_recent_labels(video_id, 1)
                .expect("list most recent label"),
            vec!["场景 1".to_string()]
        );

        repository
            .increment_completed_loops(id)
            .expect("increment first loop");
        let progress = repository
            .increment_completed_loops(id)
            .expect("increment second loop");
        assert_eq!(progress.completed_loops, 2);
        assert_eq!(progress.total_loops, 3);

        repository
            .save(&els_learning_core::Segment {
                id: Some(id),
                video_id,
                range: els_types::TimeRange {
                    start: 11.5,
                    end: 22.0,
                },
                repeat_count: 5,
                interval_seconds: 1,
                completed_loops: 0,
                label: "场景 1".to_string(),
            })
            .expect("update settings without resetting history");
        let segments = repository.find_by_video(video_id).expect("list segments");
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].completed_loops, 2);
        assert_eq!(segments[0].repeat_count, 5);
        assert_eq!(segments[0].range.start, 11.5);
        assert_eq!(segments[0].label, "场景 1");
        assert_eq!(segments[0].range.end, 22.0);

        drop(repository);
        let mut repository =
            SqliteSegmentRepository::open_path(&db_path).expect("reopen repository");
        let persisted = repository.find_by_video(video_id).expect("reload segments");
        assert_eq!(persisted[0].completed_loops, 2);
        assert_eq!(
            repository
                .find_recent_labels(video_id, 10)
                .expect("reload recent labels"),
            vec!["场景 1".to_string(), "对话".to_string()]
        );

        repository
            .increment_completed_loops_many(&[id])
            .expect("increment grouped playback loop");
        assert_eq!(
            repository
                .find_by_video(video_id)
                .expect("reload grouped progress")[0]
                .completed_loops,
            3
        );

        let earlier_id = repository
            .save(&els_learning_core::Segment {
                id: None,
                video_id,
                range: els_types::TimeRange {
                    start: 2.0,
                    end: 4.0,
                },
                repeat_count: 2,
                interval_seconds: 0,
                completed_loops: 0,
                label: String::new(),
            })
            .expect("save earlier segment");
        let ordered = repository
            .find_by_video(video_id)
            .expect("list segments by time");
        assert_eq!(ordered.len(), 2);
        assert_eq!(ordered[0].id, Some(earlier_id));
        assert_eq!(ordered[1].id, Some(id));

        repository
            .set_labels(&[id, earlier_id], video_id, "批量标记")
            .expect("set label for multiple segments");
        assert!(repository
            .find_by_video(video_id)
            .expect("reload batch labels")
            .iter()
            .all(|segment| segment.label == "批量标记"));
        assert!(repository
            .set_labels(&[id, -1], video_id, "不应保存")
            .is_err());
        assert!(repository
            .find_by_video(video_id)
            .expect("verify batch rollback")
            .iter()
            .all(|segment| segment.label == "批量标记"));

        repository
            .delete(earlier_id)
            .expect("delete earlier segment");
        repository.delete(id).expect("delete segment");
        assert!(repository
            .find_by_video(video_id)
            .expect("list empty")
            .is_empty());

        drop(repository);
        let _ = std::fs::remove_file(&db_path);
    }
}
