#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RecordingPlaybackTimeline {
    selection_duration: f64,
    recording_duration: f64,
    alignment_offset: f64,
}

impl RecordingPlaybackTimeline {
    pub fn new(
        selection_duration: f64,
        recording_duration: f64,
        alignment_offset: f64,
    ) -> els_types::AppResult<Self> {
        if !selection_duration.is_finite()
            || selection_duration <= 0.0
            || !recording_duration.is_finite()
            || recording_duration <= 0.0
            || !alignment_offset.is_finite()
        {
            return Err(els_types::AppError::InvalidArgument(
                "recording playback timeline is invalid".to_string(),
            ));
        }
        Ok(Self {
            selection_duration,
            recording_duration,
            alignment_offset,
        })
    }

    pub fn selection_duration(self) -> f64 {
        self.selection_duration
    }

    pub fn clamp_position(self, position_secs: f64) -> f64 {
        if !position_secs.is_finite() {
            return 0.0;
        }
        position_secs.clamp(0.0, self.selection_duration)
    }

    pub fn recording_position_at(self, selection_position: f64) -> Option<f64> {
        let selection_position = self.clamp_position(selection_position);
        let recording_position = selection_position - self.alignment_offset;
        if recording_position >= 0.0 && recording_position < self.recording_duration {
            Some(recording_position)
        } else {
            None
        }
    }

    pub fn has_overlap(self) -> bool {
        let recording_start = self.alignment_offset;
        let recording_end = recording_start + self.recording_duration;
        recording_end > 0.0 && recording_start < self.selection_duration
    }
}

#[cfg(test)]
mod tests {
    use super::RecordingPlaybackTimeline;

    #[test]
    fn maps_positive_alignment_to_leading_silence() {
        let timeline = RecordingPlaybackTimeline::new(5.0, 3.0, 1.0).expect("timeline");
        assert_eq!(timeline.recording_position_at(0.5), None);
        assert_eq!(timeline.recording_position_at(1.0), Some(0.0));
        assert_eq!(timeline.recording_position_at(3.5), Some(2.5));
        assert_eq!(timeline.recording_position_at(4.0), None);
    }

    #[test]
    fn trims_recording_that_starts_before_the_selection() {
        let timeline = RecordingPlaybackTimeline::new(4.0, 3.0, -1.0).expect("timeline");
        assert_eq!(timeline.recording_position_at(0.0), Some(1.0));
        assert_eq!(timeline.recording_position_at(1.9), Some(2.9));
        assert_eq!(timeline.recording_position_at(2.0), None);
        assert_eq!(timeline.clamp_position(10.0), 4.0);
    }

    #[test]
    fn detects_recordings_outside_the_selection() {
        let before = RecordingPlaybackTimeline::new(4.0, 3.0, -4.0).expect("timeline");
        let after = RecordingPlaybackTimeline::new(4.0, 3.0, 4.0).expect("timeline");
        let overlap = RecordingPlaybackTimeline::new(4.0, 3.0, 3.5).expect("timeline");
        assert!(!before.has_overlap());
        assert!(!after.has_overlap());
        assert!(overlap.has_overlap());
    }
}
