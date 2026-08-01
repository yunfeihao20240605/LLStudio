//! 学习管理器占位实现（真正实现阶段处理循环调度、进度持久化调用等）。

/// 学习进度占位类型。
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Progress {
    pub completed_loops: u32,
    pub total_loops: u32,
}

/// `LearningManager` 的占位实现，持有一个 `SegmentRepository` trait 对象
/// 以实现依赖倒置：具体存储实现由调用方（组合根）注入，本 struct 不关心
/// 它是 SQLite 还是其他实现。
pub struct DefaultLearningManager<R: crate::SegmentRepository> {
    repository: R,
}

impl<R: crate::SegmentRepository> DefaultLearningManager<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }
}

impl<R: crate::SegmentRepository> crate::LearningManager for DefaultLearningManager<R> {
    fn ensure_video(
        &mut self,
        path: &str,
        title: &str,
        duration_secs: f64,
    ) -> els_types::AppResult<i64> {
        if path.trim().is_empty() {
            return Err(els_types::AppError::InvalidArgument(
                "video path cannot be empty".to_string(),
            ));
        }
        self.repository
            .ensure_video(path, title, duration_secs.max(0.0))
    }

    fn add_segment(&mut self, segment: crate::Segment) -> els_types::AppResult<i64> {
        if !segment.range.start.is_finite()
            || !segment.range.end.is_finite()
            || segment.range.start < 0.0
            || segment.range.end <= segment.range.start
            || segment.repeat_count == 0
        {
            return Err(els_types::AppError::InvalidArgument(
                "segment requires a valid range and repeat count".to_string(),
            ));
        }
        self.repository.save(&segment)
    }

    fn list_segments(&self, video_id: i64) -> els_types::AppResult<Vec<crate::Segment>> {
        self.repository.find_by_video(video_id)
    }

    fn delete_segment(&mut self, segment_id: i64) -> els_types::AppResult<()> {
        self.repository.delete(segment_id)
    }

    fn set_segment_label(
        &mut self,
        segment_id: i64,
        video_id: i64,
        label: &str,
    ) -> els_types::AppResult<()> {
        self.set_segment_labels(&[segment_id], video_id, label)
    }

    fn set_segment_labels(
        &mut self,
        segment_ids: &[i64],
        video_id: i64,
        label: &str,
    ) -> els_types::AppResult<()> {
        if segment_ids.is_empty() {
            return Err(els_types::AppError::InvalidArgument(
                "at least one segment is required".to_string(),
            ));
        }
        let mut segment_ids = segment_ids.to_vec();
        segment_ids.sort_unstable();
        segment_ids.dedup();
        let label = label.trim().chars().take(80).collect::<String>();
        self.repository.set_labels(&segment_ids, video_id, &label)
    }

    fn list_recent_labels(&self, video_id: i64, limit: usize) -> els_types::AppResult<Vec<String>> {
        self.repository.find_recent_labels(video_id, limit)
    }

    fn record_completed_loop(&mut self, segment_id: i64) -> els_types::AppResult<Progress> {
        self.repository.increment_completed_loops(segment_id)
    }

    fn record_completed_loops(&mut self, segment_ids: &[i64]) -> els_types::AppResult<()> {
        self.repository.increment_completed_loops_many(segment_ids)
    }
}
