//! `els-storage`：数据持久化实现（SQLite）。
//!
//! 依赖方向规则：依赖 `els-types` 与 `els-learning-core`，
//! **实现**（而不是定义）`els-learning-core::SegmentRepository` trait，
//! 体现依赖倒置——业务逻辑（`els-learning-core`）不依赖具体存储技术，
//! 具体存储技术反过来依赖业务层定义的接口。
//!
//! 本 crate 不被 `els-media-core` / `els-waveform-core` / `els-subtitle-core`
//! 依赖，也不依赖 `els-qt-bridge`。

mod library;
mod note;
mod recording;
mod schema;
mod settings;
mod speech_settings;
mod sqlite;

pub use library::{LearningVideo, VideoLibraryRepository, VideoList};
pub use note::SqliteNoteRepository;
pub use recording::SqliteRecordingRepository;
pub use settings::{
    AiSettingsRepository, SettingsStore, AI_API_KEY_KEY, AI_BASE_URL_KEY, AI_MODEL_KEY,
    AI_SYSTEM_PROMPT_KEY, LAYOUT_DETAILS_PANEL_EXPANDED_KEY, LAYOUT_LIBRARY_PANEL_EXPANDED_KEY,
    LAYOUT_MODE_KEY, LAYOUT_WAVEFORM_ON_RIGHT_KEY, THEME_MODE_KEY,
};
pub use speech_settings::{SpeechProviderProfile, SpeechSettingsRepository};
pub use sqlite::SqliteSegmentRepository;
