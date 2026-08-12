//! 跟读录音与 QML 的薄适配层。

use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QVector};
use els_recording_core::{RecordingManager, RecordingState};
use els_waveform_core::WaveformEngine;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        include!("cxx-qt-lib/core/qvector/qvector_f32.h");
        type QString = cxx_qt_lib::QString;
        type QVector_f32 = cxx_qt_lib::QVector<f32>;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(bool, has_video, cxx_name = "hasVideo")]
        #[qproperty(bool, has_target, cxx_name = "hasTarget")]
        #[qproperty(bool, is_recording, cxx_name = "isRecording")]
        #[qproperty(bool, is_processing, cxx_name = "isProcessing")]
        #[qproperty(bool, has_recording, cxx_name = "hasRecording")]
        #[qproperty(f64, target_start, cxx_name = "targetStart")]
        #[qproperty(f64, target_end, cxx_name = "targetEnd")]
        #[qproperty(f64, recording_duration, cxx_name = "recordingDuration")]
        #[qproperty(f64, recording_elapsed, cxx_name = "recordingElapsed")]
        #[qproperty(f64, alignment_offset, cxx_name = "alignmentOffset")]
        #[qproperty(QString, recording_file_path, cxx_name = "recordingFilePath")]
        #[qproperty(QVector_f32, recording_peak_values, cxx_name = "recordingPeakValues")]
        #[qproperty(i32, recording_revision, cxx_name = "recordingRevision")]
        #[qproperty(QString, status_message, cxx_name = "statusMessage")]
        type RecordingBridge = super::RecordingBridgeRust;

        #[qinvokable]
        #[cxx_name = "loadForVideoPath"]
        fn load_for_video_path(
            self: Pin<&mut RecordingBridge>,
            path: &QString,
            duration_secs: f64,
        ) -> bool;

        #[qinvokable]
        #[cxx_name = "syncTargetRange"]
        fn sync_target_range(
            self: Pin<&mut RecordingBridge>,
            start_secs: f64,
            end_secs: f64,
            has_range: bool,
        ) -> bool;

        #[qinvokable]
        #[cxx_name = "startRecording"]
        fn start_recording(self: Pin<&mut RecordingBridge>) -> bool;

        #[qinvokable]
        #[cxx_name = "stopRecording"]
        fn stop_recording(self: Pin<&mut RecordingBridge>) -> bool;

        #[qinvokable]
        #[cxx_name = "pollBackgroundTask"]
        fn poll_background_task(self: Pin<&mut RecordingBridge>) -> bool;

        #[qinvokable]
        #[cxx_name = "saveAlignmentOffset"]
        fn save_alignment_offset(self: Pin<&mut RecordingBridge>, offset_secs: f64) -> bool;

        #[qinvokable]
        #[cxx_name = "resetAlignment"]
        fn reset_alignment(self: Pin<&mut RecordingBridge>) -> bool;

        #[qinvokable]
        #[cxx_name = "deleteRecording"]
        fn delete_recording(self: Pin<&mut RecordingBridge>) -> bool;
    }
}

type RecordingsManager =
    els_recording_core::DefaultRecordingManager<els_storage::SqliteRecordingRepository>;

pub struct RecordingBridgeRust {
    has_video: bool,
    has_target: bool,
    is_recording: bool,
    is_processing: bool,
    has_recording: bool,
    target_start: f64,
    target_end: f64,
    recording_duration: f64,
    recording_elapsed: f64,
    alignment_offset: f64,
    recording_file_path: QString,
    recording_peak_values: QVector<f32>,
    recording_revision: i32,
    status_message: QString,
    manager: RecordingsManager,
    session: els_recording_core::RecordingSession,
    current_video_id: Option<i64>,
    active_capture: Option<els_audio_capture::ActiveCapture>,
    task_receiver: Option<Receiver<WaveformTaskEvent>>,
    active_task_id: u64,
}

impl Default for RecordingBridgeRust {
    fn default() -> Self {
        Self {
            has_video: false,
            has_target: false,
            is_recording: false,
            is_processing: false,
            has_recording: false,
            target_start: 0.0,
            target_end: 0.0,
            recording_duration: 0.0,
            recording_elapsed: 0.0,
            alignment_offset: 0.0,
            recording_file_path: QString::from(""),
            recording_peak_values: QVector::from(Vec::new()),
            recording_revision: 1,
            status_message: QString::from("请先选择录音范围"),
            manager: RecordingsManager::new(els_storage::SqliteRecordingRepository::default()),
            session: els_recording_core::RecordingSession::default(),
            current_video_id: None,
            active_capture: None,
            task_receiver: None,
            active_task_id: 0,
        }
    }
}

impl qobject::RecordingBridge {
    fn load_for_video_path(mut self: Pin<&mut Self>, path: &QString, duration_secs: f64) -> bool {
        if self.rust().active_capture.is_some() {
            return false;
        }
        let path = path.to_string();
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
                Err(error) => return self.as_mut().report_error("加载录音失败", error),
            };
        self.as_mut().rust_mut().current_video_id = Some(video_id);
        self.as_mut().set_has_video(true);
        self.as_mut().clear_target();
        self.as_mut()
            .set_status_message(QString::from("请选择 A～B 录音范围"));
        true
    }

    fn sync_target_range(
        mut self: Pin<&mut Self>,
        start_secs: f64,
        end_secs: f64,
        has_range: bool,
    ) -> bool {
        if self.rust().active_capture.is_some()
            || self.rust().session.state() == &RecordingState::Processing
        {
            return false;
        }
        self.as_mut().rust_mut().task_receiver = None;
        self.as_mut().set_is_processing(false);
        let video_id = match self.rust().current_video_id {
            Some(video_id) => video_id,
            None => return false,
        };
        if !has_range {
            self.as_mut().clear_target();
            self.as_mut()
                .set_status_message(QString::from("请选择 A～B 录音范围"));
            return true;
        }
        let range = els_types::TimeRange {
            start: start_secs,
            end: end_secs,
        };
        if let Err(error) = self.as_mut().rust_mut().session.set_target(Some(range)) {
            return self.as_mut().report_error("设置录音范围失败", error);
        }
        self.as_mut().set_target_start(start_secs);
        self.as_mut().set_target_end(end_secs);
        self.as_mut().set_has_target(true);
        let recording = match self.rust().manager.latest_for_range(video_id, range) {
            Ok(recording) => recording,
            Err(error) => return self.as_mut().report_error("读取录音失败", error),
        };
        self.as_mut()
            .rust_mut()
            .session
            .load_recording(recording.clone());
        if let Some(recording) = recording {
            self.as_mut().apply_recording_metadata(&recording);
            self.as_mut().start_waveform_task(recording, None);
        } else {
            self.as_mut().clear_recording();
            self.as_mut()
                .set_status_message(QString::from("可以开始录音"));
        }
        true
    }

    fn start_recording(mut self: Pin<&mut Self>) -> bool {
        if !self.rust().has_video || !self.rust().has_target {
            return false;
        }
        if let Err(error) = self.as_mut().rust_mut().session.start() {
            return self.as_mut().report_error("开始录音失败", error);
        }
        let video_id = self.rust().current_video_id.unwrap_or_default();
        let path = match els_audio_capture::next_recording_path(video_id) {
            Ok(path) => path,
            Err(error) => return self.as_mut().fail_session("创建录音文件失败", error),
        };
        let capture = match els_audio_capture::MicrophoneRecorder::start(&path) {
            Ok(capture) => capture,
            Err(error) => {
                let _ = std::fs::remove_file(path);
                return self.as_mut().fail_session("打开麦克风失败", error);
            }
        };
        self.as_mut().rust_mut().active_capture = Some(capture);
        self.as_mut().set_is_recording(true);
        self.as_mut().set_recording_elapsed(0.0);
        self.as_mut().clear_recording();
        self.as_mut().set_status_message(QString::from("正在录音"));
        true
    }

    fn stop_recording(mut self: Pin<&mut Self>) -> bool {
        let capture = match self.as_mut().rust_mut().active_capture.take() {
            Some(capture) => capture,
            None => return false,
        };
        self.as_mut().set_is_recording(false);
        if let Err(error) = self.as_mut().rust_mut().session.begin_processing() {
            return self.as_mut().report_error("停止录音失败", error);
        }
        let captured = match capture.stop() {
            Ok(captured) => captured,
            Err(error) => return self.as_mut().fail_session("保存录音失败", error),
        };
        self.as_mut().set_is_processing(true);
        self.as_mut()
            .set_status_message(QString::from("正在生成录音波形"));
        self.as_mut().start_captured_waveform_task(captured);
        true
    }

    fn poll_background_task(mut self: Pin<&mut Self>) -> bool {
        if let Some(capture) = self.rust().active_capture.as_ref() {
            let elapsed = capture.elapsed().as_secs_f64();
            self.as_mut().set_recording_elapsed(elapsed);
            if elapsed >= 600.0 {
                return self.as_mut().stop_recording();
            }
        }

        let event = match self.rust().task_receiver.as_ref() {
            Some(receiver) => match receiver.try_recv() {
                Ok(event) => Some(event),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => Some(WaveformTaskEvent::Failed {
                    task_id: self.rust().active_task_id,
                    message: "录音波形任务意外结束".to_string(),
                    captured_path: None,
                }),
            },
            None => None,
        };
        let Some(event) = event else {
            return self.rust().is_recording || self.rust().is_processing;
        };
        match event {
            WaveformTaskEvent::Ready {
                task_id,
                recording,
                captured,
                waveform,
            } if task_id == self.rust().active_task_id => {
                self.as_mut().rust_mut().task_receiver = None;
                let recording =
                    if let Some(captured) = captured {
                        let video_id = self.rust().current_video_id.unwrap_or_default();
                        let range = match self.rust().session.target() {
                            Some(range) => range,
                            None => return false,
                        };
                        match self.as_mut().rust_mut().manager.save(
                            els_recording_core::NewRecording {
                                video_id,
                                range,
                                file_path: captured.path.to_string_lossy().into_owned(),
                                duration_secs: captured.duration_secs,
                                sample_rate: captured.sample_rate,
                                alignment_offset: 0.0,
                            },
                        ) {
                            Ok(recording) => recording,
                            Err(error) => {
                                let _ = std::fs::remove_file(captured.path);
                                return self.as_mut().fail_session("保存录音信息失败", error);
                            }
                        }
                    } else {
                        match recording {
                            Some(recording) => recording,
                            None => return false,
                        }
                    };
                if self.rust().session.state() == &RecordingState::Processing {
                    if let Err(error) = self.as_mut().rust_mut().session.complete(recording.clone())
                    {
                        return self.as_mut().report_error("完成录音失败", error);
                    }
                }
                self.as_mut().apply_recording_metadata(&recording);
                self.as_mut()
                    .set_recording_peak_values(flatten_bins(&waveform.bins));
                self.as_mut().bump_recording_revision();
                self.as_mut().set_is_processing(false);
                self.as_mut()
                    .set_status_message(QString::from("录音已保存"));
                true
            }
            WaveformTaskEvent::Failed {
                task_id,
                message,
                captured_path,
            } if task_id == self.rust().active_task_id => {
                self.as_mut().rust_mut().task_receiver = None;
                self.as_mut().set_is_processing(false);
                if let Some(path) = captured_path {
                    let _ = std::fs::remove_file(path);
                    self.as_mut().rust_mut().session.fail(message.clone());
                    self.as_mut().clear_recording();
                }
                self.as_mut().set_status_message(QString::from(&message));
                false
            }
            _ => true,
        }
    }

    fn save_alignment_offset(mut self: Pin<&mut Self>, offset_secs: f64) -> bool {
        let mut recording = match self.rust().session.recording().cloned() {
            Some(recording) => recording,
            None => return false,
        };
        if let Err(error) = self
            .as_mut()
            .rust_mut()
            .manager
            .set_alignment(&mut recording, offset_secs)
        {
            return self.as_mut().report_error("保存录音对齐失败", error);
        }
        self.as_mut()
            .rust_mut()
            .session
            .load_recording(Some(recording.clone()));
        self.as_mut()
            .set_alignment_offset(recording.alignment_offset);
        true
    }

    fn reset_alignment(self: Pin<&mut Self>) -> bool {
        self.save_alignment_offset(0.0)
    }

    fn delete_recording(mut self: Pin<&mut Self>) -> bool {
        if self.rust().is_recording || self.rust().is_processing {
            return false;
        }
        let recording = match self.rust().session.recording().cloned() {
            Some(recording) => recording,
            None => return false,
        };
        if let Err(error) = self.as_mut().rust_mut().manager.delete(&recording) {
            return self.as_mut().report_error("删除录音失败", error);
        }
        let _ = std::fs::remove_file(&recording.file_path);
        self.as_mut().rust_mut().session.load_recording(None);
        self.as_mut().clear_recording();
        self.as_mut()
            .set_status_message(QString::from("录音已删除"));
        true
    }
}

impl qobject::RecordingBridge {
    fn start_captured_waveform_task(
        mut self: Pin<&mut Self>,
        captured: els_audio_capture::CapturedAudio,
    ) {
        let task_id = self.rust().active_task_id.saturating_add(1);
        self.as_mut().rust_mut().active_task_id = task_id;
        let (sender, receiver) = mpsc::channel();
        let path = captured.path.to_string_lossy().into_owned();
        let captured_path = captured.path.clone();
        let duration = captured.duration_secs;
        thread::spawn(move || {
            let engine = els_waveform_core::FfmpegWaveformEngine;
            match engine.generate(&els_waveform_core::AudioSource {
                video_path: Some(path),
                duration_secs: duration,
                quality: els_waveform_core::WaveformQuality::Preview,
            }) {
                Ok(waveform) => {
                    let _ = sender.send(WaveformTaskEvent::Ready {
                        task_id,
                        recording: None,
                        captured: Some(captured),
                        waveform,
                    });
                }
                Err(error) => {
                    let _ = sender.send(WaveformTaskEvent::Failed {
                        task_id,
                        message: format!("生成录音波形失败：{error}"),
                        captured_path: Some(captured_path),
                    });
                }
            }
        });
        self.as_mut().rust_mut().task_receiver = Some(receiver);
    }

    fn start_waveform_task(
        mut self: Pin<&mut Self>,
        recording: els_recording_core::Recording,
        captured: Option<els_audio_capture::CapturedAudio>,
    ) {
        let task_id = self.rust().active_task_id.saturating_add(1);
        self.as_mut().rust_mut().active_task_id = task_id;
        self.as_mut().set_is_processing(true);
        let (sender, receiver) = mpsc::channel();
        let path = recording.file_path.clone();
        let duration = recording.duration_secs;
        thread::spawn(move || {
            let engine = els_waveform_core::FfmpegWaveformEngine;
            match engine.generate(&els_waveform_core::AudioSource {
                video_path: Some(path),
                duration_secs: duration,
                quality: els_waveform_core::WaveformQuality::Preview,
            }) {
                Ok(waveform) => {
                    let _ = sender.send(WaveformTaskEvent::Ready {
                        task_id,
                        recording: Some(recording),
                        captured,
                        waveform,
                    });
                }
                Err(error) => {
                    let _ = sender.send(WaveformTaskEvent::Failed {
                        task_id,
                        message: format!("读取录音波形失败：{error}"),
                        captured_path: None,
                    });
                }
            }
        });
        self.as_mut().rust_mut().task_receiver = Some(receiver);
    }

    fn apply_recording_metadata(
        mut self: Pin<&mut Self>,
        recording: &els_recording_core::Recording,
    ) {
        self.as_mut()
            .set_recording_duration(recording.duration_secs);
        self.as_mut()
            .set_alignment_offset(recording.alignment_offset);
        self.as_mut()
            .set_recording_file_path(QString::from(&recording.file_path));
        self.as_mut().set_has_recording(true);
    }

    fn clear_recording(mut self: Pin<&mut Self>) {
        self.as_mut().set_has_recording(false);
        self.as_mut().set_recording_duration(0.0);
        self.as_mut().set_recording_elapsed(0.0);
        self.as_mut().set_alignment_offset(0.0);
        self.as_mut().set_recording_file_path(QString::from(""));
        self.as_mut()
            .set_recording_peak_values(QVector::from(Vec::new()));
        self.as_mut().bump_recording_revision();
    }

    fn clear_target(mut self: Pin<&mut Self>) {
        let _ = self.as_mut().rust_mut().session.set_target(None);
        self.as_mut().rust_mut().task_receiver = None;
        self.as_mut().set_has_target(false);
        self.as_mut().set_target_start(0.0);
        self.as_mut().set_target_end(0.0);
        self.as_mut().set_is_processing(false);
        self.as_mut().clear_recording();
    }

    fn bump_recording_revision(mut self: Pin<&mut Self>) {
        let revision = self.rust().recording_revision.wrapping_add(1).max(1);
        self.as_mut().set_recording_revision(revision);
    }

    fn fail_session(mut self: Pin<&mut Self>, context: &str, error: els_types::AppError) -> bool {
        let message = format!("{context}：{error}");
        self.as_mut().rust_mut().session.fail(message.clone());
        self.as_mut().set_is_recording(false);
        self.as_mut().set_is_processing(false);
        self.as_mut().set_status_message(QString::from(&message));
        false
    }

    fn report_error(mut self: Pin<&mut Self>, context: &str, error: els_types::AppError) -> bool {
        let message = format!("{context}：{error}");
        eprintln!("{message}");
        self.as_mut().set_status_message(QString::from(&message));
        false
    }
}

enum WaveformTaskEvent {
    Ready {
        task_id: u64,
        recording: Option<els_recording_core::Recording>,
        captured: Option<els_audio_capture::CapturedAudio>,
        waveform: els_waveform_core::WaveformData,
    },
    Failed {
        task_id: u64,
        message: String,
        captured_path: Option<std::path::PathBuf>,
    },
}

fn flatten_bins(bins: &[els_waveform_core::WaveformBin]) -> QVector<f32> {
    let mut values = Vec::with_capacity(bins.len().saturating_mul(2));
    for bin in bins {
        values.push(bin.min);
        values.push(bin.max);
    }
    QVector::from(values)
}
