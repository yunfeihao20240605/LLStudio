//! `els-media-core`：视频/音频播放与解码的核心业务逻辑。
//!
//! 依赖方向规则：只依赖 `els-types`；不依赖 `els-waveform-core` /
//! `els-subtitle-core` / `els-learning-core` / `els-storage` / `els-qt-bridge`。
//! 上层（`els-qt-bridge`、`els-app`）只通过 [`MediaController`] trait 使用本 crate，
//! 不直接引用具体实现类型，以便未来替换播放/解码实现而不影响调用方。

mod decoder;
mod mpv;
mod player;

pub use decoder::{Decoder, MediaProbe};
pub use player::{Player, PlayerState as PlaybackState};

/// 播放器对外契约。具体实现（基于 FFmpeg 绑定）放在 `player.rs` / `decoder.rs`，
/// 不对外暴露具体 struct，调用方只依赖此 trait。
pub trait MediaController {
    fn load(&mut self, path: &str) -> els_types::AppResult<()>;
    fn play(&mut self) -> els_types::AppResult<()>;
    fn pause(&mut self) -> els_types::AppResult<()>;
    fn seek(&mut self, position_secs: f64) -> els_types::AppResult<()>;
    fn set_playback_rate(&mut self, playback_rate: f64) -> els_types::AppResult<()>;
    fn playback_rate(&self) -> f64;
    fn state(&self) -> PlaybackState;
}
