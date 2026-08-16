//! 录音播放与 QML 的薄适配层，使用独立选区时钟避免受 WAV 结尾状态影响。

use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use els_media_core::{MediaController, PlaybackState};
use els_recording_core::RecordingPlaybackTimeline;
use std::time::Instant;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(bool, has_recording, cxx_name = "hasRecording")]
        #[qproperty(bool, is_playing, cxx_name = "isPlaying")]
        #[qproperty(f64, current_position, cxx_name = "currentPosition")]
        #[qproperty(f64, duration)]
        #[qproperty(f64, playback_rate, cxx_name = "playbackRate")]
        #[qproperty(bool, has_playable_overlap, cxx_name = "hasPlayableOverlap")]
        #[qproperty(QString, loaded_path, cxx_name = "loadedPath")]
        #[qproperty(QString, status_message, cxx_name = "statusMessage")]
        type RecordingPlaybackBridge = super::RecordingPlaybackBridgeRust;

        #[qinvokable]
        #[cxx_name = "loadRecording"]
        fn load_recording(self: Pin<&mut RecordingPlaybackBridge>, path: &QString) -> bool;

        #[qinvokable]
        #[cxx_name = "configureTimeline"]
        fn configure_timeline(
            self: Pin<&mut RecordingPlaybackBridge>,
            selection_duration: f64,
            alignment_offset: f64,
        ) -> bool;

        #[qinvokable]
        fn unload(self: Pin<&mut RecordingPlaybackBridge>) -> bool;

        #[qinvokable]
        fn play(self: Pin<&mut RecordingPlaybackBridge>) -> bool;

        #[qinvokable]
        fn pause(self: Pin<&mut RecordingPlaybackBridge>) -> bool;

        #[qinvokable]
        fn seek(self: Pin<&mut RecordingPlaybackBridge>, position_secs: f64) -> bool;

        #[qinvokable]
        #[cxx_name = "applyPlaybackRate"]
        fn apply_playback_rate(self: Pin<&mut RecordingPlaybackBridge>, playback_rate: f64)
            -> bool;

        #[qinvokable]
        fn tick(self: Pin<&mut RecordingPlaybackBridge>) -> bool;
    }
}

pub struct RecordingPlaybackBridgeRust {
    has_recording: bool,
    is_playing: bool,
    current_position: f64,
    duration: f64,
    playback_rate: f64,
    has_playable_overlap: bool,
    loaded_path: QString,
    status_message: QString,
    player: els_media_core::Player,
    recording_duration: f64,
    timeline: Option<RecordingPlaybackTimeline>,
    timeline_anchor: f64,
    timeline_started_at: Option<Instant>,
    audio_is_playing: bool,
    seek_retry_pending: bool,
}

impl Default for RecordingPlaybackBridgeRust {
    fn default() -> Self {
        Self {
            has_recording: false,
            is_playing: false,
            current_position: 0.0,
            duration: 0.0,
            playback_rate: 1.0,
            has_playable_overlap: false,
            loaded_path: QString::from(""),
            status_message: QString::from("尚未加载录音"),
            player: els_media_core::Player::new_audio_playback(),
            recording_duration: 0.0,
            timeline: None,
            timeline_anchor: 0.0,
            timeline_started_at: None,
            audio_is_playing: false,
            seek_retry_pending: false,
        }
    }
}

impl RecordingPlaybackBridgeRust {
    fn timeline_position_now(&self) -> f64 {
        let elapsed = self
            .timeline_started_at
            .map(|started_at| started_at.elapsed().as_secs_f64() * self.playback_rate)
            .unwrap_or(0.0);
        self.timeline
            .map(|timeline| timeline.clamp_position(self.timeline_anchor + elapsed))
            .unwrap_or(0.0)
    }

    fn pause_audio(&mut self) -> els_types::AppResult<()> {
        if self.audio_is_playing || self.player.state() == PlaybackState::Playing {
            self.player.pause()?;
        }
        self.audio_is_playing = false;
        Ok(())
    }

    /// Returns `true` once playback can advance. A `false` result means mpv is
    /// still loading after a seek retry, so the selection clock must stay frozen.
    fn sync_audio(&mut self, timeline_position: f64, force_seek: bool) -> els_types::AppResult<bool> {
        let recording_position = self
            .timeline
            .and_then(|timeline| timeline.recording_position_at(timeline_position));
        if !self.is_playing || recording_position.is_none() {
            self.seek_retry_pending = false;
            self.pause_audio()?;
            return Ok(true);
        }

        let recording_position = recording_position.unwrap_or_default();
        if force_seek || !self.audio_is_playing {
            if let Err(error) = self.player.seek(recording_position) {
                if !is_time_position_unavailable(&error) {
                    return Err(error);
                }
                self.audio_is_playing = false;
                if !self.seek_retry_pending {
                    let path = self.loaded_path.to_string();
                    self.player.load(&path)?;
                    self.seek_retry_pending = true;
                }
                return Ok(false);
            }
            self.player.play()?;
            self.audio_is_playing = true;
            self.seek_retry_pending = false;
        }
        Ok(true)
    }
}

impl qobject::RecordingPlaybackBridge {
    fn load_recording(mut self: Pin<&mut Self>, path: &QString) -> bool {
        let path = path.to_string();
        if path.trim().is_empty() {
            return self.as_mut().unload();
        }
        if self.rust().loaded_path.to_string() == path && self.rust().has_recording {
            return true;
        }

        match self.as_mut().rust_mut().player.load(&path) {
            Ok(()) => {
                let recording_duration = self.rust().player.duration_secs();
                self.as_mut().rust_mut().recording_duration = recording_duration;
                self.as_mut().rust_mut().timeline = None;
                self.as_mut().rust_mut().timeline_anchor = 0.0;
                self.as_mut().rust_mut().timeline_started_at = None;
                self.as_mut().rust_mut().audio_is_playing = false;
                self.as_mut().rust_mut().seek_retry_pending = false;
                self.as_mut().set_current_position(0.0);
                self.as_mut().set_duration(recording_duration);
                self.as_mut().set_has_playable_overlap(false);
                self.as_mut().set_is_playing(false);
                self.as_mut().set_has_recording(true);
                self.as_mut().set_loaded_path(QString::from(&path));
                self.as_mut()
                    .set_status_message(QString::from("录音已就绪"));
                true
            }
            Err(error) => self.as_mut().report_error("加载录音失败", error),
        }
    }

    fn configure_timeline(
        mut self: Pin<&mut Self>,
        selection_duration: f64,
        alignment_offset: f64,
    ) -> bool {
        if !self.rust().has_recording {
            return false;
        }
        let timeline = match RecordingPlaybackTimeline::new(
            selection_duration,
            self.rust().recording_duration,
            alignment_offset,
        ) {
            Ok(timeline) => timeline,
            Err(error) => return self.as_mut().report_error("设置录音播放范围失败", error),
        };

        let was_playing = self.rust().is_playing;
        let position = self.rust().timeline_position_now();
        let position = timeline.clamp_position(position);
        self.as_mut().rust_mut().timeline = Some(timeline);
        self.as_mut().rust_mut().timeline_anchor = position;
        self.as_mut().rust_mut().timeline_started_at = None;
        self.as_mut().set_current_position(position);
        self.as_mut().set_duration(timeline.selection_duration());
        self.as_mut().set_has_playable_overlap(timeline.has_overlap());
        match self.as_mut().rust_mut().sync_audio(position, true) {
            Ok(true) if was_playing => {
                self.as_mut().rust_mut().timeline_started_at = Some(Instant::now())
            }
            Ok(_) => {}
            Err(error) => return self.as_mut().report_error("更新录音对齐失败", error),
        }
        true
    }

    fn unload(mut self: Pin<&mut Self>) -> bool {
        if !self.rust().has_recording && self.rust().loaded_path.to_string().is_empty() {
            return true;
        }
        self.as_mut().rust_mut().player = els_media_core::Player::new_audio_playback();
        self.as_mut().rust_mut().recording_duration = 0.0;
        self.as_mut().rust_mut().timeline = None;
        self.as_mut().rust_mut().timeline_anchor = 0.0;
        self.as_mut().rust_mut().timeline_started_at = None;
        self.as_mut().rust_mut().audio_is_playing = false;
        self.as_mut().rust_mut().seek_retry_pending = false;
        self.as_mut().set_has_recording(false);
        self.as_mut().set_is_playing(false);
        self.as_mut().set_current_position(0.0);
        self.as_mut().set_duration(0.0);
        self.as_mut().set_playback_rate(1.0);
        self.as_mut().set_has_playable_overlap(false);
        self.as_mut().set_loaded_path(QString::from(""));
        self.as_mut()
            .set_status_message(QString::from("尚未加载录音"));
        true
    }

    fn play(mut self: Pin<&mut Self>) -> bool {
        if !self.rust().has_recording || self.rust().timeline.is_none() {
            return false;
        }
        let position = self.rust().current_position;
        self.as_mut().rust_mut().timeline_anchor = position;
        self.as_mut().rust_mut().timeline_started_at = None;
        self.as_mut().set_is_playing(true);
        match self.as_mut().rust_mut().sync_audio(position, true) {
            Ok(true) => self.as_mut().rust_mut().timeline_started_at = Some(Instant::now()),
            Ok(false) => {}
            Err(error) => return self.as_mut().report_error("播放录音失败", error),
        }
        self.as_mut()
            .set_status_message(QString::from("正在播放录音"));
        true
    }

    fn pause(mut self: Pin<&mut Self>) -> bool {
        if !self.rust().has_recording {
            return false;
        }
        let position = self.rust().timeline_position_now();
        self.as_mut().rust_mut().timeline_anchor = position;
        self.as_mut().rust_mut().timeline_started_at = None;
        self.as_mut().set_current_position(position);
        self.as_mut().set_is_playing(false);
        if let Err(error) = self.as_mut().rust_mut().pause_audio() {
            return self.as_mut().report_error("暂停录音失败", error);
        }
        self.as_mut()
            .set_status_message(QString::from("录音已暂停"));
        true
    }

    fn seek(mut self: Pin<&mut Self>, position_secs: f64) -> bool {
        let timeline = match self.rust().timeline {
            Some(timeline) => timeline,
            None => return false,
        };
        let position = timeline.clamp_position(position_secs);
        let is_playing = self.rust().is_playing;
        self.as_mut().rust_mut().timeline_anchor = position;
        self.as_mut().rust_mut().timeline_started_at = None;
        self.as_mut().set_current_position(position);
        match self.as_mut().rust_mut().sync_audio(position, true) {
            Ok(true) if is_playing => {
                self.as_mut().rust_mut().timeline_started_at = Some(Instant::now())
            }
            Ok(_) => {}
            Err(error) => return self.as_mut().report_error("定位录音失败", error),
        }
        true
    }

    fn apply_playback_rate(mut self: Pin<&mut Self>, playback_rate: f64) -> bool {
        if !self.rust().has_recording {
            return false;
        }
        let position = self.rust().timeline_position_now();
        match self
            .as_mut()
            .rust_mut()
            .player
            .set_playback_rate(playback_rate)
        {
            Ok(()) => {
                let is_playing = self.rust().is_playing;
                let clock_was_running = self.rust().timeline_started_at.is_some();
                self.as_mut().rust_mut().timeline_anchor = position;
                self.as_mut().rust_mut().timeline_started_at =
                    (is_playing && clock_was_running).then(Instant::now);
                self.as_mut().set_current_position(position);
                self.as_mut().set_playback_rate(playback_rate);
                true
            }
            Err(error) => self.as_mut().report_error("设置录音播放速率失败", error),
        }
    }

    fn tick(mut self: Pin<&mut Self>) -> bool {
        if !self.rust().has_recording || !self.rust().is_playing {
            return false;
        }
        let timeline = match self.rust().timeline {
            Some(timeline) => timeline,
            None => return false,
        };
        let position = self.rust().timeline_position_now();
        if self.rust().timeline_started_at.is_none() {
            match self.as_mut().rust_mut().sync_audio(position, false) {
                Ok(true) => {
                    self.as_mut().rust_mut().timeline_anchor = position;
                    self.as_mut().rust_mut().timeline_started_at = Some(Instant::now());
                    self.as_mut().set_current_position(position);
                    return true;
                }
                Ok(false) => return true,
                Err(error) => {
                    return self.as_mut().report_error("准备录音播放失败", error);
                }
            }
        }
        let completed = position >= timeline.selection_duration() - 0.001;

        if completed {
            self.as_mut().rust_mut().timeline_anchor = timeline.selection_duration();
            self.as_mut().rust_mut().timeline_started_at = None;
            if let Err(error) = self.as_mut().rust_mut().pause_audio() {
                return self.as_mut().report_error("完成录音播放失败", error);
            }
            self.as_mut()
                .set_current_position(timeline.selection_duration());
            self.as_mut().set_is_playing(false);
            return true;
        }

        if self.rust().audio_is_playing {
            if let Err(error) = self.as_mut().rust_mut().player.tick() {
                return self.as_mut().report_error("刷新录音播放器状态失败", error);
            }
            if self.rust().player.state() != PlaybackState::Playing {
                self.as_mut().rust_mut().audio_is_playing = false;
            }
        }
        if let Err(error) = self.as_mut().rust_mut().sync_audio(position, false) {
            return self.as_mut().report_error("同步录音播放状态失败", error);
        }
        self.as_mut().set_current_position(position);
        true
    }
}

impl qobject::RecordingPlaybackBridge {
    fn report_error(mut self: Pin<&mut Self>, context: &str, error: els_types::AppError) -> bool {
        let message = format!("{context}：{error}");
        eprintln!("{message}");
        self.as_mut().rust_mut().timeline_started_at = None;
        self.as_mut().rust_mut().audio_is_playing = false;
        self.as_mut().rust_mut().seek_retry_pending = false;
        self.as_mut().set_is_playing(false);
        self.as_mut().set_status_message(QString::from(&message));
        false
    }
}

fn is_time_position_unavailable(error: &els_types::AppError) -> bool {
    let message = error.to_string();
    message.contains("time-pos") && message.contains("property unavailable")
}
