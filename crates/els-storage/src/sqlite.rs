//! `SegmentRepository` 的 SQLite 实现。

use crate::schema::{CREATE_SEGMENT_TABLE, CREATE_VIDEO_LIST_TABLE, CREATE_VIDEO_TABLE};
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};

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
                        interval_seconds = ?4
                     WHERE id = ?5 AND video_id = ?6",
                    params![
                        segment.range.start,
                        segment.range.end,
                        segment.repeat_count,
                        segment.interval_seconds,
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
                    interval_seconds, completed_loops
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    segment.video_id,
                    segment.range.start,
                    segment.range.end,
                    segment.repeat_count,
                    segment.interval_seconds,
                    segment.completed_loops,
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
                        interval_seconds, completed_loops
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
}

pub(crate) fn ensure_schema(connection: &Connection) -> els_types::AppResult<()> {
    connection
        .execute_batch(CREATE_VIDEO_TABLE)
        .map_err(sqlite_error)?;
    connection
        .execute_batch(CREATE_VIDEO_LIST_TABLE)
        .map_err(sqlite_error)?;
    connection
        .execute_batch(CREATE_SEGMENT_TABLE)
        .map_err(sqlite_error)?;

    for migration in [
        "ALTER TABLE video ADD COLUMN learning_status TEXT NOT NULL DEFAULT 'in_progress'",
        "ALTER TABLE video ADD COLUMN last_opened_at INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE video ADD COLUMN last_position REAL NOT NULL DEFAULT 0",
        "ALTER TABLE video ADD COLUMN list_id INTEGER",
        "ALTER TABLE segment ADD COLUMN interval_seconds INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE segment ADD COLUMN completed_loops INTEGER NOT NULL DEFAULT 0",
    ] {
        if let Err(err) = connection.execute(migration, []) {
            if !err.to_string().contains("duplicate column name") {
                return Err(sqlite_error(err));
            }
        }
    }
    Ok(())
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
            })
            .expect("save segment");

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
            })
            .expect("update settings without resetting history");
        let segments = repository.find_by_video(video_id).expect("list segments");
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].completed_loops, 2);
        assert_eq!(segments[0].repeat_count, 5);
        assert_eq!(segments[0].range.start, 11.5);
        assert_eq!(segments[0].range.end, 22.0);

        drop(repository);
        let mut repository =
            SqliteSegmentRepository::open_path(&db_path).expect("reopen repository");
        let persisted = repository.find_by_video(video_id).expect("reload segments");
        assert_eq!(persisted[0].completed_loops, 2);

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
            })
            .expect("save earlier segment");
        let ordered = repository
            .find_by_video(video_id)
            .expect("list segments by time");
        assert_eq!(ordered.len(), 2);
        assert_eq!(ordered[0].id, Some(earlier_id));
        assert_eq!(ordered[1].id, Some(id));

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
