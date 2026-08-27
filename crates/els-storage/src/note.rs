use crate::sqlite::{default_database_path, ensure_schema, prepare_database_path, sqlite_error};
use els_note_core::{NewNote, Note, NoteRepository, NoteSummary};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct SqliteNoteRepository {
    connection: Option<Connection>,
}

impl SqliteNoteRepository {
    pub fn open_default() -> els_types::AppResult<Self> {
        Self::open_path(default_database_path())
    }

    pub fn open_path(path: impl AsRef<Path>) -> els_types::AppResult<Self> {
        let path = prepare_database_path(path)?;
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
            .ok_or_else(|| els_types::AppError::Io("note repository is unavailable".to_string()))
    }

    fn now_millis() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .min(i64::MAX as u128) as i64
    }
}

impl Default for SqliteNoteRepository {
    fn default() -> Self {
        Self::open_default().unwrap_or_else(|err| {
            eprintln!("Failed to initialize note repository: {err}");
            Self::disabled()
        })
    }
}

impl NoteRepository for SqliteNoteRepository {
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

    fn insert(&mut self, note: &NewNote) -> els_types::AppResult<i64> {
        let timestamp = Self::now_millis();
        self.connection()?
            .execute(
                "INSERT INTO video_note (
                    video_id, start_time, end_time, content, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                params![
                    note.video_id,
                    note.start_time,
                    note.end_time,
                    &note.content,
                    timestamp
                ],
            )
            .map_err(sqlite_error)?;
        Ok(self.connection()?.last_insert_rowid())
    }

    fn find_summaries_by_video(&self, video_id: i64) -> els_types::AppResult<Vec<NoteSummary>> {
        let mut statement = self
            .connection()?
            .prepare(
                "SELECT id, start_time, end_time,
                        REPLACE(REPLACE(SUBSTR(content, 1, 160), CHAR(13), ' '), CHAR(10), ' '),
                        updated_at
                 FROM video_note
                 WHERE video_id = ?1
                 ORDER BY start_time ASC, id ASC",
            )
            .map_err(sqlite_error)?;
        let rows = statement
            .query_map(params![video_id], |row| {
                Ok(NoteSummary {
                    id: row.get(0)?,
                    start_time: row.get(1)?,
                    end_time: row.get(2)?,
                    preview: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            })
            .map_err(sqlite_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(sqlite_error)
    }

    fn find_by_id(&self, note_id: i64, video_id: i64) -> els_types::AppResult<Note> {
        self.connection()?
            .query_row(
                "SELECT id, video_id, start_time, end_time, content, created_at, updated_at
                 FROM video_note WHERE id = ?1 AND video_id = ?2",
                params![note_id, video_id],
                |row| {
                    Ok(Note {
                        id: row.get(0)?,
                        video_id: row.get(1)?,
                        start_time: row.get(2)?,
                        end_time: row.get(3)?,
                        content: row.get(4)?,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                    })
                },
            )
            .optional()
            .map_err(sqlite_error)?
            .ok_or(els_types::AppError::NotFound)
    }

    fn update_content(
        &mut self,
        note_id: i64,
        video_id: i64,
        content: &str,
    ) -> els_types::AppResult<()> {
        let changed = self
            .connection()?
            .execute(
                "UPDATE video_note
                 SET content = ?1, updated_at = MAX(updated_at + 1, ?2)
                 WHERE id = ?3 AND video_id = ?4",
                params![content, Self::now_millis(), note_id, video_id],
            )
            .map_err(sqlite_error)?;
        if changed == 0 {
            return Err(els_types::AppError::NotFound);
        }
        Ok(())
    }

    fn delete(&mut self, note_id: i64, video_id: i64) -> els_types::AppResult<()> {
        let changed = self
            .connection()?
            .execute(
                "DELETE FROM video_note WHERE id = ?1 AND video_id = ?2",
                params![note_id, video_id],
            )
            .map_err(sqlite_error)?;
        if changed == 0 {
            return Err(els_types::AppError::NotFound);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::SqliteNoteRepository;
    use els_note_core::{NewNote, NoteRepository};

    #[test]
    fn persists_orders_updates_and_isolates_video_notes() {
        let db_path = std::env::temp_dir().join(format!(
            "els-notes-{}-{}.sqlite3",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock")
                .as_nanos()
        ));
        let mut repository = SqliteNoteRepository::open_path(&db_path).expect("open notes");
        let first_video = repository
            .ensure_video("/tmp/first.mp4", "first.mp4", 60.0)
            .expect("ensure first video");
        let second_video = repository
            .ensure_video("/tmp/second.mp4", "second.mp4", 90.0)
            .expect("ensure second video");
        let later_id = repository
            .insert(&NewNote {
                video_id: first_video,
                start_time: 20.0,
                end_time: None,
                content: "later note".to_string(),
            })
            .expect("insert later note");
        let earlier_id = repository
            .insert(&NewNote {
                video_id: first_video,
                start_time: 10.0,
                end_time: Some(12.0),
                content: "first line\nsecond line".to_string(),
            })
            .expect("insert earlier note");
        repository
            .insert(&NewNote {
                video_id: second_video,
                start_time: 5.0,
                end_time: None,
                content: "other video".to_string(),
            })
            .expect("insert other note");

        let summaries = repository
            .find_summaries_by_video(first_video)
            .expect("list first video notes");
        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0].id, earlier_id);
        assert_eq!(summaries[0].preview, "first line second line");
        assert_eq!(summaries[1].id, later_id);

        repository
            .update_content(earlier_id, first_video, "updated")
            .expect("update note");
        assert_eq!(
            repository
                .find_by_id(earlier_id, first_video)
                .expect("load note")
                .content,
            "updated"
        );
        repository
            .delete(later_id, first_video)
            .expect("delete note");
        assert_eq!(
            repository
                .find_summaries_by_video(first_video)
                .expect("list remaining notes")
                .len(),
            1
        );

        drop(repository);
        let _ = std::fs::remove_file(db_path);
    }
}
