//! 波形分析数据结构。

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WaveformBin {
    pub min: f32,
    pub max: f32,
}

/// 波形数据。每个桶代表一个时间段内的最小/最大振幅。
#[derive(Debug, Default, Clone)]
pub struct WaveformData {
    pub duration_secs: f64,
    pub bins: Vec<WaveformBin>,
}
