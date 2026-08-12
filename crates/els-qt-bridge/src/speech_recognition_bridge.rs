use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use els_speech_core::{
    RecognitionOptions, RecognitionStatus, RecognitionSubmission, SpeechProviderRegistry,
};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(bool, is_recognizing, cxx_name = "isRecognizing")]
        #[qproperty(QString, status_message, cxx_name = "statusMessage")]
        #[qproperty(QString, error_message, cxx_name = "errorMessage")]
        #[qproperty(QString, result_text, cxx_name = "resultText")]
        #[qproperty(f64, result_start, cxx_name = "resultStart")]
        #[qproperty(f64, result_end, cxx_name = "resultEnd")]
        #[qproperty(i32, result_revision, cxx_name = "resultRevision")]
        type SpeechRecognitionBridge = super::SpeechRecognitionBridgeRust;

        #[qinvokable]
        #[cxx_name = "recognizeSelection"]
        fn recognize_selection(
            self: Pin<&mut SpeechRecognitionBridge>,
            video_path: &QString,
            start_secs: f64,
            end_secs: f64,
        ) -> bool;

        #[qinvokable]
        fn poll(self: Pin<&mut SpeechRecognitionBridge>) -> bool;
    }
}

pub struct SpeechRecognitionBridgeRust {
    is_recognizing: bool,
    status_message: QString,
    error_message: QString,
    result_text: QString,
    result_start: f64,
    result_end: f64,
    result_revision: i32,
    receiver: Option<Receiver<WorkerEvent>>,
}

impl Default for SpeechRecognitionBridgeRust {
    fn default() -> Self {
        Self {
            is_recognizing: false,
            status_message: QString::from(""),
            error_message: QString::from(""),
            result_text: QString::from(""),
            result_start: 0.0,
            result_end: 0.0,
            result_revision: 1,
            receiver: None,
        }
    }
}

enum WorkerEvent {
    Status(String),
    Completed(String),
    Failed(String),
}

impl qobject::SpeechRecognitionBridge {
    fn recognize_selection(
        mut self: Pin<&mut Self>,
        video_path: &QString,
        start_secs: f64,
        end_secs: f64,
    ) -> bool {
        if self.rust().is_recognizing {
            return false;
        }
        let video_path = video_path.to_string();
        if video_path.trim().is_empty() || end_secs <= start_secs {
            self.as_mut()
                .set_error_message(QString::from("请先选择有效的学习片段"));
            return false;
        }
        let profile = match els_storage::SpeechSettingsRepository::default().load_active() {
            Ok(Some(value)) => value,
            Ok(None) => {
                self.as_mut()
                    .set_error_message(QString::from("请先配置语音识别提供商"));
                return false;
            }
            Err(error) => {
                self.as_mut()
                    .set_error_message(QString::from(&format!("读取语音识别配置失败：{error}")));
                return false;
            }
        };
        let (sender, receiver) = mpsc::channel();
        self.as_mut().rust_mut().receiver = Some(receiver);
        self.as_mut().set_is_recognizing(true);
        self.as_mut()
            .set_status_message(QString::from("正在准备当前片段…"));
        self.as_mut().set_error_message(QString::from(""));
        self.as_mut().set_result_start(start_secs);
        self.as_mut().set_result_end(end_secs);

        std::thread::spawn(move || {
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                run_recognition(&profile, &video_path, start_secs, end_secs, &sender)
            }));
            match outcome {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    let _ = sender.send(WorkerEvent::Failed(error));
                }
                Err(panic) => {
                    let detail = panic
                        .downcast_ref::<String>()
                        .cloned()
                        .or_else(|| panic.downcast_ref::<&str>().map(|value| value.to_string()))
                        .unwrap_or_else(|| "未知后台异常".to_string());
                    let _ = sender.send(WorkerEvent::Failed(format!("语音识别后台异常：{detail}")));
                }
            }
        });
        true
    }

    fn poll(mut self: Pin<&mut Self>) -> bool {
        let Some(receiver) = self.as_mut().rust_mut().receiver.take() else {
            return false;
        };
        match receiver.try_recv() {
            Ok(WorkerEvent::Status(message)) => {
                self.as_mut().set_status_message(QString::from(&message));
                self.as_mut().rust_mut().receiver = Some(receiver);
                true
            }
            Ok(WorkerEvent::Completed(text)) => {
                self.as_mut().set_result_text(QString::from(&text));
                let revision = self.rust().result_revision.wrapping_add(1).max(1);
                self.as_mut().set_result_revision(revision);
                self.as_mut()
                    .set_status_message(QString::from("识别完成，已填入字幕编辑区"));
                self.as_mut().set_is_recognizing(false);
                true
            }
            Ok(WorkerEvent::Failed(message)) => {
                self.as_mut().set_error_message(QString::from(&message));
                self.as_mut().set_status_message(QString::from(""));
                self.as_mut().set_is_recognizing(false);
                true
            }
            Err(mpsc::TryRecvError::Empty) => {
                self.as_mut().rust_mut().receiver = Some(receiver);
                false
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.as_mut()
                    .set_error_message(QString::from("语音识别任务已断开"));
                self.as_mut().set_is_recognizing(false);
                false
            }
        }
    }
}

fn run_recognition(
    profile: &els_storage::SpeechProviderProfile,
    video_path: &str,
    start_secs: f64,
    end_secs: f64,
    sender: &mpsc::Sender<WorkerEvent>,
) -> Result<(), String> {
    let mut registry = SpeechProviderRegistry::default();
    registry.register(Box::new(els_speech_tencent::TencentAsrProviderFactory));
    let provider = registry
        .create(&profile.provider_kind, &profile.config_json)
        .map_err(|error| error.to_string())?;
    let capabilities = provider.capabilities();
    let _ = sender.send(WorkerEvent::Status("正在提取当前片段音频…".to_string()));
    let audio = els_audio_export::export_video_range(
        video_path,
        els_types::TimeRange {
            start: start_secs,
            end: end_secs,
        },
        capabilities.preferred_format,
        capabilities.sample_rate,
        capabilities.channels,
    )
    .map_err(|error| error.to_string())?;
    let _ = sender.send(WorkerEvent::Status("正在提交语音识别任务…".to_string()));
    let submission = provider
        .submit(
            audio.input(),
            &RecognitionOptions {
                language: String::new(),
            },
        )
        .map_err(|error| error.to_string())?;
    let task = match submission {
        RecognitionSubmission::Completed(result) => {
            let _ = sender.send(WorkerEvent::Completed(result.text));
            return Ok(());
        }
        RecognitionSubmission::Pending(task) => task,
    };
    for _ in 0..120 {
        std::thread::sleep(Duration::from_secs(1));
        match provider.query(&task).map_err(|error| error.to_string())? {
            RecognitionStatus::Waiting(message) => {
                let _ = sender.send(WorkerEvent::Status(if message.trim().is_empty() {
                    "腾讯云正在识别…".to_string()
                } else {
                    message
                }));
            }
            RecognitionStatus::Completed(result) => {
                let _ = sender.send(WorkerEvent::Completed(result.text));
                return Ok(());
            }
            RecognitionStatus::Failed(message) => return Err(message),
        }
    }
    Err("语音识别任务超时".to_string())
}
