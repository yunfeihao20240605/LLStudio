use crate::sqlite::{default_database_path, ensure_schema, prepare_database_path, sqlite_error};
use els_recording_core::{NewRecording, Recording, RecordingRepository};
use els_types::TimeRange;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct SqliteRecordingRepository {
    connection: Option<Connection>,
}

impl SqliteRecordingRepository {
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
        self.connection.as_ref().ok_or_else(|| {
            els_types::AppError::Io("recording repository is unavailable".to_string())
        })
    }

    fn now_millis() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .min(i64::MAX as u128) as i64
    }
}

impl Default for SqliteRecordingRepository {
    fn default() -> Self {
        Self::open_default().unwrap_or_else(|error| {
            eprintln!("Failed to initialize recording repository: {error}");
            Self::disabled()
        })
    }
}

impl RecordingRepository for SqliteRecordingRepository {
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

    fn insert(&mut self, recording: &NewRecording) -> els_types::AppResult<i64> {
        self.connection()?
            .execute(
                "INSERT INTO recording (
                    video_id, range_start, range_end, file_path, duration,
                    sample_rate, alignment_offset, created_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    recording.video_id,
                    recording.range.start,
                    recording.range.end,
                    &recording.file_path,
                    recording.duration_secs,
                    recording.sample_rate,
                    recording.alignment_offset,
                    Self::now_millis(),
                ],
            )
            .map_err(sqlite_error)?;
        Ok(self.connection()?.last_insert_rowid())
    }

    fn latest_for_range(
        &self,
        video_id: i64,
        range: TimeRange,
    ) -> els_types::AppResult<Option<Recording>> {
        self.connection()?
            .query_row(
                "SELECT id, video_id, range_start, range_end, file_path,
                        duration, sample_rate, alignment_offset, active_variant,
                        denoised_light_path, denoised_standard_path,
                        denoised_strong_path, created_at
                 FROM recording
                 WHERE video_id = ?1
                   AND ABS(range_start - ?2) <= 0.001
                   AND ABS(range_end - ?3) <= 0.001
                 ORDER BY created_at DESC, id DESC
                 LIMIT 1",
                params![video_id, range.start, range.end],
                |row| {
                    Ok(Recording {
                        id: row.get(0)?,
                        video_id: row.get(1)?,
                        range: TimeRange {
                            start: row.get(2)?,
                            end: row.get(3)?,
                        },
                        file_path: row.get(4)?,
                        duration_secs: row.get(5)?,
                        sample_rate: row.get(6)?,
                        alignment_offset: row.get(7)?,
                        active_variant: row.get(8)?,
                        denoised_light_path: row.get(9)?,
                        denoised_standard_path: row.get(10)?,
                        denoised_strong_path: row.get(11)?,
                        created_at: row.get(12)?,
                    })
                },
            )
            .optional()
            .map_err(sqlite_error)
    }

    fn list_ranges(&self, video_id: i64) -> els_types::AppResult<Vec<TimeRange>> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT DISTINCT range_start, range_end
                 FROM recording
                 WHERE video_id = ?1
                 ORDER BY range_start ASC, range_end ASC",
            )
            .map_err(sqlite_error)?;
        let rows = statement
            .query_map(params![video_id], |row| {
                Ok(TimeRange {
                    start: row.get(0)?,
                    end: row.get(1)?,
                })
            })
            .map_err(sqlite_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(sqlite_error)
    }

    fn update_alignment(
        &mut self,
        recording_id: i64,
        video_id: i64,
        offset_secs: f64,
    ) -> els_types::AppResult<()> {
        let changed = self
            .connection()?
            .execute(
                "UPDATE recording SET alignment_offset = ?1
             WHERE id = ?2 AND video_id = ?3",
                params![offset_secs, recording_id, video_id],
            )
            .map_err(sqlite_error)?;
        if changed == 0 {
            return Err(els_types::AppError::NotFound);
        }
        Ok(())
    }

    fn save_variant(
        &mut self,
        recording_id: i64,
        video_id: i64,
        variant: &str,
        file_path: &str,
    ) -> els_types::AppResult<()> {
        let column = match variant {
            "light" => "denoised_light_path",
            "standard" => "denoised_standard_path",
            "strong" => "denoised_strong_path",
            _ => {
                return Err(els_types::AppError::InvalidArgument(
                    "降噪版本无效".to_string(),
                ))
            }
        };
        let sql = format!(
            "UPDATE recording SET {column} = ?1, active_variant = ?2
             WHERE id = ?3 AND video_id = ?4"
        );
        let changed = self
            .connection()?
            .execute(&sql, params![file_path, variant, recording_id, video_id])
            .map_err(sqlite_error)?;
        if changed == 0 {
            return Err(els_types::AppError::NotFound);
        }
        Ok(())
    }

    fn set_active_variant(
        &mut self,
        recording_id: i64,
        video_id: i64,
        variant: &str,
    ) -> els_types::AppResult<()> {
        let changed = self
            .connection()?
            .execute(
                "UPDATE recording SET active_variant = ?1
                 WHERE id = ?2 AND video_id = ?3",
                params![variant, recording_id, video_id],
            )
            .map_err(sqlite_error)?;
        if changed == 0 {
            return Err(els_types::AppError::NotFound);
        }
        Ok(())
    }

    fn delete(&mut self, recording_id: i64, video_id: i64) -> els_types::AppResult<()> {
        let changed = self
            .connection()?
            .execute(
                "DELETE FROM recording WHERE id = ?1 AND video_id = ?2",
                params![recording_id, video_id],
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
    use super::SqliteRecordingRepository;
    use els_recording_core::{NewRecording, RecordingRepository};
    use els_types::TimeRange;

    #[test]
    fn loads_the_latest_recording_for_an_exact_video_range() {
        let path = std::env::temp_dir().join(format!(
            "els-recordings-{}-{}.sqlite3",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let mut repository = SqliteRecordingRepository::open_path(&path).expect("open");
        let video_id = repository
            .ensure_video("/tmp/video.mp4", "video", 60.0)
            .expect("video");
        let range = TimeRange {
            start: 10.0,
            end: 12.0,
        };
        let first_id = repository
            .insert(&NewRecording {
                video_id,
                range,
                file_path: "/tmp/first.wav".to_string(),
                duration_secs: 2.1,
                sample_rate: 48_000,
                alignment_offset: 0.0,
            })
            .expect("first");
        let second_id = repository
            .insert(&NewRecording {
                video_id,
                range,
                file_path: "/tmp/second.wav".to_string(),
                duration_secs: 2.0,
                sample_rate: 48_000,
                alignment_offset: 0.2,
            })
            .expect("second");
        assert!(second_id > first_id);
        let mut latest = repository
            .latest_for_range(video_id, range)
            .expect("latest")
            .expect("recording");
        assert_eq!(latest.id, second_id);
        repository
            .update_alignment(second_id, video_id, -0.1)
            .expect("offset");
        latest = repository
            .latest_for_range(video_id, range)
            .expect("reload")
            .expect("recording");
        assert_eq!(latest.alignment_offset, -0.1);
        repository.delete(second_id, video_id).expect("delete");
        assert_eq!(
            repository
                .latest_for_range(video_id, range)
                .expect("fallback")
                .expect("first")
                .id,
            first_id
        );
        drop(repository);
        let _ = std::fs::remove_file(path);
    }
}
