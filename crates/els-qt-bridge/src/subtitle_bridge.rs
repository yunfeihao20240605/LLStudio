//! 视频级字幕轨道与 QML 的桥接层。

use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use els_subtitle_core::SubtitleProvider;
use std::path::{Path, PathBuf};

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, entries_json, cxx_name = "entriesJson")]
        #[qproperty(i32, active_cue_index, cxx_name = "activeCueIndex")]
        #[qproperty(f64, active_cue_start, cxx_name = "activeCueStart")]
        #[qproperty(f64, active_cue_end, cxx_name = "activeCueEnd")]
        #[qproperty(QString, active_original_text, cxx_name = "activeOriginalText")]
        #[qproperty(QString, active_translated_text, cxx_name = "activeTranslatedText")]
        #[qproperty(i32, editing_cue_index, cxx_name = "editingCueIndex")]
        #[qproperty(QString, editing_text, cxx_name = "editingText")]
        #[qproperty(bool, has_video, cxx_name = "hasVideo")]
        #[qproperty(QString, status_message, cxx_name = "statusMessage")]
        type SubtitleBridge = super::SubtitleBridgeRust;

        #[qinvokable]
        #[cxx_name = "loadForVideoPath"]
        fn load_for_video_path(self: Pin<&mut SubtitleBridge>, video_path: &QString) -> bool;

        #[qinvokable]
        #[cxx_name = "syncPlaybackPosition"]
        fn sync_playback_position(self: Pin<&mut SubtitleBridge>, position_secs: f64) -> bool;

        #[qinvokable]
        #[cxx_name = "syncSelectionRange"]
        fn sync_selection_range(
            self: Pin<&mut SubtitleBridge>,
            start_secs: f64,
            end_secs: f64,
        ) -> bool;

        #[qinvokable]
        #[cxx_name = "saveCueForRange"]
        fn save_cue_for_range(
            self: Pin<&mut SubtitleBridge>,
            start_secs: f64,
            end_secs: f64,
            text: &QString,
        ) -> bool;

        #[qinvokable]
        #[cxx_name = "deleteCue"]
        fn delete_cue(self: Pin<&mut SubtitleBridge>, cue_index: i32) -> bool;

        #[qinvokable]
        #[cxx_name = "deleteCuesForRange"]
        fn delete_cues_for_range(
            self: Pin<&mut SubtitleBridge>,
            start_secs: f64,
            end_secs: f64,
        ) -> bool;

        #[qinvokable]
        #[cxx_name = "selectCue"]
        fn select_cue(self: Pin<&mut SubtitleBridge>, cue_index: i32) -> bool;

        #[qinvokable]
        #[cxx_name = "updateCueRange"]
        fn update_cue_range(
            self: Pin<&mut SubtitleBridge>,
            cue_index: i32,
            start_secs: f64,
            end_secs: f64,
        ) -> bool;
    }
}

pub struct SubtitleBridgeRust {
    entries_json: QString,
    active_cue_index: i32,
    active_cue_start: f64,
    active_cue_end: f64,
    active_original_text: QString,
    active_translated_text: QString,
    editing_cue_index: i32,
    editing_text: QString,
    has_video: bool,
    status_message: QString,
    track: els_subtitle_core::SubtitleTrack,
    subtitle_path: Option<PathBuf>,
    playback_position: f64,
}

impl Default for SubtitleBridgeRust {
    fn default() -> Self {
        Self {
            entries_json: QString::from("[]"),
            active_cue_index: -1,
            active_cue_start: 0.0,
            active_cue_end: 0.0,
            active_original_text: QString::from(""),
            active_translated_text: QString::from(""),
            editing_cue_index: -1,
            editing_text: QString::from(""),
            has_video: false,
            status_message: QString::from("请先加载视频"),
            track: els_subtitle_core::SubtitleTrack::default(),
            subtitle_path: None,
            playback_position: 0.0,
        }
    }
}

impl qobject::SubtitleBridge {
    fn load_for_video_path(mut self: Pin<&mut Self>, video_path: &QString) -> bool {
        let requested_path = video_path.to_string();
        let requested_path = requested_path.trim();
        if requested_path.is_empty() {
            self.as_mut().rust_mut().track.replace_cues(Vec::new());
            self.as_mut().rust_mut().subtitle_path = None;
            self.as_mut().set_has_video(false);
            self.as_mut().sync_track_state("请先加载视频");
            return false;
        }

        let save_path = srt_path_for_video(requested_path);
        let source_path = sibling_subtitle_path(requested_path);
        let mut track = els_subtitle_core::SubtitleTrack::default();
        if let Some(path) = source_path.as_ref() {
            if let Err(err) = track.load(&path.to_string_lossy()) {
                return self.as_mut().report_error("加载字幕失败", err);
            }
        }

        self.as_mut().rust_mut().track = track;
        self.as_mut().rust_mut().subtitle_path = save_path.clone();
        self.as_mut().rust_mut().playback_position = 0.0;
        self.as_mut().set_has_video(save_path.is_some());

        let status = if source_path.is_some() {
            String::new()
        } else {
            match save_path {
                Some(target) => {
                    format!("当前视频暂无字幕，将保存到 {}", target.to_string_lossy())
                }
                None => "无法确定字幕保存路径".to_string(),
            }
        };
        self.as_mut().sync_track_state(&status);
        true
    }

    fn sync_playback_position(mut self: Pin<&mut Self>, position_secs: f64) -> bool {
        let position = position_secs.max(0.0);
        self.as_mut().rust_mut().playback_position = position;
        let cue_index = self
            .rust()
            .track
            .cue_index_at(position)
            .map(|index| index as i32)
            .unwrap_or(-1);
        self.as_mut().set_active_cue(cue_index);
        cue_index >= 0
    }

    fn sync_selection_range(mut self: Pin<&mut Self>, start_secs: f64, end_secs: f64) -> bool {
        let range = els_types::TimeRange {
            start: start_secs,
            end: end_secs,
        };
        let cue_index = self
            .rust()
            .track
            .cue_index_for_range(range)
            .map(|index| index as i32)
            .unwrap_or(-1);
        self.as_mut().set_editing_cue(cue_index);
        cue_index >= 0
    }

    fn save_cue_for_range(
        mut self: Pin<&mut Self>,
        start_secs: f64,
        end_secs: f64,
        text: &QString,
    ) -> bool {
        let Some(subtitle_path) = self.rust().subtitle_path.clone() else {
            self.as_mut()
                .set_status_message(QString::from("请先加载视频"));
            return false;
        };
        let previous_track = self.rust().track.clone();
        let cue_index = match self.as_mut().rust_mut().track.add_or_update_for_range(
            els_types::TimeRange {
                start: start_secs,
                end: end_secs,
            },
            &text.to_string(),
        ) {
            Ok(index) => index,
            Err(err) => return self.as_mut().report_error("保存字幕失败", err),
        };

        if let Err(err) = self.rust().track.save_srt(&subtitle_path.to_string_lossy()) {
            self.as_mut().rust_mut().track = previous_track;
            return self.as_mut().report_error("保存字幕失败", err);
        }

        self.as_mut().refresh_entries();
        self.as_mut().set_editing_cue(cue_index as i32);
        let playback_position = self.rust().playback_position;
        self.as_mut().sync_playback_position(playback_position);
        self.as_mut().set_status_message(QString::from(format!(
            "字幕已保存：{}",
            subtitle_path.to_string_lossy()
        )));
        true
    }

    fn delete_cue(mut self: Pin<&mut Self>, cue_index: i32) -> bool {
        let Some(subtitle_path) = self.rust().subtitle_path.clone() else {
            return false;
        };
        if cue_index < 0 {
            return false;
        }

        let previous_track = self.rust().track.clone();
        if let Err(err) = self
            .as_mut()
            .rust_mut()
            .track
            .remove_cue(cue_index as usize)
        {
            return self.as_mut().report_error("删除字幕失败", err);
        }
        if let Err(err) = self.rust().track.save_srt(&subtitle_path.to_string_lossy()) {
            self.as_mut().rust_mut().track = previous_track;
            return self.as_mut().report_error("删除字幕失败", err);
        }

        self.as_mut().refresh_entries();
        self.as_mut().set_editing_cue(-1);
        let playback_position = self.rust().playback_position;
        self.as_mut().sync_playback_position(playback_position);
        self.as_mut()
            .set_status_message(QString::from("字幕已删除"));
        true
    }

    fn delete_cues_for_range(mut self: Pin<&mut Self>, start_secs: f64, end_secs: f64) -> bool {
        let Some(subtitle_path) = self.rust().subtitle_path.clone() else {
            return false;
        };
        let previous_track = self.rust().track.clone();
        let removed_count =
            match self
                .as_mut()
                .rust_mut()
                .track
                .remove_cues_for_range(els_types::TimeRange {
                    start: start_secs,
                    end: end_secs,
                }) {
                Ok(count) => count,
                Err(err) => return self.as_mut().report_error("删除片段字幕失败", err),
            };

        if removed_count == 0 {
            self.as_mut()
                .set_status_message(QString::from("片段范围内没有字幕"));
            return true;
        }

        if let Err(err) = self.rust().track.save_srt(&subtitle_path.to_string_lossy()) {
            self.as_mut().rust_mut().track = previous_track;
            return self.as_mut().report_error("删除片段字幕失败", err);
        }

        self.as_mut().refresh_entries();
        self.as_mut().set_editing_cue(-1);
        let playback_position = self.rust().playback_position;
        self.as_mut().sync_playback_position(playback_position);
        self.as_mut().set_status_message(QString::from(format!(
            "已删除片段对应的 {} 条字幕",
            removed_count
        )));
        true
    }

    fn select_cue(mut self: Pin<&mut Self>, cue_index: i32) -> bool {
        self.as_mut().set_editing_cue(cue_index);
        cue_index >= 0 && (cue_index as usize) < self.rust().track.cues().len()
    }

    fn update_cue_range(
        mut self: Pin<&mut Self>,
        cue_index: i32,
        start_secs: f64,
        end_secs: f64,
    ) -> bool {
        let Some(subtitle_path) = self.rust().subtitle_path.clone() else {
            return false;
        };
        if cue_index < 0 {
            return false;
        }
        let previous_track = self.rust().track.clone();
        let updated_index = match self.as_mut().rust_mut().track.update_cue_range(
            cue_index as usize,
            els_types::TimeRange { start: start_secs, end: end_secs },
        ) {
            Ok(index) => index,
            Err(error) => return self.as_mut().report_error("更新字幕时间失败", error),
        };
        if let Err(error) = self.rust().track.save_srt(&subtitle_path.to_string_lossy()) {
            self.as_mut().rust_mut().track = previous_track;
            return self.as_mut().report_error("保存字幕时间失败", error);
        }
        self.as_mut().refresh_entries();
        self.as_mut().set_editing_cue(updated_index as i32);
        let playback_position = self.rust().playback_position;
        self.as_mut().sync_playback_position(playback_position);
        self.as_mut().set_status_message(QString::from("字幕时间已同步"));
        true
    }
}

impl qobject::SubtitleBridge {
    fn sync_track_state(mut self: Pin<&mut Self>, status_message: &str) {
        self.as_mut().refresh_entries();
        self.as_mut().set_active_cue(-1);
        self.as_mut().set_editing_cue(-1);
        self.as_mut()
            .set_status_message(QString::from(status_message));
    }

    fn refresh_entries(mut self: Pin<&mut Self>) {
        let entries = entries_json(self.rust().track.cues());
        self.as_mut().set_entries_json(QString::from(&entries));
    }

    fn set_active_cue(mut self: Pin<&mut Self>, cue_index: i32) {
        let Some(cue) = (cue_index >= 0)
            .then(|| self.rust().track.cues().get(cue_index as usize))
            .flatten()
        else {
            self.as_mut().set_active_cue_index(-1);
            self.as_mut().set_active_cue_start(0.0);
            self.as_mut().set_active_cue_end(0.0);
            self.as_mut().set_active_original_text(QString::from(""));
            self.as_mut().set_active_translated_text(QString::from(""));
            return;
        };

        let cue_start = cue.range.start;
        let cue_end = cue.range.end;
        let original_text = cue.original_text.clone();
        let translated_text = cue.translated_text.clone().unwrap_or_default();
        self.as_mut().set_active_cue_index(cue_index);
        self.as_mut().set_active_cue_start(cue_start);
        self.as_mut().set_active_cue_end(cue_end);
        self.as_mut()
            .set_active_original_text(QString::from(&original_text));
        self.as_mut()
            .set_active_translated_text(QString::from(&translated_text));
    }

    fn set_editing_cue(mut self: Pin<&mut Self>, cue_index: i32) {
        let text = (cue_index >= 0)
            .then(|| self.rust().track.cues().get(cue_index as usize))
            .flatten()
            .map(combined_text)
            .unwrap_or_default();
        let valid_index = if text.is_empty() { -1 } else { cue_index };
        self.as_mut().set_editing_cue_index(valid_index);
        self.as_mut().set_editing_text(QString::from(&text));
    }

    fn report_error(mut self: Pin<&mut Self>, context: &str, error: els_types::AppError) -> bool {
        let message = format!("{context}: {error}");
        eprintln!("{message}");
        self.as_mut().set_status_message(QString::from(&message));
        false
    }
}

fn srt_path_for_video(video_path: &str) -> Option<PathBuf> {
    let video = Path::new(video_path);
    let parent = video.parent()?;
    let stem = video.file_stem()?.to_string_lossy();
    Some(parent.join(format!("{stem}.srt")))
}

pub(crate) fn sibling_subtitle_path(video_path: &str) -> Option<PathBuf> {
    let video = Path::new(video_path);
    let parent = video.parent()?;
    let stem = video.file_stem()?.to_string_lossy();
    ["srt", "vtt"]
        .into_iter()
        .map(|extension| parent.join(format!("{stem}.{extension}")))
        .find(|candidate| candidate.exists())
}

fn combined_text(cue: &els_subtitle_core::SubtitleCue) -> String {
    match cue
        .translated_text
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        Some(translated) => format!("{}\n{}", cue.original_text, translated),
        None => cue.original_text.clone(),
    }
}

fn entries_json(cues: &[els_subtitle_core::SubtitleCue]) -> String {
    let items = cues
        .iter()
        .enumerate()
        .map(|(index, cue)| {
            format!(
                "{{\"index\":{},\"start\":{},\"end\":{},\"startTime\":\"{}\",\"endTime\":\"{}\",\"text\":\"{}\"}}",
                index,
                cue.range.start,
                cue.range.end,
                format_display_timestamp(cue.range.start),
                format_display_timestamp(cue.range.end),
                escape_json(&combined_text(cue)),
            )
        })
        .collect::<Vec<_>>();
    format!("[{}]", items.join(","))
}

fn format_display_timestamp(seconds: f64) -> String {
    let total_millis = (seconds.max(0.0) * 1_000.0).round() as u64;
    let hours = total_millis / 3_600_000;
    let minutes = (total_millis % 3_600_000) / 60_000;
    let secs = (total_millis % 60_000) / 1_000;
    let millis = total_millis % 1_000;
    format!("{hours:02}:{minutes:02}:{secs:02},{millis:03}")
}

fn escape_json(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
