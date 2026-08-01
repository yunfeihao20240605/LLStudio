//! `els-learning-core`：片段循环训练与学习进度计算的核心业务逻辑。
//!
//! 依赖方向规则（依赖倒置 / DIP）：本 crate 只依赖 `els-types`，并且
//! **定义** [`SegmentRepository`] trait 作为持久化接口，但不依赖任何具体的
//! 数据库实现。真正的 SQLite 实现在 `els-storage` crate 中，`els-storage`
//! 反过来依赖本 crate 并实现该 trait，注入方式交给组合根 `els-app` 完成。
//! 这样业务逻辑就不会被具体存储技术（SQLite/其他）绑死。

mod manager;
mod playback_plan;
mod segment;

pub use els_types::AppResult;
pub use manager::{DefaultLearningManager, Progress};
pub use playback_plan::{build_label_playback_plan, LabelPlaybackPlan};
pub use segment::Segment;

/// 学习管理器对外契约：处理片段增删、循环训练调度、进度计算等业务规则。
pub trait LearningManager {
    fn ensure_video(
        &mut self,
        path: &str,
        title: &str,
        duration_secs: f64,
    ) -> els_types::AppResult<i64>;
    fn add_segment(&mut self, segment: Segment) -> els_types::AppResult<i64>;
    fn list_segments(&self, video_id: i64) -> els_types::AppResult<Vec<Segment>>;
    fn delete_segment(&mut self, segment_id: i64) -> els_types::AppResult<()>;
    fn set_segment_label(
        &mut self,
        segment_id: i64,
        video_id: i64,
        label: &str,
    ) -> els_types::AppResult<()>;
    fn set_segment_labels(
        &mut self,
        segment_ids: &[i64],
        video_id: i64,
        label: &str,
    ) -> els_types::AppResult<()>;
    fn list_recent_labels(&self, video_id: i64, limit: usize) -> els_types::AppResult<Vec<String>>;
    fn record_completed_loop(&mut self, segment_id: i64) -> els_types::AppResult<Progress>;
    fn record_completed_loops(&mut self, segment_ids: &[i64]) -> els_types::AppResult<()>;
}

/// 持久化接口（"port"），由 `els-storage` 实现（依赖倒置）。
pub trait SegmentRepository {
    fn ensure_video(
        &mut self,
        path: &str,
        title: &str,
        duration_secs: f64,
    ) -> els_types::AppResult<i64>;
    fn save(&mut self, segment: &Segment) -> els_types::AppResult<i64>;
    fn find_by_video(&self, video_id: i64) -> els_types::AppResult<Vec<Segment>>;
    fn delete(&mut self, segment_id: i64) -> els_types::AppResult<()>;
    fn set_labels(
        &mut self,
        segment_ids: &[i64],
        video_id: i64,
        label: &str,
    ) -> els_types::AppResult<()>;
    fn find_recent_labels(&self, video_id: i64, limit: usize) -> els_types::AppResult<Vec<String>>;
    fn increment_completed_loops(&mut self, segment_id: i64) -> els_types::AppResult<Progress>;
    fn increment_completed_loops_many(&mut self, segment_ids: &[i64]) -> els_types::AppResult<()>;
}
