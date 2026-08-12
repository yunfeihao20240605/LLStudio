use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioFormat {
    WavPcm16,
    RawPcm16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderCapabilities {
    pub preferred_format: AudioFormat,
    pub sample_rate: u32,
    pub channels: u16,
    pub supports_async_tasks: bool,
}

#[derive(Debug, Clone)]
pub struct AudioInput {
    pub path: PathBuf,
    pub format: AudioFormat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecognitionOptions {
    pub language: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecognitionResult {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecognitionTask {
    pub provider_kind: String,
    pub task_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecognitionSubmission {
    Completed(RecognitionResult),
    Pending(RecognitionTask),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecognitionStatus {
    Waiting(String),
    Completed(RecognitionResult),
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpeechError {
    Configuration(String),
    Audio(String),
    Transport(String),
    Protocol(String),
    Provider(String),
    Cancelled,
}

impl std::fmt::Display for SpeechError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Configuration(value)
            | Self::Audio(value)
            | Self::Transport(value)
            | Self::Protocol(value)
            | Self::Provider(value) => formatter.write_str(value),
            Self::Cancelled => formatter.write_str("语音识别已取消"),
        }
    }
}

impl std::error::Error for SpeechError {}

pub trait SpeechRecognizer: Send + Sync {
    fn capabilities(&self) -> ProviderCapabilities;
    fn submit(
        &self,
        audio: &AudioInput,
        options: &RecognitionOptions,
    ) -> Result<RecognitionSubmission, SpeechError>;
    fn query(&self, task: &RecognitionTask) -> Result<RecognitionStatus, SpeechError>;
}

pub trait SpeechProviderFactory: Send + Sync {
    fn provider_kind(&self) -> &'static str;
    fn create(&self, config_json: &str) -> Result<Box<dyn SpeechRecognizer>, SpeechError>;
}

#[derive(Default)]
pub struct SpeechProviderRegistry {
    factories: HashMap<String, Box<dyn SpeechProviderFactory>>,
}

impl SpeechProviderRegistry {
    pub fn register(&mut self, factory: Box<dyn SpeechProviderFactory>) {
        self.factories
            .insert(factory.provider_kind().to_string(), factory);
    }

    pub fn create(
        &self,
        provider_kind: &str,
        config_json: &str,
    ) -> Result<Box<dyn SpeechRecognizer>, SpeechError> {
        self.factories
            .get(provider_kind)
            .ok_or_else(|| {
                SpeechError::Configuration(format!("不支持的语音识别提供商：{provider_kind}"))
            })?
            .create(config_json)
    }
}
