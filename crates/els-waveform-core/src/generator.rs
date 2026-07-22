//! 波形生成输入类型。

use crate::WaveformQuality;

/// 波形数据生成参数。
#[derive(Debug, Clone)]
pub struct AudioSource {
    pub video_path: Option<String>,
    pub duration_secs: f64,
    pub quality: WaveformQuality,
}

impl Default for AudioSource {
    fn default() -> Self {
        Self {
            video_path: None,
            duration_secs: 180.0,
            quality: WaveformQuality::Preview,
        }
    }
}
