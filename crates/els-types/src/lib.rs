//! `els-types`：跨 crate 共享的公共类型与错误定义。
//!
//! 依赖方向规则：本 crate 不依赖任何业务 crate（`els-*-core`、`els-storage`、
//! `els-qt-bridge`、`els-app`），是整个 workspace 依赖图的最底层。
//! 所有其他 crate 都可以依赖它，但它不能反向依赖任何其他 crate。

/// 时间范围（秒），用于表示视频片段、字幕区间、波形选区等。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeRange {
    pub start: f64,
    pub end: f64,
}

/// 学习片段（A-B 循环片段）的公共数据结构。
/// 具体的持久化/业务逻辑分别由 `els-learning-core` / `els-storage` 负责，
/// 本 crate 只提供数据结构定义。
#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub id: Option<i64>,
    pub video_id: i64,
    pub range: TimeRange,
    pub repeat_count: u32,
    pub interval_seconds: f64,
    pub completed_loops: u32,
    pub label: String,
}

/// 跨 crate 统一的错误类型占位。真正实现阶段可替换为 `thiserror` 派生。
#[derive(Debug)]
pub enum AppError {
    NotFound,
    InvalidArgument(String),
    Io(String),
    Unimplemented,
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::NotFound => write!(f, "not found"),
            AppError::InvalidArgument(msg) => write!(f, "invalid argument: {msg}"),
            AppError::Io(msg) => write!(f, "io error: {msg}"),
            AppError::Unimplemented => write!(f, "unimplemented"),
        }
    }
}

impl std::error::Error for AppError {}

pub type AppResult<T> = Result<T, AppError>;
