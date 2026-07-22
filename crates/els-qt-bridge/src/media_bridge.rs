//! 包装 `els_media_core::Player`，把 FFmpeg / ffplay 后端暴露给 QML。
//! 当前阶段用 `ffprobe` 探测媒体信息，用 `ffplay` 以外部窗口方式实际播放。

use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use els_media_core::MediaController;
use std::path::{Path, PathBuf};
use std::process::Command;

const DEMO_VIDEO_PATH: &str = "TED_AI_未来.mp4";

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, loaded_path, cxx_name = "loadedPath")]
        #[qproperty(bool, is_playing, cxx_name = "isPlaying")]
        #[qproperty(f64, current_position, cxx_name = "currentPosition")]
        #[qproperty(f64, duration)]
        #[qproperty(f64, playback_rate, cxx_name = "playbackRate")]
        #[qproperty(QString, status_message, cxx_name = "statusMessage")]
        #[qproperty(QString, backend_name, cxx_name = "backendName")]
        #[qproperty(QString, media_summary, cxx_name = "mediaSummary")]
        #[qproperty(QString, mpv_handle_token, cxx_name = "mpvHandleToken")]
        type MediaBridge = super::MediaBridgeRust;

        #[qinvokable]
        #[cxx_name = "loadDemoVideo"]
        fn load_demo_video(self: Pin<&mut MediaBridge>) -> bool;

        #[qinvokable]
        #[cxx_name = "loadVideoPath"]
        fn load_video_path(self: Pin<&mut MediaBridge>, path: &QString) -> bool;

        #[qinvokable]
        #[cxx_name = "pickVideoFile"]
        fn pick_video_file(self: Pin<&mut MediaBridge>) -> QString;

        #[qinvokable]
        fn play(self: Pin<&mut MediaBridge>) -> bool;

        #[qinvokable]
        fn pause(self: Pin<&mut MediaBridge>) -> bool;

        #[qinvokable]
        fn seek(self: Pin<&mut MediaBridge>, position_secs: f64) -> bool;

        #[qinvokable]
        #[cxx_name = "applyPlaybackRate"]
        fn apply_playback_rate(self: Pin<&mut MediaBridge>, playback_rate: f64) -> bool;

        #[qinvokable]
        #[cxx_name = "syncRuntimeState"]
        fn sync_runtime_state(
            self: Pin<&mut MediaBridge>,
            position_secs: f64,
            duration_secs: f64,
            is_playing: bool,
        ) -> bool;

        #[qinvokable]
        fn tick(self: Pin<&mut MediaBridge>) -> bool;
    }
}

pub struct MediaBridgeRust {
    loaded_path: QString,
    is_playing: bool,
    current_position: f64,
    duration: f64,
    playback_rate: f64,
    status_message: QString,
    backend_name: QString,
    media_summary: QString,
    mpv_handle_token: QString,
    player: els_media_core::Player,
}

impl Default for MediaBridgeRust {
    fn default() -> Self {
        let player = els_media_core::Player::default();
        let mpv_handle = player.mpv_handle_value();
        let status_message = if mpv_handle == 0 {
            QString::from("libmpv backend is unavailable")
        } else {
            QString::from("libmpv embedded backend ready")
        };

        Self {
            loaded_path: QString::from(""),
            is_playing: false,
            current_position: 0.0,
            duration: 0.0,
            playback_rate: player.playback_rate(),
            status_message,
            backend_name: QString::from(player.backend_name()),
            media_summary: QString::from(&player.media_summary()),
            mpv_handle_token: QString::from(&mpv_handle.to_string()),
            player,
        }
    }
}

impl qobject::MediaBridge {
    fn load_demo_video(mut self: Pin<&mut Self>) -> bool {
        let demo_path = resolve_video_path(DEMO_VIDEO_PATH);
        self.as_mut()
            .load_video_path(&QString::from(&demo_path.to_string_lossy().to_string()))
    }

    fn load_video_path(mut self: Pin<&mut Self>, path: &QString) -> bool {
        let requested_path = path.to_string();
        let normalized_path = normalize_video_path(&requested_path);
        let normalized_string = normalized_path.to_string_lossy().to_string();

        let result = self.as_mut().rust_mut().player.load(&normalized_string);
        match result {
            Ok(()) => {
                let file_status = if normalized_path.exists() {
                    "Video loaded"
                } else {
                    "Video path registered (file not found in current environment)"
                };
                self.as_mut().sync_from_player(file_status);
                true
            }
            Err(err) => {
                let message = format!("Failed to load video path: {err}");
                eprintln!("{message}");
                self.as_mut().set_status_message(QString::from(&message));
                false
            }
        }
    }

    fn pick_video_file(mut self: Pin<&mut Self>) -> QString {
        match pick_video_file_path() {
            Ok(Some(path)) => {
                let selected = path.to_string_lossy().to_string();
                self.as_mut()
                    .set_status_message(QString::from("File selected"));
                QString::from(&selected)
            }
            Ok(None) => {
                self.as_mut()
                    .set_status_message(QString::from("File selection cancelled"));
                QString::from("")
            }
            Err(err) => {
                let message = format!("Failed to open file picker: {err}");
                eprintln!("{message}");
                self.as_mut().set_status_message(QString::from(&message));
                QString::from("")
            }
        }
    }

    fn play(mut self: Pin<&mut Self>) -> bool {
        let result = self.as_mut().rust_mut().player.play();
        match result {
            Ok(()) => {
                self.as_mut().sync_from_player("Playing");
                true
            }
            Err(err) => {
                let message = format!("Failed to play: {err}");
                eprintln!("{message}");
                self.as_mut().set_status_message(QString::from(&message));
                false
            }
        }
    }

    fn pause(mut self: Pin<&mut Self>) -> bool {
        let result = self.as_mut().rust_mut().player.pause();
        match result {
            Ok(()) => {
                self.as_mut().sync_from_player("Paused");
                true
            }
            Err(err) => {
                let message = format!("Failed to pause: {err}");
                eprintln!("{message}");
                self.as_mut().set_status_message(QString::from(&message));
                false
            }
        }
    }

    fn seek(mut self: Pin<&mut Self>, position_secs: f64) -> bool {
        let result = self.as_mut().rust_mut().player.seek(position_secs);
        match result {
            Ok(()) => {
                self.as_mut().sync_from_player("Seeked");
                true
            }
            Err(err) => {
                let message = format!("Failed to seek: {err}");
                eprintln!("{message}");
                self.as_mut().set_status_message(QString::from(&message));
                false
            }
        }
    }

    fn apply_playback_rate(mut self: Pin<&mut Self>, playback_rate: f64) -> bool {
        match self
            .as_mut()
            .rust_mut()
            .player
            .set_playback_rate(playback_rate)
        {
            Ok(()) => {
                self.as_mut().set_playback_rate(playback_rate);
                self.as_mut().set_status_message(QString::from(&format!(
                    "Playback rate: {playback_rate:.2}x"
                )));
                true
            }
            Err(err) => {
                let message = format!("Failed to set playback rate: {err}");
                eprintln!("{message}");
                self.as_mut().set_status_message(QString::from(&message));
                false
            }
        }
    }

    fn sync_runtime_state(
        mut self: Pin<&mut Self>,
        position_secs: f64,
        duration_secs: f64,
        is_playing: bool,
    ) -> bool {
        let result = self.as_mut().rust_mut().player.sync_runtime_state(
            position_secs,
            duration_secs,
            is_playing,
        );
        match result {
            Ok(()) => {
                let status = if is_playing { "Playing" } else { "Ready" };
                self.as_mut().sync_from_player(status);
                true
            }
            Err(err) => {
                let message = format!("Failed to sync runtime state: {err}");
                eprintln!("{message}");
                self.as_mut().set_status_message(QString::from(&message));
                false
            }
        }
    }

    fn tick(mut self: Pin<&mut Self>) -> bool {
        let result = self.as_mut().rust_mut().player.tick();
        match result {
            Ok(()) => {
                let status = if self.rust().player.state() == els_media_core::PlaybackState::Playing
                {
                    "Playing"
                } else {
                    "Paused"
                };
                self.as_mut().sync_from_player(status);
                true
            }
            Err(err) => {
                let message = format!("Failed to tick player: {err}");
                eprintln!("{message}");
                self.as_mut().set_status_message(QString::from(&message));
                false
            }
        }
    }
}

impl qobject::MediaBridge {
    fn sync_from_player(mut self: Pin<&mut Self>, status_message: &str) {
        let loaded_path = self
            .rust()
            .player
            .loaded_path()
            .map(QString::from)
            .unwrap_or_else(|| QString::from(""));
        let is_playing = self.rust().player.state() == els_media_core::PlaybackState::Playing;
        let current_position = self.rust().player.current_position_secs();
        let duration = self.rust().player.duration_secs();
        let playback_rate = self.rust().player.playback_rate();
        let backend_name = QString::from(self.rust().player.backend_name());
        let media_summary = QString::from(&self.rust().player.media_summary());
        let mpv_handle_token = QString::from(&self.rust().player.mpv_handle_value().to_string());

        self.as_mut().set_loaded_path(loaded_path);
        self.as_mut().set_is_playing(is_playing);
        self.as_mut().set_current_position(current_position);
        self.as_mut().set_duration(duration);
        self.as_mut().set_playback_rate(playback_rate);
        self.as_mut()
            .set_status_message(QString::from(status_message));
        self.as_mut().set_backend_name(backend_name);
        self.as_mut().set_media_summary(media_summary);
        self.as_mut().set_mpv_handle_token(mpv_handle_token);
    }
}

fn normalize_video_path(path: &str) -> PathBuf {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return PathBuf::new();
    }

    let candidate = PathBuf::from(trimmed);
    if candidate.is_absolute() {
        candidate
    } else {
        resolve_video_path(trimmed)
    }
}

fn resolve_video_path(path: &str) -> PathBuf {
    let candidate = PathBuf::from(path);
    if candidate.is_absolute() {
        candidate
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(Path::new(path))
    }
}

fn pick_video_file_path() -> Result<Option<PathBuf>, String> {
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("osascript")
            .args([
                "-e",
                r#"POSIX path of (choose file with prompt "Select a video file for Language Learning Studio")"#,
            ])
            .output()
            .map_err(|err| err.to_string())?;

        if output.status.success() {
            let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if selected.is_empty() {
                Ok(None)
            } else {
                Ok(Some(PathBuf::from(selected)))
            }
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("User canceled") || stderr.contains("-128") {
                Ok(None)
            } else {
                Err(stderr.trim().to_string())
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok(None)
    }
}
