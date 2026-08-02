//! 视频时间笔记与 QML 的薄适配层。

use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use els_note_core::NoteManager;
use std::path::Path;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(i32, note_count, cxx_name = "noteCount")]
        #[qproperty(i32, revision)]
        #[qproperty(i32, editing_note_index, cxx_name = "editingNoteIndex")]
        #[qproperty(i32, playback_note_index, cxx_name = "playbackNoteIndex")]
        #[qproperty(QString, editing_note_content, cxx_name = "editingNoteContent")]
        #[qproperty(f64, editing_note_start, cxx_name = "editingNoteStart")]
        #[qproperty(f64, editing_note_end, cxx_name = "editingNoteEnd")]
        #[qproperty(bool, editing_note_has_range, cxx_name = "editingNoteHasRange")]
        #[qproperty(bool, has_video, cxx_name = "hasVideo")]
        #[qproperty(QString, status_message, cxx_name = "statusMessage")]
        type NoteBridge = super::NoteBridgeRust;

        #[qinvokable]
        #[cxx_name = "loadForVideoPath"]
        fn load_for_video_path(
            self: Pin<&mut NoteBridge>,
            path: &QString,
            duration_secs: f64,
        ) -> bool;

        #[qinvokable]
        #[cxx_name = "createForRange"]
        fn create_for_range(self: Pin<&mut NoteBridge>, start_secs: f64, end_secs: f64) -> bool;

        #[qinvokable]
        #[cxx_name = "createAtPosition"]
        fn create_at_position(self: Pin<&mut NoteBridge>, position_secs: f64) -> bool;

        #[qinvokable]
        #[cxx_name = "selectNote"]
        fn select_note(self: Pin<&mut NoteBridge>, index: i32) -> bool;

        #[qinvokable]
        #[cxx_name = "updateActiveNote"]
        fn update_active_note(self: Pin<&mut NoteBridge>, content: &QString) -> bool;

        #[qinvokable]
        #[cxx_name = "deleteNote"]
        fn delete_note(self: Pin<&mut NoteBridge>, index: i32) -> bool;

        #[qinvokable]
        #[cxx_name = "syncPlaybackPosition"]
        fn sync_playback_position(self: Pin<&mut NoteBridge>, position_secs: f64) -> bool;

        #[qinvokable]
        #[cxx_name = "noteStartAt"]
        fn note_start_at(&self, index: i32) -> f64;

        #[qinvokable]
        #[cxx_name = "noteEndAt"]
        fn note_end_at(&self, index: i32) -> f64;

        #[qinvokable]
        #[cxx_name = "noteHasRangeAt"]
        fn note_has_range_at(&self, index: i32) -> bool;

        #[qinvokable]
        #[cxx_name = "notePreviewAt"]
        fn note_preview_at(&self, index: i32) -> QString;
    }
}

type NotesManager = els_note_core::DefaultNoteManager<els_storage::SqliteNoteRepository>;

pub struct NoteBridgeRust {
    note_count: i32,
    revision: i32,
    editing_note_index: i32,
    playback_note_index: i32,
    editing_note_content: QString,
    editing_note_start: f64,
    editing_note_end: f64,
    editing_note_has_range: bool,
    has_video: bool,
    status_message: QString,
    manager: NotesManager,
    current_video_id: Option<i64>,
    summaries: Vec<els_note_core::NoteSummary>,
}

impl Default for NoteBridgeRust {
    fn default() -> Self {
        Self {
            note_count: 0,
            revision: 1,
            editing_note_index: -1,
            playback_note_index: -1,
            editing_note_content: QString::from(""),
            editing_note_start: 0.0,
            editing_note_end: 0.0,
            editing_note_has_range: false,
            has_video: false,
            status_message: QString::from("请先加载视频"),
            manager: NotesManager::new(els_storage::SqliteNoteRepository::default()),
            current_video_id: None,
            summaries: Vec::new(),
        }
    }
}

impl qobject::NoteBridge {
    fn load_for_video_path(mut self: Pin<&mut Self>, path: &QString, duration_secs: f64) -> bool {
        let path = path.to_string();
        if path.trim().is_empty() {
            self.as_mut().clear_video();
            return false;
        }
        let title = Path::new(&path)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(&path)
            .to_string();
        let video_id =
            match self
                .as_mut()
                .rust_mut()
                .manager
                .ensure_video(&path, &title, duration_secs)
            {
                Ok(video_id) => video_id,
                Err(err) => return self.as_mut().report_error("加载笔记失败", err),
            };
        let summaries = match self.rust().manager.list_summaries(video_id) {
            Ok(summaries) => summaries,
            Err(err) => return self.as_mut().report_error("读取笔记失败", err),
        };
        self.as_mut().rust_mut().current_video_id = Some(video_id);
        self.as_mut().rust_mut().summaries = summaries;
        self.as_mut().set_has_video(true);
        self.as_mut().refresh_summary_properties();
        self.as_mut().clear_editing_note();
        self.as_mut().set_playback_note_index(-1);
        self.as_mut()
            .set_status_message(QString::from("笔记已加载"));
        true
    }

    fn create_for_range(mut self: Pin<&mut Self>, start_secs: f64, end_secs: f64) -> bool {
        self.as_mut()
            .create_note(start_secs, Some(end_secs), "创建范围笔记失败")
    }

    fn create_at_position(mut self: Pin<&mut Self>, position_secs: f64) -> bool {
        self.as_mut()
            .create_note(position_secs, None, "创建时间点笔记失败")
    }

    fn select_note(mut self: Pin<&mut Self>, index: i32) -> bool {
        if index < 0 {
            self.as_mut().clear_editing_note();
            return false;
        }
        let video_id = match self.rust().current_video_id {
            Some(video_id) => video_id,
            None => return false,
        };
        let note_id = match self.rust().summaries.get(index as usize) {
            Some(summary) => summary.id,
            None => {
                self.as_mut().clear_editing_note();
                return false;
            }
        };
        let note = match self.rust().manager.load_note(note_id, video_id) {
            Ok(note) => note,
            Err(err) => return self.as_mut().report_error("加载笔记内容失败", err),
        };
        self.as_mut().set_editing_note_index(index);
        self.as_mut()
            .set_editing_note_content(QString::from(&note.content));
        self.as_mut().set_editing_note_start(note.start_time);
        self.as_mut()
            .set_editing_note_end(note.end_time.unwrap_or(note.start_time));
        self.as_mut()
            .set_editing_note_has_range(note.end_time.is_some());
        true
    }

    fn update_active_note(mut self: Pin<&mut Self>, content: &QString) -> bool {
        let video_id = match self.rust().current_video_id {
            Some(video_id) => video_id,
            None => return false,
        };
        let active_index = self.rust().editing_note_index;
        let note_id = match self.rust().summaries.get(active_index.max(0) as usize) {
            Some(summary) if active_index >= 0 => summary.id,
            _ => return false,
        };
        let content = content.to_string();
        if let Err(err) = self
            .as_mut()
            .rust_mut()
            .manager
            .update_content(note_id, video_id, &content)
        {
            return self.as_mut().report_error("保存笔记失败", err);
        }
        if !self.as_mut().reload_summaries() {
            return false;
        }
        let new_index = self
            .rust()
            .summaries
            .iter()
            .position(|summary| summary.id == note_id)
            .map(|index| index as i32)
            .unwrap_or(-1);
        self.as_mut().set_editing_note_index(new_index);
        self.as_mut()
            .set_editing_note_content(QString::from(&content));
        self.as_mut()
            .set_status_message(QString::from("笔记已保存"));
        true
    }

    fn delete_note(mut self: Pin<&mut Self>, index: i32) -> bool {
        if index < 0 {
            return false;
        }
        let video_id = match self.rust().current_video_id {
            Some(video_id) => video_id,
            None => return false,
        };
        let note_id = match self.rust().summaries.get(index as usize) {
            Some(summary) => summary.id,
            None => return false,
        };
        if let Err(err) = self
            .as_mut()
            .rust_mut()
            .manager
            .delete_note(note_id, video_id)
        {
            return self.as_mut().report_error("删除笔记失败", err);
        }
        if !self.as_mut().reload_summaries() {
            return false;
        }
        self.as_mut().clear_editing_note();
        self.as_mut()
            .set_status_message(QString::from("笔记已删除"));
        true
    }

    fn sync_playback_position(mut self: Pin<&mut Self>, position_secs: f64) -> bool {
        let index = els_note_core::active_note_index(&self.rust().summaries, position_secs)
            .map(|index| index as i32)
            .unwrap_or(-1);
        self.as_mut().set_playback_note_index(index);
        index >= 0
    }

    fn note_start_at(&self, index: i32) -> f64 {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().summaries.get(index))
            .map(|summary| summary.start_time)
            .unwrap_or(0.0)
    }

    fn note_end_at(&self, index: i32) -> f64 {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().summaries.get(index))
            .and_then(|summary| summary.end_time)
            .unwrap_or_else(|| self.note_start_at(index))
    }

    fn note_has_range_at(&self, index: i32) -> bool {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().summaries.get(index))
            .and_then(|summary| summary.end_time)
            .is_some()
    }

    fn note_preview_at(&self, index: i32) -> QString {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.rust().summaries.get(index))
            .map(|summary| QString::from(&summary.preview))
            .unwrap_or_else(|| QString::from(""))
    }
}

impl qobject::NoteBridge {
    fn create_note(
        mut self: Pin<&mut Self>,
        start_time: f64,
        end_time: Option<f64>,
        error_context: &str,
    ) -> bool {
        let video_id = match self.rust().current_video_id {
            Some(video_id) => video_id,
            None => return false,
        };
        let note_id = match self
            .as_mut()
            .rust_mut()
            .manager
            .create_note(els_note_core::NewNote {
                video_id,
                start_time,
                end_time,
                content: String::new(),
            }) {
            Ok(note_id) => note_id,
            Err(err) => return self.as_mut().report_error(error_context, err),
        };
        if !self.as_mut().reload_summaries() {
            return false;
        }
        let index = self
            .rust()
            .summaries
            .iter()
            .position(|summary| summary.id == note_id)
            .map(|index| index as i32)
            .unwrap_or(-1);
        self.as_mut().select_note(index)
    }

    fn reload_summaries(mut self: Pin<&mut Self>) -> bool {
        let video_id = match self.rust().current_video_id {
            Some(video_id) => video_id,
            None => return false,
        };
        let summaries = match self.rust().manager.list_summaries(video_id) {
            Ok(summaries) => summaries,
            Err(err) => return self.as_mut().report_error("刷新笔记失败", err),
        };
        self.as_mut().rust_mut().summaries = summaries;
        self.as_mut().refresh_summary_properties();
        true
    }

    fn refresh_summary_properties(mut self: Pin<&mut Self>) {
        let count = self.rust().summaries.len().min(i32::MAX as usize) as i32;
        self.as_mut().set_note_count(count);
        let revision = self.rust().revision.wrapping_add(1).max(1);
        self.as_mut().set_revision(revision);
    }

    fn clear_editing_note(mut self: Pin<&mut Self>) {
        self.as_mut().set_editing_note_index(-1);
        self.as_mut().set_editing_note_content(QString::from(""));
        self.as_mut().set_editing_note_start(0.0);
        self.as_mut().set_editing_note_end(0.0);
        self.as_mut().set_editing_note_has_range(false);
    }

    fn clear_video(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().current_video_id = None;
        self.as_mut().rust_mut().summaries.clear();
        self.as_mut().set_has_video(false);
        self.as_mut().refresh_summary_properties();
        self.as_mut().clear_editing_note();
        self.as_mut().set_playback_note_index(-1);
        self.as_mut()
            .set_status_message(QString::from("请先加载视频"));
    }

    fn report_error(mut self: Pin<&mut Self>, context: &str, error: els_types::AppError) -> bool {
        let message = format!("{context}：{error}");
        eprintln!("{message}");
        self.as_mut().set_status_message(QString::from(&message));
        false
    }
}
