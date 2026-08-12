//! 跟读录音的核心模型、状态机和持久化端口。

mod manager;
mod playback;
mod recording;
mod session;

pub use manager::{clamp_alignment_offset, DefaultRecordingManager, RecordingManager};
pub use playback::RecordingPlaybackTimeline;
pub use recording::{NewRecording, Recording};
pub use session::{RecordingSession, RecordingState};

pub trait RecordingRepository {
    fn ensure_video(
        &mut self,
        path: &str,
        title: &str,
        duration_secs: f64,
    ) -> els_types::AppResult<i64>;
    fn insert(&mut self, recording: &NewRecording) -> els_types::AppResult<i64>;
    fn latest_for_range(
        &self,
        video_id: i64,
        range: els_types::TimeRange,
    ) -> els_types::AppResult<Option<Recording>>;
    fn update_alignment(
        &mut self,
        recording_id: i64,
        video_id: i64,
        offset_secs: f64,
    ) -> els_types::AppResult<()>;
    fn delete(&mut self, recording_id: i64, video_id: i64) -> els_types::AppResult<()>;
}
