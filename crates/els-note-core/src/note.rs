#[derive(Clone, Debug, PartialEq)]
pub struct NewNote {
    pub video_id: i64,
    pub start_time: f64,
    pub end_time: Option<f64>,
    pub content: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    pub id: i64,
    pub video_id: i64,
    pub start_time: f64,
    pub end_time: Option<f64>,
    pub content: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NoteSummary {
    pub id: i64,
    pub start_time: f64,
    pub end_time: Option<f64>,
    pub preview: String,
    pub updated_at: i64,
}
