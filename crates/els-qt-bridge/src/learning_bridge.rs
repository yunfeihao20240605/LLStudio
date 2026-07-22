//! 包装 `els_learning_core::LearningManager`，未来暴露给 QML 的
//! 片段列表/学习控制面板使用。只做适配转换。

/// QObject 适配层占位。
pub struct LearningBridge<M: els_learning_core::LearningManager> {
    manager: M,
}

impl<M: els_learning_core::LearningManager> LearningBridge<M> {
    pub fn new(manager: M) -> Self {
        Self { manager }
    }

    pub fn add_segment(
        &mut self,
        segment: els_learning_core::Segment,
    ) -> els_types::AppResult<i64> {
        self.manager.add_segment(segment)
    }
}
