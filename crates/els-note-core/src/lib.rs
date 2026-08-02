//! 视频时间笔记的核心模型、业务规则和持久化端口。

mod manager;
mod note;

pub use manager::{active_note_index, DefaultNoteManager, NoteManager};
pub use note::{NewNote, Note, NoteSummary};

pub trait NoteRepository {
    fn ensure_video(
        &mut self,
        path: &str,
        title: &str,
        duration_secs: f64,
    ) -> els_types::AppResult<i64>;
    fn insert(&mut self, note: &NewNote) -> els_types::AppResult<i64>;
    fn find_summaries_by_video(&self, video_id: i64) -> els_types::AppResult<Vec<NoteSummary>>;
    fn find_by_id(&self, note_id: i64, video_id: i64) -> els_types::AppResult<Note>;
    fn update_content(
        &mut self,
        note_id: i64,
        video_id: i64,
        content: &str,
    ) -> els_types::AppResult<()>;
    fn delete(&mut self, note_id: i64, video_id: i64) -> els_types::AppResult<()>;
}
