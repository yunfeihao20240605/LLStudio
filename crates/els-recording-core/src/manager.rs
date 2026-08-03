use els_types::TimeRange;

use crate::{session::validate_range, NewRecording, Recording, RecordingRepository};

pub trait RecordingManager {
    fn ensure_video(
        &mut self,
        path: &str,
        title: &str,
        duration_secs: f64,
    ) -> els_types::AppResult<i64>;
    fn save(&mut self, recording: NewRecording) -> els_types::AppResult<Recording>;
    fn latest_for_range(
        &self,
        video_id: i64,
        range: TimeRange,
    ) -> els_types::AppResult<Option<Recording>>;
    fn set_alignment(
        &mut self,
        recording: &mut Recording,
        offset_secs: f64,
    ) -> els_types::AppResult<()>;
    fn delete(&mut self, recording: &Recording) -> els_types::AppResult<()>;
}

pub struct DefaultRecordingManager<R> {
    repository: R,
}

impl<R> DefaultRecordingManager<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }
}

impl<R: RecordingRepository> RecordingManager for DefaultRecordingManager<R> {
    fn ensure_video(
        &mut self,
        path: &str,
        title: &str,
        duration_secs: f64,
    ) -> els_types::AppResult<i64> {
        if path.trim().is_empty() || !duration_secs.is_finite() || duration_secs <= 0.0 {
            return Err(els_types::AppError::InvalidArgument(
                "video metadata is invalid".to_string(),
            ));
        }
        self.repository.ensure_video(path, title, duration_secs)
    }

    fn save(&mut self, recording: NewRecording) -> els_types::AppResult<Recording> {
        validate_new_recording(&recording)?;
        let id = self.repository.insert(&recording)?;
        Ok(Recording {
            id,
            video_id: recording.video_id,
            range: recording.range,
            file_path: recording.file_path,
            duration_secs: recording.duration_secs,
            sample_rate: recording.sample_rate,
            alignment_offset: recording.alignment_offset,
            created_at: 0,
        })
    }

    fn latest_for_range(
        &self,
        video_id: i64,
        range: TimeRange,
    ) -> els_types::AppResult<Option<Recording>> {
        validate_range(range)?;
        self.repository.latest_for_range(video_id, range)
    }

    fn set_alignment(
        &mut self,
        recording: &mut Recording,
        offset_secs: f64,
    ) -> els_types::AppResult<()> {
        let clamped = clamp_alignment_offset(recording.range, recording.duration_secs, offset_secs);
        self.repository
            .update_alignment(recording.id, recording.video_id, clamped)?;
        recording.alignment_offset = clamped;
        Ok(())
    }

    fn delete(&mut self, recording: &Recording) -> els_types::AppResult<()> {
        self.repository.delete(recording.id, recording.video_id)
    }
}

pub fn clamp_alignment_offset(range: TimeRange, duration_secs: f64, offset_secs: f64) -> f64 {
    if !offset_secs.is_finite() || !duration_secs.is_finite() || duration_secs <= 0.0 {
        return 0.0;
    }
    let minimum = -duration_secs + 0.01;
    let maximum = (range.end - range.start) - 0.01;
    offset_secs.clamp(minimum.min(maximum), maximum.max(minimum))
}

fn validate_new_recording(recording: &NewRecording) -> els_types::AppResult<()> {
    validate_range(recording.range)?;
    if recording.video_id <= 0
        || recording.file_path.trim().is_empty()
        || !recording.duration_secs.is_finite()
        || recording.duration_secs <= 0.0
        || recording.sample_rate == 0
    {
        return Err(els_types::AppError::InvalidArgument(
            "recording metadata is invalid".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::clamp_alignment_offset;
    use els_types::TimeRange;

    #[test]
    fn alignment_keeps_part_of_the_recording_inside_the_target() {
        let range = TimeRange {
            start: 10.0,
            end: 14.0,
        };
        assert_eq!(clamp_alignment_offset(range, 3.0, -10.0), -2.99);
        assert_eq!(clamp_alignment_offset(range, 3.0, 10.0), 3.99);
        assert_eq!(clamp_alignment_offset(range, 3.0, 0.25), 0.25);
    }
}
