//! 基于 libmpv 的嵌入式播放器核心。
//! 当前阶段由 Rust 侧负责媒体控制与状态同步，Qt/QML 侧通过 libmpv render API
//! 把同一个 mpv handle 渲染到界面内部。

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlayerState {
    #[default]
    Stopped,
    Playing,
    Paused,
}

pub struct Player {
    state: PlayerState,
    loaded_path: Option<String>,
    current_position_secs: f64,
    duration_secs: f64,
    playback_rate: f64,
    media_probe: Option<crate::MediaProbe>,
    mpv: Option<crate::mpv::MpvHandle>,
    backend_error: Option<String>,
    keep_open: bool,
    pending_initial_position_secs: Option<f64>,
}

impl Default for Player {
    fn default() -> Self {
        Self::new_with_keep_open(false)
    }
}

impl Player {
    pub fn new_audio_playback() -> Self {
        Self::new_with_keep_open(true)
    }

    fn new_with_keep_open(keep_open: bool) -> Self {
        match crate::mpv::MpvHandle::new_with_keep_open(keep_open) {
            Ok(mpv) => Self {
                state: PlayerState::Stopped,
                loaded_path: None,
                current_position_secs: 0.0,
                duration_secs: 0.0,
                playback_rate: 1.0,
                media_probe: None,
                mpv: Some(mpv),
                backend_error: None,
                keep_open,
                pending_initial_position_secs: None,
            },
            Err(err) => Self {
                state: PlayerState::Stopped,
                loaded_path: None,
                current_position_secs: 0.0,
                duration_secs: 0.0,
                playback_rate: 1.0,
                media_probe: None,
                mpv: None,
                backend_error: Some(err.to_string()),
                keep_open,
                pending_initial_position_secs: None,
            },
        }
    }
}

impl crate::MediaController for Player {
    fn load(&mut self, path: &str) -> els_types::AppResult<()> {
        if path.trim().is_empty() {
            return Err(els_types::AppError::InvalidArgument(
                "video path must not be empty".to_string(),
            ));
        }

        let probe = crate::Decoder::probe(path)?;
        let playback_rate = self.playback_rate;
        self.with_mpv(|mpv| {
            mpv.load_file(path)?;
            mpv.set_pause(true)?;
            mpv.set_speed(playback_rate)
        })?;

        self.loaded_path = Some(path.to_string());
        self.current_position_secs = 0.0;
        self.duration_secs = probe.duration_secs;
        self.media_probe = Some(probe);
        self.state = PlayerState::Paused;
        self.pending_initial_position_secs = Some(0.0);
        Ok(())
    }

    fn play(&mut self) -> els_types::AppResult<()> {
        self.ensure_loaded()?;
        self.with_mpv(|mpv| mpv.set_pause(false))?;
        self.state = PlayerState::Playing;
        Ok(())
    }

    fn pause(&mut self) -> els_types::AppResult<()> {
        self.ensure_loaded()?;
        self.with_mpv(|mpv| mpv.set_pause(true))?;
        self.sync_from_backend()?;
        self.state = PlayerState::Paused;
        Ok(())
    }

    fn seek(&mut self, position_secs: f64) -> els_types::AppResult<()> {
        self.ensure_loaded()?;

        if position_secs.is_nan() || position_secs.is_sign_negative() {
            return Err(els_types::AppError::InvalidArgument(
                "seek position must be a non-negative number".to_string(),
            ));
        }

        if position_secs > self.duration_secs {
            return Err(els_types::AppError::InvalidArgument(format!(
                "seek position {position_secs:.2} exceeds duration {:.2}",
                self.duration_secs
            )));
        }

        self.with_mpv(|mpv| mpv.seek_absolute(position_secs))?;
        self.current_position_secs = position_secs;
        Ok(())
    }

    fn set_playback_rate(&mut self, playback_rate: f64) -> els_types::AppResult<()> {
        if !playback_rate.is_finite() || !(0.25..=2.0).contains(&playback_rate) {
            return Err(els_types::AppError::InvalidArgument(
                "playback rate must be between 0.25 and 2.0".to_string(),
            ));
        }

        self.with_mpv(|mpv| mpv.set_speed(playback_rate))?;
        self.playback_rate = playback_rate;
        Ok(())
    }

    fn playback_rate(&self) -> f64 {
        self.playback_rate
    }

    fn state(&self) -> PlayerState {
        self.state
    }
}

impl Player {
    pub fn loaded_path(&self) -> Option<&str> {
        self.loaded_path.as_deref()
    }

    pub fn current_position_secs(&self) -> f64 {
        self.current_position_secs
    }

    pub fn duration_secs(&self) -> f64 {
        self.duration_secs
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded_path.is_some()
    }

    pub fn prepare_initial_frame(&mut self, position_secs: f64) -> els_types::AppResult<()> {
        self.ensure_loaded()?;
        if !position_secs.is_finite() || position_secs.is_sign_negative() {
            return Err(els_types::AppError::InvalidArgument(
                "initial playback position must be a non-negative number".to_string(),
            ));
        }
        self.pending_initial_position_secs = Some(position_secs.min(self.duration_secs));
        Ok(())
    }

    pub fn is_preparing_initial_frame(&self) -> bool {
        self.pending_initial_position_secs.is_some()
    }

    pub fn backend_name(&self) -> &'static str {
        "libmpv (embedded)"
    }

    pub fn media_summary(&self) -> String {
        if let Some(error) = &self.backend_error {
            return format!("libmpv backend error: {error}");
        }

        match &self.media_probe {
            Some(probe) => format!(
                "Container: {} | Video: {} | Audio: {}",
                probe.container,
                probe.video_codec.as_deref().unwrap_or("unknown"),
                probe.audio_codec.as_deref().unwrap_or("unknown")
            ),
            None => "libmpv ready. No media loaded".to_string(),
        }
    }

    pub fn mpv_handle_value(&self) -> u64 {
        self.mpv
            .as_ref()
            .map(crate::mpv::MpvHandle::raw_handle_value)
            .unwrap_or(0)
    }

    pub fn sync_runtime_state(
        &mut self,
        position_secs: f64,
        duration_secs: f64,
        is_playing: bool,
    ) -> els_types::AppResult<()> {
        self.ensure_loaded()?;

        if position_secs.is_nan() || position_secs.is_sign_negative() {
            return Err(els_types::AppError::InvalidArgument(
                "runtime position must be a non-negative number".to_string(),
            ));
        }
        if duration_secs.is_nan() || duration_secs.is_sign_negative() {
            return Err(els_types::AppError::InvalidArgument(
                "runtime duration must be a non-negative number".to_string(),
            ));
        }

        self.duration_secs = if duration_secs > 0.0 {
            duration_secs
        } else {
            self.duration_secs
        };
        self.current_position_secs = if self.duration_secs > 0.0 {
            position_secs.min(self.duration_secs)
        } else {
            position_secs
        };
        self.state = if is_playing {
            PlayerState::Playing
        } else {
            PlayerState::Paused
        };
        Ok(())
    }

    pub fn tick(&mut self) -> els_types::AppResult<()> {
        if !self.is_loaded() {
            return Ok(());
        }

        self.sync_from_backend()?;
        self.try_prepare_initial_frame()
    }

    fn try_prepare_initial_frame(&mut self) -> els_types::AppResult<()> {
        let Some(target_position) = self.pending_initial_position_secs else {
            return Ok(());
        };
        let media_ready = self.with_mpv(|mpv| mpv.time_pos())?.is_some();
        if !media_ready {
            return Ok(());
        }

        self.with_mpv(|mpv| {
            mpv.set_pause(true)?;
            mpv.seek_absolute(target_position)
        })?;
        self.current_position_secs = target_position;
        self.state = PlayerState::Paused;
        self.pending_initial_position_secs = None;
        Ok(())
    }

    fn sync_from_backend(&mut self) -> els_types::AppResult<()> {
        let time_pos = self.with_mpv(|mpv| mpv.time_pos())?;
        let duration = self.with_mpv(|mpv| mpv.duration())?;
        let paused = self.with_mpv(|mpv| mpv.paused())?;

        if let Some(duration) = duration {
            self.duration_secs = duration;
        }
        if let Some(position) = time_pos {
            self.current_position_secs = if self.duration_secs > 0.0 {
                position.min(self.duration_secs)
            } else {
                position
            };
        }

        self.state = match paused {
            Some(true) => PlayerState::Paused,
            Some(false) => PlayerState::Playing,
            None if self.loaded_path.is_some() => PlayerState::Paused,
            None => PlayerState::Stopped,
        };

        Ok(())
    }

    fn ensure_loaded(&self) -> els_types::AppResult<()> {
        if self.is_loaded() {
            Ok(())
        } else {
            Err(els_types::AppError::InvalidArgument(
                "no video loaded".to_string(),
            ))
        }
    }

    fn ensure_mpv(&mut self) -> els_types::AppResult<&mut crate::mpv::MpvHandle> {
        if self.mpv.is_none() {
            match crate::mpv::MpvHandle::new_with_keep_open(self.keep_open) {
                Ok(mpv) => {
                    self.backend_error = None;
                    self.mpv = Some(mpv);
                }
                Err(err) => {
                    self.backend_error = Some(err.to_string());
                    return Err(err);
                }
            }
        }

        self.mpv
            .as_mut()
            .ok_or_else(|| els_types::AppError::Io("libmpv handle is unavailable".to_string()))
    }

    fn with_mpv<T>(
        &mut self,
        operation: impl FnOnce(&crate::mpv::MpvHandle) -> els_types::AppResult<T>,
    ) -> els_types::AppResult<T> {
        let mpv = self.ensure_mpv()?;
        operation(mpv)
    }
}

#[cfg(test)]
mod tests {
    use crate::{MediaController, PlaybackState, Player};

    #[test]
    fn loading_video_initializes_player_state() {
        let mut player = Player::default();
        player.loaded_path = Some("TED_AI_未来.mp4".to_string());
        player.duration_secs = 1470.0;
        player.state = PlaybackState::Paused;

        assert_eq!(player.loaded_path(), Some("TED_AI_未来.mp4"));
        assert_eq!(player.duration_secs(), 1470.0);
        assert_eq!(player.current_position_secs(), 0.0);
        assert_eq!(player.state(), PlaybackState::Paused);
    }

    #[test]
    fn play_pause_and_seek_update_state_machine() {
        let mut player = Player::default();
        player.loaded_path = Some("demo.mp4".to_string());
        player.duration_secs = 300.0;
        player.state = PlaybackState::Paused;
        player.current_position_secs = 42.5;
        player.state = PlaybackState::Playing;
        assert_eq!(player.state(), PlaybackState::Playing);

        player.current_position_secs = 42.5;
        assert!((player.current_position_secs() - 42.5).abs() < 0.01);

        player.state = PlaybackState::Paused;
        assert_eq!(player.state(), PlaybackState::Paused);
    }

    #[test]
    fn initial_frame_target_is_clamped_to_media_duration() {
        let mut player = Player::default();
        player.loaded_path = Some("demo.mp4".to_string());
        player.duration_secs = 300.0;

        player
            .prepare_initial_frame(420.0)
            .expect("prepare initial frame");

        assert!(player.is_preparing_initial_frame());
        assert_eq!(player.pending_initial_position_secs, Some(300.0));
    }

    #[test]
    fn runtime_sync_overrides_duration_and_position() {
        let mut player = Player::default();
        player.loaded_path = Some("demo.mp4".to_string());
        player.duration_secs = 300.0;
        player.state = PlaybackState::Paused;

        player
            .sync_runtime_state(12.0, 90.0, true)
            .expect("sync runtime state");

        assert_eq!(player.current_position_secs(), 12.0);
        assert_eq!(player.duration_secs(), 90.0);
        assert_eq!(player.state(), PlaybackState::Playing);
    }

    #[test]
    fn audio_player_keeps_the_loaded_file_open() {
        assert!(!Player::default().keep_open);
        assert!(Player::new_audio_playback().keep_open);
    }
}
