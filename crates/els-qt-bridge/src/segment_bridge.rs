//! 学习片段列表与当前片段的 QML 桥接层。

use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use els_learning_core::LearningManager;
use std::path::Path;

type SegmentManager =
    els_learning_core::DefaultLearningManager<els_storage::SqliteSegmentRepository>;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(i32, segment_count, cxx_name = "segmentCount")]
        #[qproperty(i32, revision)]
        #[qproperty(i32, active_index, cxx_name = "activeIndex")]
        #[qproperty(f64, active_start, cxx_name = "activeStart")]
        #[qproperty(f64, active_end, cxx_name = "activeEnd")]
        #[qproperty(i32, active_repeat_count, cxx_name = "activeRepeatCount")]
        #[qproperty(i32, active_interval_seconds, cxx_name = "activeIntervalSeconds")]
        #[qproperty(i32, active_completed_loops, cxx_name = "activeCompletedLoops")]
        #[qproperty(QString, current_video_path, cxx_name = "currentVideoPath")]
        #[qproperty(QString, current_video_title, cxx_name = "currentVideoTitle")]
        #[qproperty(QString, status_message, cxx_name = "statusMessage")]
        type SegmentBridge = super::SegmentBridgeRust;

        #[qinvokable]
        #[cxx_name = "loadForVideoPath"]
        fn load_for_video_path(
            self: Pin<&mut SegmentBridge>,
            path: &QString,
            duration_secs: f64,
        ) -> bool;

        #[qinvokable]
        #[cxx_name = "saveCurrentSelection"]
        fn save_current_selection(
            self: Pin<&mut SegmentBridge>,
            start_secs: f64,
            end_secs: f64,
            repeat_count: i32,
            interval_seconds: i32,
        ) -> bool;

        #[qinvokable]
        #[cxx_name = "activateSegment"]
        fn activate_segment(self: Pin<&mut SegmentBridge>, index: i32) -> bool;

        #[qinvokable]
        #[cxx_name = "deactivateSegment"]
        fn deactivate_segment(self: Pin<&mut SegmentBridge>) -> bool;

        #[qinvokable]
        #[cxx_name = "deleteSegment"]
        fn delete_segment(self: Pin<&mut SegmentBridge>, index: i32) -> bool;

        #[qinvokable]
        #[cxx_name = "incrementCompletedLoops"]
        fn increment_completed_loops(self: Pin<&mut SegmentBridge>) -> bool;

        #[qinvokable]
        #[cxx_name = "segmentStartAt"]
        fn segment_start_at(self: Pin<&mut SegmentBridge>, index: i32) -> f64;

        #[qinvokable]
        #[cxx_name = "segmentEndAt"]
        fn segment_end_at(self: Pin<&mut SegmentBridge>, index: i32) -> f64;

        #[qinvokable]
        #[cxx_name = "segmentRepeatCountAt"]
        fn segment_repeat_count_at(self: Pin<&mut SegmentBridge>, index: i32) -> i32;

        #[qinvokable]
        #[cxx_name = "segmentIntervalSecondsAt"]
        fn segment_interval_seconds_at(self: Pin<&mut SegmentBridge>, index: i32) -> i32;

        #[qinvokable]
        #[cxx_name = "segmentCompletedLoopsAt"]
        fn segment_completed_loops_at(self: Pin<&mut SegmentBridge>, index: i32) -> i32;
    }
}

pub struct SegmentBridgeRust {
    segment_count: i32,
    revision: i32,
    active_index: i32,
    active_start: f64,
    active_end: f64,
    active_repeat_count: i32,
    active_interval_seconds: i32,
    active_completed_loops: i32,
    current_video_path: QString,
    current_video_title: QString,
    status_message: QString,
    manager: SegmentManager,
    current_video_id: Option<i64>,
    segments: Vec<els_learning_core::Segment>,
}

impl Default for SegmentBridgeRust {
    fn default() -> Self {
        Self {
            segment_count: 0,
            revision: 1,
            active_index: -1,
            active_start: 0.0,
            active_end: 0.0,
            active_repeat_count: 0,
            active_interval_seconds: 0,
            active_completed_loops: 0,
            current_video_path: QString::from(""),
            current_video_title: QString::from(""),
            status_message: QString::from("尚未加载视频片段"),
            manager: SegmentManager::new(els_storage::SqliteSegmentRepository::default()),
            current_video_id: None,
            segments: Vec::new(),
        }
    }
}

impl qobject::SegmentBridge {
    fn load_for_video_path(mut self: Pin<&mut Self>, path: &QString, duration_secs: f64) -> bool {
        let path = path.to_string();
        if path.trim().is_empty() {
            self.as_mut().rust_mut().current_video_id = None;
            self.as_mut().rust_mut().segments.clear();
            self.as_mut().set_segment_count(0);
            self.as_mut().clear_active_segment();
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
                Err(err) => return self.as_mut().report_error("加载片段失败", err),
            };
        let segments = match self.rust().manager.list_segments(video_id) {
            Ok(segments) => segments,
            Err(err) => return self.as_mut().report_error("读取片段失败", err),
        };

        self.as_mut().rust_mut().current_video_id = Some(video_id);
        self.as_mut().rust_mut().segments = segments;
        self.as_mut().set_current_video_path(QString::from(&path));
        self.as_mut().set_current_video_title(QString::from(&title));
        self.as_mut().refresh_list_properties();
        self.as_mut().clear_active_segment();
        self.as_mut()
            .set_status_message(QString::from("学习片段已加载"));
        true
    }

    fn save_current_selection(
        mut self: Pin<&mut Self>,
        start_secs: f64,
        end_secs: f64,
        repeat_count: i32,
        interval_seconds: i32,
    ) -> bool {
        let video_id = match self.rust().current_video_id {
            Some(video_id) => video_id,
            None => return false,
        };
        let active_segment = (self.rust().active_index >= 0)
            .then(|| self.rust().segments.get(self.rust().active_index as usize))
            .flatten()
            .cloned();
        let matching_segment = active_segment.or_else(|| {
            self.rust()
                .segments
                .iter()
                .find(|segment| {
                    (segment.range.start - start_secs).abs() < 0.05
                        && (segment.range.end - end_secs).abs() < 0.05
                })
                .cloned()
        });
        let segment = els_learning_core::Segment {
            id: matching_segment.as_ref().and_then(|segment| segment.id),
            video_id,
            range: els_types::TimeRange {
                start: start_secs,
                end: end_secs,
            },
            repeat_count: repeat_count.max(1) as u32,
            interval_seconds: interval_seconds.max(0) as u32,
            completed_loops: matching_segment
                .as_ref()
                .map(|segment| segment.completed_loops)
                .unwrap_or(0),
        };
        let saved_id = match self.as_mut().rust_mut().manager.add_segment(segment) {
            Ok(id) => id,
            Err(err) => return self.as_mut().report_error("保存片段失败", err),
        };
        if !self.as_mut().reload_segments() {
            return false;
        }
        let index = self
            .rust()
            .segments
            .iter()
            .position(|segment| segment.id == Some(saved_id))
            .unwrap_or(0) as i32;
        self.as_mut().activate_segment(index)
    }

    fn activate_segment(mut self: Pin<&mut Self>, index: i32) -> bool {
        let segment = match self.rust().segments.get(index.max(0) as usize).cloned() {
            Some(segment) => segment,
            None => return false,
        };
        self.as_mut().set_active_index(index);
        self.as_mut().set_active_start(segment.range.start);
        self.as_mut().set_active_end(segment.range.end);
        self.as_mut()
            .set_active_repeat_count(segment.repeat_count as i32);
        self.as_mut()
            .set_active_interval_seconds(segment.interval_seconds as i32);
        self.as_mut()
            .set_active_completed_loops(segment.completed_loops as i32);
        self.as_mut()
            .set_status_message(QString::from("当前片段已切换"));
        true
    }

    fn deactivate_segment(mut self: Pin<&mut Self>) -> bool {
        self.as_mut().clear_active_segment();
        self.as_mut()
            .set_status_message(QString::from("已退出片段编辑"));
        true
    }

    fn delete_segment(mut self: Pin<&mut Self>, index: i32) -> bool {
        let active_id = self
            .rust()
            .segments
            .get(self.rust().active_index.max(0) as usize)
            .and_then(|segment| segment.id);
        let id = match self
            .rust()
            .segments
            .get(index.max(0) as usize)
            .and_then(|segment| segment.id)
        {
            Some(id) => id,
            None => return false,
        };
        if let Err(err) = self.as_mut().rust_mut().manager.delete_segment(id) {
            return self.as_mut().report_error("删除片段失败", err);
        }
        if !self.as_mut().reload_segments() {
            return false;
        }
        if active_id == Some(id) {
            self.as_mut().clear_active_segment();
        } else if let Some(active_id) = active_id {
            if let Some(active_index) = self
                .rust()
                .segments
                .iter()
                .position(|segment| segment.id == Some(active_id))
            {
                self.as_mut().activate_segment(active_index as i32);
            }
        }
        self.as_mut()
            .set_status_message(QString::from("片段已删除"));
        true
    }

    fn increment_completed_loops(mut self: Pin<&mut Self>) -> bool {
        let index = self.rust().active_index;
        let id = match self
            .rust()
            .segments
            .get(index.max(0) as usize)
            .and_then(|segment| segment.id)
        {
            Some(id) => id,
            None => return false,
        };
        let progress = match self.as_mut().rust_mut().manager.record_completed_loop(id) {
            Ok(progress) => progress,
            Err(err) => return self.as_mut().report_error("保存训练进度失败", err),
        };
        if let Some(segment) = self
            .as_mut()
            .rust_mut()
            .segments
            .get_mut(index.max(0) as usize)
        {
            segment.completed_loops = progress.completed_loops;
        }
        self.as_mut()
            .set_active_completed_loops(progress.completed_loops as i32);
        self.as_mut().bump_revision();
        true
    }

    fn segment_start_at(self: Pin<&mut Self>, index: i32) -> f64 {
        self.rust()
            .segments
            .get(index.max(0) as usize)
            .map(|segment| segment.range.start)
            .unwrap_or(0.0)
    }

    fn segment_end_at(self: Pin<&mut Self>, index: i32) -> f64 {
        self.rust()
            .segments
            .get(index.max(0) as usize)
            .map(|segment| segment.range.end)
            .unwrap_or(0.0)
    }

    fn segment_repeat_count_at(self: Pin<&mut Self>, index: i32) -> i32 {
        self.rust()
            .segments
            .get(index.max(0) as usize)
            .map(|segment| segment.repeat_count as i32)
            .unwrap_or(0)
    }

    fn segment_interval_seconds_at(self: Pin<&mut Self>, index: i32) -> i32 {
        self.rust()
            .segments
            .get(index.max(0) as usize)
            .map(|segment| segment.interval_seconds as i32)
            .unwrap_or(0)
    }

    fn segment_completed_loops_at(self: Pin<&mut Self>, index: i32) -> i32 {
        self.rust()
            .segments
            .get(index.max(0) as usize)
            .map(|segment| segment.completed_loops as i32)
            .unwrap_or(0)
    }

    fn reload_segments(mut self: Pin<&mut Self>) -> bool {
        let video_id = match self.rust().current_video_id {
            Some(video_id) => video_id,
            None => return false,
        };
        let segments = match self.rust().manager.list_segments(video_id) {
            Ok(segments) => segments,
            Err(err) => return self.as_mut().report_error("刷新片段失败", err),
        };
        self.as_mut().rust_mut().segments = segments;
        self.as_mut().refresh_list_properties();
        true
    }

    fn refresh_list_properties(mut self: Pin<&mut Self>) {
        let segment_count = self.rust().segments.len() as i32;
        self.as_mut().set_segment_count(segment_count);
        self.as_mut().bump_revision();
    }

    fn clear_active_segment(mut self: Pin<&mut Self>) {
        self.as_mut().set_active_index(-1);
        self.as_mut().set_active_start(0.0);
        self.as_mut().set_active_end(0.0);
        self.as_mut().set_active_repeat_count(0);
        self.as_mut().set_active_interval_seconds(0);
        self.as_mut().set_active_completed_loops(0);
    }

    fn bump_revision(mut self: Pin<&mut Self>) {
        let next_revision = self.rust().revision.saturating_add(1);
        self.as_mut().set_revision(next_revision);
    }

    fn report_error(mut self: Pin<&mut Self>, context: &str, error: els_types::AppError) -> bool {
        let message = format!("{context}: {error}");
        eprintln!("{message}");
        self.as_mut().set_status_message(QString::from(&message));
        false
    }
}
