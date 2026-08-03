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
    pub created_at: i64,
}
