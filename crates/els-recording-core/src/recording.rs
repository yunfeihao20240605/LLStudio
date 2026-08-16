use els_types::TimeRange;

#[derive(Clone, Debug, PartialEq)]
pub struct NewRecording {
    pub video_id: i64,
    pub range: TimeRange,
    pub file_path: String,
    pub duration_secs: f64,
    pub sample_rate: u32,
    pub alignment_offset: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Recording {
    pub id: i64,
    pub video_id: i64,
    pub range: TimeRange,
    pub file_path: String,
    pub duration_secs: f64,
    pub sample_rate: u32,
    pub alignment_offset: f64,
    pub active_variant: String,
    pub denoised_light_path: Option<String>,
    pub denoised_standard_path: Option<String>,
    pub denoised_strong_path: Option<String>,
    pub created_at: i64,
}

impl Recording {
    pub fn active_file_path(&self) -> &str {
        match self.active_variant.as_str() {
            "light" => self.denoised_light_path.as_deref(),
            "standard" => self.denoised_standard_path.as_deref(),
            "strong" => self.denoised_strong_path.as_deref(),
            _ => None,
        }
        .unwrap_or(&self.file_path)
    }

    pub fn variant_path(&self, variant: &str) -> Option<&str> {
        match variant {
            "original" => Some(&self.file_path),
            "light" => self.denoised_light_path.as_deref(),
            "standard" => self.denoised_standard_path.as_deref(),
            "strong" => self.denoised_strong_path.as_deref(),
            _ => None,
        }
    }

    pub fn all_file_paths(&self) -> Vec<&str> {
        let mut paths = vec![self.file_path.as_str()];
        for path in [
            self.denoised_light_path.as_deref(),
            self.denoised_standard_path.as_deref(),
            self.denoised_strong_path.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            if !paths.contains(&path) {
                paths.push(path);
            }
        }
        paths
    }
}
