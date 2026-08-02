use crate::{NewNote, Note, NoteRepository, NoteSummary};

const MAX_NOTE_BYTES: usize = 100 * 1024;
const POINT_NOTE_TOLERANCE_SECS: f64 = 1.0;

pub trait NoteManager {
    fn ensure_video(
        &mut self,
        path: &str,
        title: &str,
        duration_secs: f64,
    ) -> els_types::AppResult<i64>;
    fn create_note(&mut self, note: NewNote) -> els_types::AppResult<i64>;
    fn list_summaries(&self, video_id: i64) -> els_types::AppResult<Vec<NoteSummary>>;
    fn load_note(&self, note_id: i64, video_id: i64) -> els_types::AppResult<Note>;
    fn update_content(
        &mut self,
        note_id: i64,
        video_id: i64,
        content: &str,
    ) -> els_types::AppResult<()>;
    fn delete_note(&mut self, note_id: i64, video_id: i64) -> els_types::AppResult<()>;
}

pub struct DefaultNoteManager<R> {
    repository: R,
}

impl<R> DefaultNoteManager<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }
}

impl<R: NoteRepository> NoteManager for DefaultNoteManager<R> {
    fn ensure_video(
        &mut self,
        path: &str,
        title: &str,
        duration_secs: f64,
    ) -> els_types::AppResult<i64> {
        if path.trim().is_empty() || !duration_secs.is_finite() || duration_secs < 0.0 {
            return Err(els_types::AppError::InvalidArgument(
                "video metadata is invalid".to_string(),
            ));
        }
        self.repository.ensure_video(path, title, duration_secs)
    }

    fn create_note(&mut self, note: NewNote) -> els_types::AppResult<i64> {
        validate_anchor(note.start_time, note.end_time)?;
        validate_content(&note.content)?;
        self.repository.insert(&note)
    }

    fn list_summaries(&self, video_id: i64) -> els_types::AppResult<Vec<NoteSummary>> {
        self.repository.find_summaries_by_video(video_id)
    }

    fn load_note(&self, note_id: i64, video_id: i64) -> els_types::AppResult<Note> {
        self.repository.find_by_id(note_id, video_id)
    }

    fn update_content(
        &mut self,
        note_id: i64,
        video_id: i64,
        content: &str,
    ) -> els_types::AppResult<()> {
        validate_content(content)?;
        self.repository.update_content(note_id, video_id, content)
    }

    fn delete_note(&mut self, note_id: i64, video_id: i64) -> els_types::AppResult<()> {
        self.repository.delete(note_id, video_id)
    }
}

fn validate_anchor(start_time: f64, end_time: Option<f64>) -> els_types::AppResult<()> {
    if !start_time.is_finite() || start_time < 0.0 {
        return Err(els_types::AppError::InvalidArgument(
            "note start time is invalid".to_string(),
        ));
    }
    if let Some(end_time) = end_time {
        if !end_time.is_finite() || end_time <= start_time {
            return Err(els_types::AppError::InvalidArgument(
                "note end time is invalid".to_string(),
            ));
        }
    }
    Ok(())
}

fn validate_content(content: &str) -> els_types::AppResult<()> {
    if content.len() > MAX_NOTE_BYTES {
        return Err(els_types::AppError::InvalidArgument(
            "note content exceeds 100 KiB".to_string(),
        ));
    }
    Ok(())
}

pub fn active_note_index(notes: &[NoteSummary], position_secs: f64) -> Option<usize> {
    if !position_secs.is_finite() || position_secs < 0.0 {
        return None;
    }

    let mut best: Option<(usize, f64, f64)> = None;
    for (index, note) in notes.iter().enumerate() {
        let duration = note
            .end_time
            .map(|end| end - note.start_time)
            .unwrap_or(0.0);
        let contains = match note.end_time {
            Some(end) => position_secs >= note.start_time && position_secs < end,
            None => (position_secs - note.start_time).abs() <= POINT_NOTE_TOLERANCE_SECS,
        };
        if !contains {
            continue;
        }

        let replace = best
            .map(|(_, best_start, best_duration)| {
                note.start_time > best_start
                    || ((note.start_time - best_start).abs() <= f64::EPSILON
                        && duration < best_duration)
            })
            .unwrap_or(true);
        if replace {
            best = Some((index, note.start_time, duration));
        }
    }
    best.map(|(index, _, _)| index)
}

#[cfg(test)]
mod tests {
    use super::active_note_index;
    use crate::NoteSummary;

    fn summary(id: i64, start: f64, end: Option<f64>) -> NoteSummary {
        NoteSummary {
            id,
            start_time: start,
            end_time: end,
            preview: String::new(),
            updated_at: 0,
        }
    }

    #[test]
    fn chooses_one_deterministic_note_for_overlapping_ranges() {
        let notes = vec![
            summary(1, 10.0, Some(20.0)),
            summary(2, 15.0, Some(18.0)),
            summary(3, 15.0, Some(17.0)),
        ];
        assert_eq!(active_note_index(&notes, 16.0), Some(2));
    }

    #[test]
    fn highlights_point_notes_only_near_their_timestamp() {
        let notes = vec![summary(1, 10.0, None)];
        assert_eq!(active_note_index(&notes, 10.8), Some(0));
        assert_eq!(active_note_index(&notes, 11.1), None);
    }
}
