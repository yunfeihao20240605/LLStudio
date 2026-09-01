//! 基于 FFmpeg CLI 的媒体探测模块。
//! 当前阶段通过 `ffprobe` 提取容器、时长、音视频编码等信息，
//! 为后续接入更底层的 libav* 播放/解码链路做准备。

use serde::Deserialize;
use std::path::PathBuf;
use std::process::Command;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone, PartialEq)]
pub struct MediaProbe {
    pub duration_secs: f64,
    pub container: String,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
}

#[derive(Default)]
pub struct Decoder;

impl Decoder {
    pub fn probe(path: &str) -> els_types::AppResult<MediaProbe> {
        if path.trim().is_empty() {
            return Err(els_types::AppError::InvalidArgument(
                "video path must not be empty".to_string(),
            ));
        }

        let mut command = Command::new(ffprobe_cli_path());
        hide_console_window(&mut command);
        let output = command
            .args([
                "-v",
                "error",
                "-print_format",
                "json",
                "-show_format",
                "-show_streams",
                path,
            ])
            .output()
            .map_err(|err| els_types::AppError::Io(format!("failed to run ffprobe: {err}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(els_types::AppError::Io(if stderr.is_empty() {
                "ffprobe failed without stderr output".to_string()
            } else {
                stderr
            }));
        }

        let json = String::from_utf8(output.stdout).map_err(|err| {
            els_types::AppError::Io(format!("ffprobe output was not valid utf-8: {err}"))
        })?;
        Self::parse_probe_json(&json)
    }

    fn parse_probe_json(json: &str) -> els_types::AppResult<MediaProbe> {
        let parsed: FfprobeOutput = serde_json::from_str(json).map_err(|err| {
            els_types::AppError::Io(format!("failed to parse ffprobe json: {err}"))
        })?;

        let duration_secs = parsed
            .format
            .duration
            .as_deref()
            .unwrap_or("0")
            .parse::<f64>()
            .map_err(|err| els_types::AppError::Io(format!("invalid ffprobe duration: {err}")))?;

        let video_codec = parsed
            .streams
            .iter()
            .find(|stream| stream.codec_type.as_deref() == Some("video"))
            .and_then(|stream| stream.codec_name.clone());
        let audio_codec = parsed
            .streams
            .iter()
            .find(|stream| stream.codec_type.as_deref() == Some("audio"))
            .and_then(|stream| stream.codec_name.clone());

        Ok(MediaProbe {
            duration_secs,
            container: parsed
                .format
                .format_name
                .unwrap_or_else(|| "unknown".to_string()),
            video_codec,
            audio_codec,
        })
    }
}

#[cfg(target_os = "windows")]
fn hide_console_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;

    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(target_os = "windows"))]
fn hide_console_window(_command: &mut Command) {}

fn ffprobe_cli_path() -> PathBuf {
    if let Ok(value) = std::env::var("ELS_FFPROBE_BIN") {
        return PathBuf::from(value);
    }

    if let Ok(executable) = std::env::current_exe() {
        if let Some(directory) = executable.parent() {
            let bundled = directory.join(if cfg!(target_os = "windows") {
                "ffprobe.exe"
            } else {
                "ffprobe"
            });
            if bundled.exists() {
                return bundled;
            }
        }
    }

    PathBuf::from(if cfg!(target_os = "windows") {
        "ffprobe.exe"
    } else {
        "ffprobe"
    })
}

#[derive(Debug, Deserialize)]
struct FfprobeOutput {
    #[serde(default)]
    streams: Vec<FfprobeStream>,
    format: FfprobeFormat,
}

#[derive(Debug, Deserialize)]
struct FfprobeStream {
    codec_name: Option<String>,
    codec_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FfprobeFormat {
    duration: Option<String>,
    format_name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{Decoder, MediaProbe};

    #[test]
    fn parses_ffprobe_json() {
        let json = r#"{
            "streams": [
                {"codec_name": "h264", "codec_type": "video"},
                {"codec_name": "aac", "codec_type": "audio"}
            ],
            "format": {
                "duration": "123.456",
                "format_name": "matroska,webm"
            }
        }"#;

        let probe = Decoder::parse_probe_json(json).expect("parse ffprobe json");
        assert_eq!(
            probe,
            MediaProbe {
                duration_secs: 123.456,
                container: "matroska,webm".to_string(),
                video_codec: Some("h264".to_string()),
                audio_codec: Some("aac".to_string()),
            }
        );
    }
}
