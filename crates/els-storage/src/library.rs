//! Persistent video library used by the "正在学习" view.

use crate::sqlite::{default_database_path, ensure_schema, sqlite_error};
use rusqlite::{params, Connection};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, PartialEq)]
pub struct LearningVideo {
    pub path: String,
    pub title: String,
    pub duration_secs: f64,
    pub last_opened_at: i64,
    pub list_id: Option<i64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VideoList {
    pub id: i64,
    pub name: String,
}

pub struct VideoLibraryRepository {
    connection: Option<Connection>,
}

impl VideoLibraryRepository {
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

    pub fn record_opened(
        &mut self,
        path: &str,
        title: &str,
        duration_secs: f64,
    ) -> els_types::AppResult<()> {
        let clock_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .min(i64::MAX as u128) as i64;
        let connection = self.connection()?;
        let latest_opened_at = connection
            .query_row(
                "SELECT COALESCE(MAX(last_opened_at), 0) FROM video",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map_err(sqlite_error)?;
        let opened_at = clock_time.max(latest_opened_at.saturating_add(1));
        connection
            .execute(
                "INSERT INTO video (
                    path, title, duration, learning_status, last_opened_at
                 ) VALUES (?1, ?2, ?3, 'in_progress', ?4)
                 ON CONFLICT(path) DO UPDATE SET
                    title = excluded.title,
                    duration = excluded.duration,
                    learning_status = 'in_progress',
                    last_opened_at = excluded.last_opened_at",
                params![path, title, duration_secs, opened_at],
            )
            .map_err(sqlite_error)?;
        Ok(())
    }

    pub fn list_in_progress(&self) -> els_types::AppResult<Vec<LearningVideo>> {
        let mut statement = self
            .connection()?
            .prepare(
                "SELECT path, title, duration, last_opened_at, list_id
                 FROM video
                 WHERE learning_status = 'in_progress'
                 ORDER BY last_opened_at DESC, id DESC",
            )
            .map_err(sqlite_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok(LearningVideo {
                    path: row.get(0)?,
                    title: row.get(1)?,
                    duration_secs: row.get(2)?,
                    last_opened_at: row.get(3)?,
                    list_id: row.get(4)?,
                })
            })
            .map_err(sqlite_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(sqlite_error)
    }

    pub fn list_video_lists(&self) -> els_types::AppResult<Vec<VideoList>> {
        let mut statement = self
            .connection()?
            .prepare("SELECT id, name FROM video_list ORDER BY created_at, id")
            .map_err(sqlite_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok(VideoList {
                    id: row.get(0)?,
                    name: row.get(1)?,
                })
            })
            .map_err(sqlite_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(sqlite_error)
    }

    pub fn create_video_list(&mut self, name: &str) -> els_types::AppResult<()> {
        let name = name.trim();
        if name.is_empty() {
            return Err(els_types::AppError::InvalidArgument(
                "list name cannot be empty".to_string(),
            ));
        }
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .min(i64::MAX as u128) as i64;
        self.connection()?
            .execute(
                "INSERT INTO video_list (name, created_at) VALUES (?1, ?2)",
                params![name, created_at],
            )
            .map_err(sqlite_error)?;
        Ok(())
    }

    pub fn move_video_to_list(
        &mut self,
        video_path: &str,
        list_id: Option<i64>,
    ) -> els_types::AppResult<()> {
        let changed = self
            .connection()?
            .execute(
                "UPDATE video SET list_id = ?1 WHERE path = ?2 AND learning_status = 'in_progress'",
                params![list_id, video_path],
            )
            .map_err(sqlite_error)?;
        if changed == 0 {
            return Err(els_types::AppError::NotFound);
        }
        Ok(())
    }

    pub fn remove_from_learning(&mut self, video_path: &str) -> els_types::AppResult<()> {
        let changed = self
            .connection()?
            .execute(
                "UPDATE video
                 SET learning_status = 'deleted', list_id = NULL
                 WHERE path = ?1 AND learning_status = 'in_progress'",
                params![video_path],
            )
            .map_err(sqlite_error)?;
        if changed == 0 {
            return Err(els_types::AppError::NotFound);
        }
        Ok(())
    }

    pub fn delete_video_list(&mut self, list_id: i64) -> els_types::AppResult<()> {
        let connection = self.connection()?;
        let transaction = connection.unchecked_transaction().map_err(sqlite_error)?;
        transaction
            .execute(
                "UPDATE video SET list_id = NULL WHERE list_id = ?1",
                params![list_id],
            )
            .map_err(sqlite_error)?;
        transaction
            .execute("DELETE FROM video_list WHERE id = ?1", params![list_id])
            .map_err(sqlite_error)?;
        transaction.commit().map_err(sqlite_error)
    }

    fn connection(&self) -> els_types::AppResult<&Connection> {
        self.connection
            .as_ref()
            .ok_or_else(|| els_types::AppError::Io("video library is unavailable".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::VideoLibraryRepository;

    #[test]
    fn records_lists_and_updates_opened_videos_without_duplicates() {
        let db_path = std::env::temp_dir().join(format!(
            "els-video-library-{}-{}.sqlite3",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock")
                .as_nanos()
        ));
        let mut repository = VideoLibraryRepository::open_path(&db_path).expect("open repository");

        repository
            .record_opened("/tmp/first.mp4", "first.mp4", 60.0)
            .expect("record first video");
        repository
            .record_opened("/tmp/second.mp4", "second.mp4", 90.0)
            .expect("record second video");
        repository
            .record_opened("/tmp/first.mp4", "First lesson.mp4", 61.0)
            .expect("update first video");

        let videos = repository.list_in_progress().expect("list videos");
        assert_eq!(videos.len(), 2);
        assert_eq!(videos[0].path, "/tmp/first.mp4");
        assert_eq!(videos[0].title, "First lesson.mp4");
        assert_eq!(videos[0].duration_secs, 61.0);

        repository
            .create_video_list("News")
            .expect("create video list");
        let lists = repository.list_video_lists().expect("list video lists");
        assert_eq!(lists.len(), 1);
        repository
            .move_video_to_list("/tmp/first.mp4", Some(lists[0].id))
            .expect("move video into list");
        let videos = repository.list_in_progress().expect("list grouped videos");
        assert_eq!(videos[0].list_id, Some(lists[0].id));

        repository
            .delete_video_list(lists[0].id)
            .expect("delete list");
        assert!(repository
            .list_video_lists()
            .expect("list empty")
            .is_empty());
        let videos = repository
            .list_in_progress()
            .expect("list ungrouped videos");
        assert!(videos.iter().all(|video| video.list_id.is_none()));

        repository
            .remove_from_learning("/tmp/second.mp4")
            .expect("remove video from learning library");
        let videos = repository.list_in_progress().expect("list after removal");
        assert_eq!(videos.len(), 1);
        assert_eq!(videos[0].path, "/tmp/first.mp4");

        drop(repository);
        let repository = VideoLibraryRepository::open_path(&db_path).expect("reopen repository");
        assert_eq!(
            repository.list_in_progress().expect("reload videos").len(),
            1
        );

        drop(repository);
        let _ = std::fs::remove_file(db_path);
    }
}
