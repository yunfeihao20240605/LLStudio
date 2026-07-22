//! `els-storage`：数据持久化实现（SQLite）。
//!
//! 依赖方向规则：依赖 `els-types` 与 `els-learning-core`，
//! **实现**（而不是定义）`els-learning-core::SegmentRepository` trait，
//! 体现依赖倒置——业务逻辑（`els-learning-core`）不依赖具体存储技术，
//! 具体存储技术反过来依赖业务层定义的接口。
//!
//! 本 crate 不被 `els-media-core` / `els-waveform-core` / `els-subtitle-core`
//! 依赖，也不依赖 `els-qt-bridge`。

mod schema;
mod library;
mod settings;
mod sqlite;

pub use library::{LearningVideo, VideoLibraryRepository, VideoList};
pub use settings::SettingsStore;
pub use settings::THEME_MODE_KEY;
pub use sqlite::SqliteSegmentRepository;
