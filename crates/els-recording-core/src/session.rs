use els_types::TimeRange;

use crate::Recording;

#[derive(Clone, Debug, Default, PartialEq)]
pub enum RecordingState {
    #[default]
    Idle,
    Recording,
    Processing,
    Ready,
    Error(String),
}

#[derive(Clone, Debug, Default)]
pub struct RecordingSession {
    state: RecordingState,
    target: Option<TimeRange>,
    recording: Option<Recording>,
}

impl RecordingSession {
    pub fn state(&self) -> &RecordingState {
        &self.state
    }

    pub fn target(&self) -> Option<TimeRange> {
        self.target
    }

    pub fn recording(&self) -> Option<&Recording> {
        self.recording.as_ref()
    }

    pub fn set_target(&mut self, target: Option<TimeRange>) -> els_types::AppResult<()> {
        if matches!(
            self.state,
            RecordingState::Recording | RecordingState::Processing
        ) {
            return Err(els_types::AppError::InvalidArgument(
                "cannot change recording target while recording".to_string(),
            ));
        }
        if let Some(range) = target {
            validate_range(range)?;
        }
        self.target = target;
        self.recording = None;
        self.state = RecordingState::Idle;
        Ok(())
    }

    pub fn load_recording(&mut self, recording: Option<Recording>) {
        self.recording = recording;
        self.state = if self.recording.is_some() {
            RecordingState::Ready
        } else {
            RecordingState::Idle
        };
    }

    pub fn start(&mut self) -> els_types::AppResult<TimeRange> {
        let target = self.target.ok_or_else(|| {
            els_types::AppError::InvalidArgument("select a recording range first".to_string())
        })?;
        if matches!(
            self.state,
            RecordingState::Recording | RecordingState::Processing
        ) {
            return Err(els_types::AppError::InvalidArgument(
                "recording session is already active".to_string(),
            ));
        }
        self.recording = None;
        self.state = RecordingState::Recording;
        Ok(target)
    }

    pub fn begin_processing(&mut self) -> els_types::AppResult<()> {
        if self.state != RecordingState::Recording {
            return Err(els_types::AppError::InvalidArgument(
                "recording is not active".to_string(),
            ));
        }
        self.state = RecordingState::Processing;
        Ok(())
    }

    pub fn complete(&mut self, recording: Recording) -> els_types::AppResult<()> {
        if self.state != RecordingState::Processing {
            return Err(els_types::AppError::InvalidArgument(
                "recording is not being processed".to_string(),
            ));
        }
        self.recording = Some(recording);
        self.state = RecordingState::Ready;
        Ok(())
    }

    pub fn fail(&mut self, message: impl Into<String>) {
        self.recording = None;
        self.state = RecordingState::Error(message.into());
    }
}

pub(crate) fn validate_range(range: TimeRange) -> els_types::AppResult<()> {
    if !range.start.is_finite()
        || !range.end.is_finite()
        || range.start < 0.0
        || range.end <= range.start
    {
        return Err(els_types::AppError::InvalidArgument(
            "recording range is invalid".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{RecordingSession, RecordingState};
    use els_types::TimeRange;

    #[test]
    fn requires_a_target_and_preserves_explicit_states() {
        let mut session = RecordingSession::default();
        assert!(session.start().is_err());

        session
            .set_target(Some(TimeRange {
                start: 10.0,
                end: 12.5,
            }))
            .expect("set target");
        assert_eq!(session.start().expect("start").start, 10.0);
        assert_eq!(session.state(), &RecordingState::Recording);
        session.begin_processing().expect("processing");
        assert_eq!(session.state(), &RecordingState::Processing);
        assert!(session.set_target(None).is_err());
    }
}
