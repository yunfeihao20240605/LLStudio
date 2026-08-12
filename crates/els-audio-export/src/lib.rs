use els_speech_core::{AudioFormat, AudioInput, SpeechError};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct ExportedAudio {
    input: AudioInput,
}

impl ExportedAudio {
    pub fn input(&self) -> &AudioInput {
        &self.input
    }
}

impl Drop for ExportedAudio {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.input.path);
    }
}

pub fn export_video_range(
    video_path: &str,
    range: els_types::TimeRange,
    format: AudioFormat,
    sample_rate: u32,
    channels: u16,
) -> Result<ExportedAudio, SpeechError> {
    if video_path.trim().is_empty() || !Path::new(video_path).exists() {
        return Err(SpeechError::Audio("视频文件不存在".to_string()));
    }
    if !range.start.is_finite() || !range.end.is_finite() || range.end <= range.start {
        return Err(SpeechError::Audio("当前选区无效".to_string()));
    }
    let output_path = temporary_audio_path(format);
    let duration = range.end - range.start;
    let mut command = Command::new("ffmpeg");
    command
        .args(["-hide_banner", "-loglevel", "error", "-y", "-ss"])
        .arg(format!("{:.6}", range.start))
        .args(["-t"])
        .arg(format!("{duration:.6}"))
        .args(["-i", video_path, "-vn", "-ac"])
        .arg(channels.to_string())
        .arg("-ar")
        .arg(sample_rate.to_string())
        .args(["-c:a", "pcm_s16le"]);
    if format == AudioFormat::RawPcm16 {
        command.args(["-f", "s16le"]);
    }
    let output = command
        .arg(&output_path)
        .output()
        .map_err(|error| SpeechError::Audio(format!("启动 FFmpeg 失败：{error}")))?;
    if !output.status.success() {
        let _ = std::fs::remove_file(&output_path);
        return Err(SpeechError::Audio(format!(
            "提取当前片段音频失败：{}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(ExportedAudio {
        input: AudioInput {
            path: output_path,
            format,
        },
    })
}

fn temporary_audio_path(format: AudioFormat) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos())
        .unwrap_or_default();
    let extension = match format {
        AudioFormat::WavPcm16 => "wav",
        AudioFormat::RawPcm16 => "pcm",
    };
    std::env::temp_dir().join(format!(
        "language-learning-studio-asr-{}-{nonce}.{extension}",
        std::process::id()
    ))
}
