//! `els-qt-bridge`：唯一依赖 Qt/cxx-qt 的薄适配层（技术方案 3.4 节）。
//!
//! 当前已接入最小 cxx-qt 启动链路：`app_bootstrap.rs` 通过 `#[cxx_qt::bridge]`
//! 导出一个最小的 `AppBootstrap` QObject，供 `Main.qml` 验证 Rust ↔ QML 通路。
//! 其余 `*_bridge.rs` 仍保持普通 Rust 适配结构体，占位后续按模块逐步迁移。

mod app_bootstrap;
pub mod graphics_backend;
pub mod learning_bridge;
pub mod library_bridge;
pub mod media_bridge;
pub mod mpv_video_item;
pub mod note_bridge;
pub mod recording_bridge;
pub mod segment_bridge;
pub mod subtitle_bridge;
pub mod theme_bridge;
pub mod waveform_bridge;
