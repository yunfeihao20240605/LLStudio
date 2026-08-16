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
    fn list_ranges(&self, video_id: i64) -> els_types::AppResult<Vec<TimeRange>>;
    fn set_alignment(
        &mut self,
        recording: &mut Recording,
        video_duration_secs: f64,
        offset_secs: f64,
    ) -> els_types::AppResult<()>;
    fn save_variant(
        &mut self,
        recording: &mut Recording,
        variant: &str,
        file_path: String,
    ) -> els_types::AppResult<()>;
    fn set_active_variant(
        &mut self,
        recording: &mut Recording,
        variant: &str,
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
            active_variant: "original".to_string(),
            denoised_light_path: None,
            denoised_standard_path: None,
            denoised_strong_path: None,
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

    fn list_ranges(&self, video_id: i64) -> els_types::AppResult<Vec<TimeRange>> {
        if video_id <= 0 {
            return Err(els_types::AppError::InvalidArgument(
                "video id is invalid".to_string(),
            ));
        }
        self.repository.list_ranges(video_id)
    }

    fn set_alignment(
        &mut self,
        recording: &mut Recording,
        video_duration_secs: f64,
        offset_secs: f64,
    ) -> els_types::AppResult<()> {
        let clamped = clamp_alignment_offset(
            recording.range,
            recording.duration_secs,
            video_duration_secs,
            offset_secs,
        );
        self.repository
            .update_alignment(recording.id, recording.video_id, clamped)?;
        recording.alignment_offset = clamped;
        Ok(())
    }

    fn delete(&mut self, recording: &Recording) -> els_types::AppResult<()> {
        self.repository.delete(recording.id, recording.video_id)
    }

    fn save_variant(
        &mut self,
        recording: &mut Recording,
        variant: &str,
        file_path: String,
    ) -> els_types::AppResult<()> {
        if !matches!(variant, "light" | "standard" | "strong") || file_path.trim().is_empty() {
            return Err(els_types::AppError::InvalidArgument(
                "降噪版本无效".to_string(),
            ));
        }
        self.repository.save_variant(
            recording.id,
            recording.video_id,
            variant,
            &file_path,
        )?;
        match variant {
            "light" => recording.denoised_light_path = Some(file_path),
            "standard" => recording.denoised_standard_path = Some(file_path),
            "strong" => recording.denoised_strong_path = Some(file_path),
            _ => unreachable!(),
        }
        recording.active_variant = variant.to_string();
        Ok(())
    }

    fn set_active_variant(
        &mut self,
        recording: &mut Recording,
        variant: &str,
    ) -> els_types::AppResult<()> {
        if recording.variant_path(variant).is_none() {
            return Err(els_types::AppError::NotFound);
        }
        self.repository
            .set_active_variant(recording.id, recording.video_id, variant)?;
        recording.active_variant = variant.to_string();
        Ok(())
    }
}

pub fn clamp_alignment_offset(
    range: TimeRange,
    recording_duration_secs: f64,
    video_duration_secs: f64,
    offset_secs: f64,
) -> f64 {
    if !offset_secs.is_finite()
        || !recording_duration_secs.is_finite()
        || recording_duration_secs <= 0.0
        || !video_duration_secs.is_finite()
        || video_duration_secs <= 0.0
    {
        return 0.0;
    }
    let latest_start = (video_duration_secs - recording_duration_secs).max(0.0);
    let absolute_start = (range.start + offset_secs).clamp(0.0, latest_start);
    absolute_start - range.start
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
    fn alignment_allows_recording_anywhere_inside_video() {
        let range = TimeRange {
            start: 10.0,
            end: 14.0,
        };
        assert_eq!(clamp_alignment_offset(range, 3.0, 30.0, -20.0), -10.0);
        assert_eq!(clamp_alignment_offset(range, 3.0, 30.0, 30.0), 17.0);
        assert_eq!(clamp_alignment_offset(range, 3.0, 30.0, 0.25), 0.25);
    }
}
