use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoiseReductionProfile {
    Light,
    Standard,
    Strong,
}

impl NoiseReductionProfile {
    pub fn id(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Standard => "standard",
            Self::Strong => "strong",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "light" => Some(Self::Light),
            "standard" => Some(Self::Standard),
            "strong" => Some(Self::Strong),
            _ => None,
        }
    }

    fn filter(self) -> &'static str {
        match self {
            Self::Light => "highpass=f=70,afftdn=nf=-20",
            Self::Standard => "highpass=f=80,afftdn=nf=-25",
            Self::Strong => "highpass=f=100,afftdn=nf=-30",
        }
    }
}

pub fn denoised_path(original_path: &Path, profile: NoiseReductionProfile) -> PathBuf {
    let stem = original_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("recording");
    original_path.with_file_name(format!("{stem}-denoised-{}.wav", profile.id()))
}

pub fn process_recording(
    original_path: &Path,
    profile: NoiseReductionProfile,
    sample_rate: u32,
) -> els_types::AppResult<PathBuf> {
    if !original_path.exists() || sample_rate == 0 {
        return Err(els_types::AppError::InvalidArgument(
            "录音文件或采样率无效".to_string(),
        ));
    }
    let output_path = denoised_path(original_path, profile);
    let output = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
        .arg(original_path)
        .args(["-af", profile.filter(), "-ac", "1", "-ar"])
        .arg(sample_rate.to_string())
        .args(["-c:a", "pcm_s16le"])
        .arg(&output_path)
        .output()
        .map_err(|error| els_types::AppError::Io(format!("启动 FFmpeg 降噪失败：{error}")))?;
    if !output.status.success() {
        let _ = std::fs::remove_file(&output_path);
        return Err(els_types::AppError::Io(format!(
            "录音降噪失败：{}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(output_path)
}

#[cfg(test)]
mod tests {
    use super::{denoised_path, NoiseReductionProfile};
    use std::path::Path;

    #[test]
    fn derives_distinct_variant_paths() {
        assert_eq!(
            denoised_path(Path::new("/tmp/voice.wav"), NoiseReductionProfile::Standard),
            Path::new("/tmp/voice-denoised-standard.wav")
        );
        assert_eq!(NoiseReductionProfile::parse("strong"), Some(NoiseReductionProfile::Strong));
        assert_eq!(NoiseReductionProfile::parse("off"), None);
    }
}
