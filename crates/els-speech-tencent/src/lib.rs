use base64::Engine;
use els_speech_core::{
    AudioFormat, AudioInput, ProviderCapabilities, RecognitionOptions, RecognitionResult,
    RecognitionStatus, RecognitionSubmission, RecognitionTask, SpeechError, SpeechProviderFactory,
    SpeechRecognizer,
};
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tungstenite::{connect, Message};
use url::Url;

pub const PROVIDER_KIND: &str = "tencent-asr";
pub const SUPPORTED_ENGINE_MODELS: &[&str] = &[
    "16k_zh",
    "16k_zh-PY",
    "16k_zh_medical",
    "16k_en",
    "16k_yue",
    "16k_ja",
    "16k_ko",
    "16k_vi",
    "16k_ms",
    "16k_id",
    "16k_fil",
    "16k_th",
    "16k_pt",
    "16k_tr",
    "16k_ar",
    "16k_es",
    "16k_hi",
    "16k_fr",
    "16k_de",
    "16k_zh_dialect",
];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TencentRecognitionMode {
    #[default]
    Realtime,
    SentenceRecognition,
    RecordingFile,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TencentAsrConfig {
    #[serde(default)]
    pub recognition_mode: TencentRecognitionMode,
    #[serde(default)]
    pub app_id: String,
    #[serde(default = "default_realtime_endpoint")]
    pub realtime_endpoint: String,
    pub endpoint: String,
    pub secret_id: String,
    pub secret_key: String,
    pub region: String,
    pub engine_model: String,
}

impl TencentAsrConfig {
    pub fn normalized(mut self) -> Self {
        self.app_id = self.app_id.trim().to_string();
        self.realtime_endpoint = self.realtime_endpoint.trim().to_string();
        self.endpoint = self.endpoint.trim().to_string();
        self.secret_id = self.secret_id.trim().to_string();
        self.secret_key = self.secret_key.trim().to_string();
        self.region = self.region.trim().to_string();
        self.engine_model = match self.engine_model.trim() {
            "16k_zh_en" => "16k_zh-PY".to_string(),
            value => value.to_string(),
        };
        self
    }

    pub fn validate(&self) -> Result<(), SpeechError> {
        for (value, name) in [
            (&self.secret_id, "SecretId"),
            (&self.secret_key, "SecretKey"),
            (&self.engine_model, "语言模型"),
        ] {
            if value.trim().is_empty() {
                return Err(SpeechError::Configuration(format!("{name} 不能为空")));
            }
        }
        if !SUPPORTED_ENGINE_MODELS.contains(&self.engine_model.as_str()) {
            return Err(SpeechError::Configuration(format!(
                "不支持的语言模型：{}",
                self.engine_model
            )));
        }
        if !self.secret_id.trim().starts_with("AKID") {
            return Err(SpeechError::Configuration(
                "SecretId 格式不正确，应使用腾讯云 API 密钥中的 AKID…".to_string(),
            ));
        }
        if matches!(self.secret_key.chars().next(), Some('\'' | '"'))
            || matches!(self.secret_key.chars().last(), Some('\'' | '"'))
        {
            return Err(SpeechError::Configuration(
                "SecretKey 首尾不能包含引号，请只粘贴密钥内容".to_string(),
            ));
        }
        if self.secret_key.chars().any(char::is_control) {
            return Err(SpeechError::Configuration(
                "SecretKey 包含不可见控制字符，请重新粘贴".to_string(),
            ));
        }
        match self.recognition_mode {
            TencentRecognitionMode::Realtime => {
                if self.app_id.is_empty() {
                    return Err(SpeechError::Configuration("AppID 不能为空".to_string()));
                }
                if !self.app_id.chars().all(|value| value.is_ascii_digit()) {
                    return Err(SpeechError::Configuration("AppID 应为纯数字".to_string()));
                }
                parse_realtime_endpoint(&self.realtime_endpoint)?;
            }
            TencentRecognitionMode::SentenceRecognition | TencentRecognitionMode::RecordingFile => {
                if self.endpoint.is_empty() {
                    return Err(SpeechError::Configuration("Endpoint 不能为空".to_string()));
                }
                parse_endpoint(&self.endpoint)?;
            }
        }
        Ok(())
    }
}

fn default_realtime_endpoint() -> String {
    "wss://asr.cloud.tencent.com".to_string()
}

pub struct TencentAsrProvider {
    config: TencentAsrConfig,
}

impl TencentAsrProvider {
    pub fn new(config: TencentAsrConfig) -> Result<Self, SpeechError> {
        let config = config.normalized();
        config.validate()?;
        Ok(Self { config })
    }

    fn request(
        &self,
        action: &str,
        payload: &serde_json::Value,
    ) -> Result<serde_json::Value, SpeechError> {
        let payload = serde_json::to_string(payload)
            .map_err(|error| SpeechError::Protocol(format!("生成腾讯云请求失败：{error}")))?;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| SpeechError::Protocol(format!("系统时间无效：{error}")))?
            .as_secs() as i64;
        let signed = sign_request(&self.config, action, &payload, timestamp)?;
        let mut command = Command::new("curl");
        command
            .args([
                "--silent",
                "--show-error",
                "--fail-with-body",
                "--max-time",
                "30",
            ])
            .arg(&self.config.endpoint)
            .arg("-H")
            .arg("Content-Type: application/json; charset=utf-8")
            .arg("-H")
            .arg(format!("Host: {}", signed.host))
            .arg("-H")
            .arg(format!("X-TC-Action: {action}"))
            .arg("-H")
            .arg("X-TC-Version: 2019-06-14")
            .arg("-H")
            .arg(format!("X-TC-Timestamp: {timestamp}"))
            .arg("-H")
            .arg(format!("Authorization: {}", signed.authorization));
        if !self.config.region.trim().is_empty() {
            command
                .arg("-H")
                .arg(format!("X-TC-Region: {}", self.config.region.trim()));
        }
        let mut child = command
            .args(["--data-binary", "@-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| SpeechError::Transport(format!("启动腾讯云请求失败：{error}")))?;
        child
            .stdin
            .take()
            .ok_or_else(|| SpeechError::Transport("无法写入腾讯云请求".to_string()))?
            .write_all(payload.as_bytes())
            .map_err(|error| SpeechError::Transport(format!("写入腾讯云请求失败：{error}")))?;
        let output = child
            .wait_with_output()
            .map_err(|error| SpeechError::Transport(format!("等待腾讯云响应失败：{error}")))?;
        let body = String::from_utf8_lossy(&output.stdout).to_string();
        if !output.status.success() {
            return Err(SpeechError::Transport(if body.trim().is_empty() {
                String::from_utf8_lossy(&output.stderr).trim().to_string()
            } else {
                body
            }));
        }
        let value: serde_json::Value = serde_json::from_str(&body)
            .map_err(|error| SpeechError::Protocol(format!("解析腾讯云响应失败：{error}")))?;
        if let Some(error) = value["Response"]["Error"]["Message"].as_str() {
            let code = value["Response"]["Error"]["Code"]
                .as_str()
                .unwrap_or_default();
            let message = if code.starts_with("AuthFailure") {
                "腾讯云鉴权失败：请检查 SecretId、SecretKey 和系统时间，确保密钥没有包含引号或其他多余字符".to_string()
            } else {
                error.to_string()
            };
            return Err(SpeechError::Provider(message));
        }
        Ok(value)
    }

    fn submit_recording_file(
        &self,
        audio: &AudioInput,
    ) -> Result<RecognitionSubmission, SpeechError> {
        let bytes = std::fs::read(&audio.path)
            .map_err(|error| SpeechError::Audio(format!("读取待识别音频失败：{error}")))?;
        let payload = serde_json::json!({
            "EngineModelType": self.config.engine_model,
            "ChannelNum": 1,
            "ResTextFormat": 0,
            "SourceType": 1,
            "Data": base64::engine::general_purpose::STANDARD.encode(&bytes),
            "DataLen": bytes.len()
        });
        let response = self.request("CreateRecTask", &payload)?;
        let task_id = response["Response"]["Data"]["TaskId"]
            .as_u64()
            .map(|value| value.to_string())
            .or_else(|| {
                response["Response"]["Data"]["TaskId"]
                    .as_str()
                    .map(str::to_string)
            })
            .ok_or_else(|| SpeechError::Protocol("腾讯云响应缺少 TaskId".to_string()))?;
        Ok(RecognitionSubmission::Pending(RecognitionTask {
            provider_kind: PROVIDER_KIND.to_string(),
            task_id,
        }))
    }

    fn submit_sentence_recognition(
        &self,
        audio: &AudioInput,
    ) -> Result<RecognitionSubmission, SpeechError> {
        if audio.format != AudioFormat::WavPcm16 {
            return Err(SpeechError::Audio(
                "腾讯云一句话识别需要 WAV PCM 音频".to_string(),
            ));
        }
        let bytes = std::fs::read(&audio.path)
            .map_err(|error| SpeechError::Audio(format!("读取待识别音频失败：{error}")))?;
        if bytes.is_empty() {
            return Err(SpeechError::Audio("当前片段没有可识别的音频".to_string()));
        }
        if wav_duration_secs(&bytes).is_some_and(|duration| duration > 60.0) {
            return Err(SpeechError::Audio(
                "一句话识别仅支持不超过 60 秒的片段，请缩短选区或切换识别模式".to_string(),
            ));
        }
        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
        if encoded.len() > 3 * 1024 * 1024 {
            return Err(SpeechError::Audio(
                "一句话识别音频编码后不能超过 3MB，请缩短选区或切换识别模式".to_string(),
            ));
        }
        let payload = serde_json::json!({
            "EngSerViceType": sentence_engine_model(&self.config.engine_model),
            "SourceType": 1,
            "VoiceFormat": "wav",
            "Data": encoded,
            "DataLen": bytes.len()
        });
        let response = self.request("SentenceRecognition", &payload)?;
        let text = response["Response"]["Result"]
            .as_str()
            .ok_or_else(|| SpeechError::Protocol("腾讯云一句话识别响应缺少 Result".to_string()))?
            .trim()
            .to_string();
        Ok(RecognitionSubmission::Completed(RecognitionResult { text }))
    }

    fn submit_realtime(&self, audio: &AudioInput) -> Result<RecognitionSubmission, SpeechError> {
        if audio.format != AudioFormat::RawPcm16 {
            return Err(SpeechError::Audio(
                "腾讯云实时识别需要 16kHz 单声道 PCM 音频".to_string(),
            ));
        }
        let bytes = std::fs::read(&audio.path)
            .map_err(|error| SpeechError::Audio(format!("读取待识别音频失败：{error}")))?;
        if bytes.is_empty() {
            return Err(SpeechError::Audio("当前片段没有可识别的音频".to_string()));
        }
        let timestamp = current_timestamp()?;
        let voice_id = new_voice_id(timestamp);
        let signed = build_realtime_request(&self.config, &voice_id, timestamp)?;
        ensure_tls_crypto_provider();
        let (mut socket, _) = connect(signed.request_url.as_str())
            .map_err(|error| SpeechError::Transport(format!("连接腾讯云实时识别失败：{error}")))?;

        let started = socket
            .read()
            .map_err(|error| SpeechError::Transport(format!("读取腾讯云握手响应失败：{error}")))?;
        ensure_success_response(&started)?;

        const FRAME_BYTES: usize = 6_400;
        for (index, chunk) in bytes.chunks(FRAME_BYTES).enumerate() {
            socket
                .send(Message::Binary(chunk.to_vec().into()))
                .map_err(|error| {
                    SpeechError::Transport(format!("发送实时识别音频失败：{error}"))
                })?;
            if (index + 1) * FRAME_BYTES < bytes.len() {
                std::thread::sleep(Duration::from_millis(200));
            }
        }
        socket
            .send(Message::Text(r#"{"type":"end"}"#.into()))
            .map_err(|error| SpeechError::Transport(format!("结束实时识别失败：{error}")))?;

        let mut sentences = BTreeMap::<i64, String>::new();
        loop {
            let message = socket.read().map_err(|error| {
                SpeechError::Transport(format!("读取实时识别结果失败：{error}"))
            })?;
            let response = parse_realtime_response(&message)?;
            if response.code != 0 {
                return Err(SpeechError::Provider(format_provider_error(
                    response.code,
                    &response.message,
                )));
            }
            if let Some(result) = response.result {
                let text = result.voice_text_str.trim();
                if !text.is_empty() {
                    sentences.insert(result.index, text.to_string());
                }
            }
            if response.final_flag == 1 {
                break;
            }
        }
        let separator = if self.config.engine_model.contains("zh") {
            ""
        } else {
            " "
        };
        let text = sentences.into_values().collect::<Vec<_>>().join(separator);
        Ok(RecognitionSubmission::Completed(RecognitionResult { text }))
    }
}

impl SpeechRecognizer for TencentAsrProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            preferred_format: match self.config.recognition_mode {
                TencentRecognitionMode::Realtime => AudioFormat::RawPcm16,
                TencentRecognitionMode::SentenceRecognition
                | TencentRecognitionMode::RecordingFile => AudioFormat::WavPcm16,
            },
            sample_rate: 16_000,
            channels: 1,
            supports_async_tasks: self.config.recognition_mode
                == TencentRecognitionMode::RecordingFile,
        }
    }

    fn submit(
        &self,
        audio: &AudioInput,
        _options: &RecognitionOptions,
    ) -> Result<RecognitionSubmission, SpeechError> {
        match self.config.recognition_mode {
            TencentRecognitionMode::Realtime => self.submit_realtime(audio),
            TencentRecognitionMode::SentenceRecognition => self.submit_sentence_recognition(audio),
            TencentRecognitionMode::RecordingFile => self.submit_recording_file(audio),
        }
    }

    fn query(&self, task: &RecognitionTask) -> Result<RecognitionStatus, SpeechError> {
        if self.config.recognition_mode != TencentRecognitionMode::RecordingFile {
            return Err(SpeechError::Protocol(
                "当前识别模式会直接返回结果，不支持查询异步任务".to_string(),
            ));
        }
        let task_id = task
            .task_id
            .parse::<u64>()
            .map_err(|_| SpeechError::Protocol("腾讯云 TaskId 无效".to_string()))?;
        let response = self.request(
            "DescribeTaskStatus",
            &serde_json::json!({ "TaskId": task_id }),
        )?;
        let data = &response["Response"]["Data"];
        let status = data["Status"].as_i64().unwrap_or_default();
        let status_text = data["StatusStr"].as_str().unwrap_or("腾讯云正在识别");
        match status {
            2 => Ok(RecognitionStatus::Completed(RecognitionResult {
                text: data["Result"]
                    .as_str()
                    .unwrap_or_default()
                    .trim()
                    .to_string(),
            })),
            3 => Ok(RecognitionStatus::Failed(
                data["ErrorMsg"]
                    .as_str()
                    .unwrap_or("腾讯云识别任务失败")
                    .to_string(),
            )),
            _ => Ok(RecognitionStatus::Waiting(status_text.to_string())),
        }
    }
}

pub struct TencentAsrProviderFactory;

impl SpeechProviderFactory for TencentAsrProviderFactory {
    fn provider_kind(&self) -> &'static str {
        PROVIDER_KIND
    }

    fn create(&self, config_json: &str) -> Result<Box<dyn SpeechRecognizer>, SpeechError> {
        let config = serde_json::from_str(config_json)
            .map_err(|error| SpeechError::Configuration(format!("腾讯云配置无效：{error}")))?;
        Ok(Box::new(TencentAsrProvider::new(config)?))
    }
}

fn ensure_tls_crypto_provider() {
    static INSTALL: std::sync::Once = std::sync::Once::new();
    INSTALL.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

fn sentence_engine_model(configured: &str) -> &str {
    match configured.trim() {
        "16k_zh_en" => "16k_zh-PY",
        value => value,
    }
}

fn wav_duration_secs(bytes: &[u8]) -> Option<f64> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return None;
    }
    let mut offset = 12usize;
    let mut bytes_per_second = None;
    let mut data_len = None;
    while offset.checked_add(8)? <= bytes.len() {
        let chunk_id = &bytes[offset..offset + 4];
        let chunk_len = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().ok()?) as usize;
        let chunk_start = offset + 8;
        let chunk_end = chunk_start.checked_add(chunk_len)?;
        if chunk_end > bytes.len() {
            return None;
        }
        if chunk_id == b"fmt " && chunk_len >= 12 {
            bytes_per_second = Some(u32::from_le_bytes(
                bytes[chunk_start + 8..chunk_start + 12].try_into().ok()?,
            ));
        } else if chunk_id == b"data" {
            data_len = Some(chunk_len);
        }
        offset = chunk_end + (chunk_len % 2);
    }
    let bytes_per_second = bytes_per_second?;
    let data_len = data_len?;
    (bytes_per_second > 0).then(|| data_len as f64 / bytes_per_second as f64)
}

#[derive(Debug, Deserialize)]
struct RealtimeResponse {
    #[serde(default)]
    code: i64,
    #[serde(default)]
    message: String,
    #[serde(default, rename = "final")]
    final_flag: u32,
    #[serde(default)]
    result: Option<RealtimeResult>,
}

#[derive(Debug, Deserialize)]
struct RealtimeResult {
    #[serde(default)]
    index: i64,
    #[serde(default)]
    voice_text_str: String,
}

struct SignedRealtimeRequest {
    request_url: String,
    #[cfg_attr(not(test), allow(dead_code))]
    signing_target: String,
}

fn build_realtime_request(
    config: &TencentAsrConfig,
    voice_id: &str,
    timestamp: i64,
) -> Result<SignedRealtimeRequest, SpeechError> {
    let endpoint = parse_realtime_endpoint(&config.realtime_endpoint)?;
    let host = endpoint
        .host_str()
        .ok_or_else(|| SpeechError::Configuration("实时 Endpoint 缺少主机名".to_string()))?;
    let authority = match endpoint.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_string(),
    };
    let base_path = endpoint.path().trim_end_matches('/');
    let request_path = format!("{base_path}/asr/v2/{}", config.app_id);
    let mut query = BTreeMap::new();
    query.insert("convert_num_mode", "1".to_string());
    query.insert("engine_model_type", config.engine_model.clone());
    query.insert("expired", (timestamp + 86_400).to_string());
    query.insert("filter_dirty", "0".to_string());
    query.insert("filter_empty_result", "1".to_string());
    query.insert("filter_modal", "0".to_string());
    query.insert("filter_punc", "0".to_string());
    query.insert("max_speak_time", "0".to_string());
    query.insert("needvad", "1".to_string());
    query.insert("nonce", timestamp.to_string());
    query.insert("reinforce_hotword", "0".to_string());
    query.insert("secretid", config.secret_id.clone());
    query.insert("timestamp", timestamp.to_string());
    query.insert("voice_format", "1".to_string());
    query.insert("voice_id", voice_id.to_string());
    query.insert("word_info", "0".to_string());
    let query = query
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&");
    let signing_target = format!("{authority}{request_path}?{query}");
    let signature = base64::engine::general_purpose::STANDARD.encode(hmac_sha1(
        config.secret_key.as_bytes(),
        signing_target.as_bytes(),
    ));
    let encoded_signature: String =
        url::form_urlencoded::byte_serialize(signature.as_bytes()).collect();
    Ok(SignedRealtimeRequest {
        request_url: format!(
            "{}://{signing_target}&signature={encoded_signature}",
            endpoint.scheme()
        ),
        signing_target,
    })
}

fn parse_realtime_endpoint(endpoint: &str) -> Result<Url, SpeechError> {
    let value = Url::parse(endpoint.trim())
        .map_err(|error| SpeechError::Configuration(format!("实时 Endpoint 格式无效：{error}")))?;
    if !matches!(value.scheme(), "ws" | "wss") {
        return Err(SpeechError::Configuration(
            "实时 Endpoint 必须以 ws:// 或 wss:// 开头".to_string(),
        ));
    }
    if value.host_str().is_none() {
        return Err(SpeechError::Configuration(
            "实时 Endpoint 缺少主机名".to_string(),
        ));
    }
    if value.query().is_some() || value.fragment().is_some() {
        return Err(SpeechError::Configuration(
            "实时 Endpoint 不能包含查询参数或片段".to_string(),
        ));
    }
    Ok(value)
}

fn parse_realtime_response(message: &Message) -> Result<RealtimeResponse, SpeechError> {
    let bytes = match message {
        Message::Text(value) => value.as_bytes(),
        Message::Binary(value) => value.as_ref(),
        _ => {
            return Err(SpeechError::Protocol(
                "腾讯云返回了非文本识别消息".to_string(),
            ))
        }
    };
    serde_json::from_slice(bytes)
        .map_err(|error| SpeechError::Protocol(format!("解析腾讯云实时识别响应失败：{error}")))
}

fn ensure_success_response(message: &Message) -> Result<(), SpeechError> {
    let response = parse_realtime_response(message)?;
    if response.code == 0 {
        Ok(())
    } else {
        Err(SpeechError::Provider(format_provider_error(
            response.code,
            &response.message,
        )))
    }
}

fn format_provider_error(code: i64, message: &str) -> String {
    if code == 4001 || message.to_ascii_lowercase().contains("signature") {
        "腾讯云实时识别鉴权失败：请检查 AppID、SecretId、SecretKey 和系统时间".to_string()
    } else {
        format!("腾讯云实时识别失败（{code}）：{message}")
    }
}

fn current_timestamp() -> Result<i64, SpeechError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs() as i64)
        .map_err(|error| SpeechError::Protocol(format!("系统时间无效：{error}")))
}

fn new_voice_id(timestamp: i64) -> String {
    static NEXT_ID: AtomicU64 = AtomicU64::new(1);
    format!(
        "lls-{}-{timestamp}-{}",
        std::process::id(),
        NEXT_ID.fetch_add(1, Ordering::Relaxed)
    )
}

struct SignedRequest {
    host: String,
    authorization: String,
}

fn sign_request(
    config: &TencentAsrConfig,
    action: &str,
    payload: &str,
    timestamp: i64,
) -> Result<SignedRequest, SpeechError> {
    let (host, canonical_uri) = parse_endpoint(&config.endpoint)?;
    let date = unix_date(timestamp)?;
    let canonical_headers = format!(
        "content-type:application/json; charset=utf-8\nhost:{host}\nx-tc-action:{}\n",
        action.to_ascii_lowercase()
    );
    let signed_headers = "content-type;host;x-tc-action";
    let canonical_request = format!(
        "POST\n{canonical_uri}\n\n{canonical_headers}\n{signed_headers}\n{}",
        sha256_hex(payload.as_bytes())
    );
    let scope = format!("{date}/asr/tc3_request");
    let string_to_sign = format!(
        "TC3-HMAC-SHA256\n{timestamp}\n{scope}\n{}",
        sha256_hex(canonical_request.as_bytes())
    );
    let secret_date = hmac_sha256(
        format!("TC3{}", config.secret_key).as_bytes(),
        date.as_bytes(),
    );
    let secret_service = hmac_sha256(&secret_date, b"asr");
    let secret_signing = hmac_sha256(&secret_service, b"tc3_request");
    let signature = hex(&hmac_sha256(&secret_signing, string_to_sign.as_bytes()));
    let authorization = format!(
        "TC3-HMAC-SHA256 Credential={}/{scope}, SignedHeaders={signed_headers}, Signature={signature}",
        config.secret_id.trim()
    );
    Ok(SignedRequest {
        host,
        authorization,
    })
}

fn parse_endpoint(endpoint: &str) -> Result<(String, String), SpeechError> {
    let value = endpoint.trim();
    let without_scheme = value
        .strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://"))
        .ok_or_else(|| {
            SpeechError::Configuration("Endpoint 必须以 http:// 或 https:// 开头".to_string())
        })?;
    let (host, path) = without_scheme
        .split_once('/')
        .unwrap_or((without_scheme, ""));
    if host.trim().is_empty() {
        return Err(SpeechError::Configuration(
            "Endpoint 缺少主机名".to_string(),
        ));
    }
    Ok((
        host.to_string(),
        format!("/{}", path.trim_start_matches('/')),
    ))
}

fn sha256_hex(value: &[u8]) -> String {
    hex(&Sha256::digest(value))
}

fn hmac_sha256(key: &[u8], message: &[u8]) -> Vec<u8> {
    const BLOCK_SIZE: usize = 64;
    let mut normalized = if key.len() > BLOCK_SIZE {
        Sha256::digest(key).to_vec()
    } else {
        key.to_vec()
    };
    normalized.resize(BLOCK_SIZE, 0);
    let mut inner_key = [0x36_u8; BLOCK_SIZE];
    let mut outer_key = [0x5c_u8; BLOCK_SIZE];
    for index in 0..BLOCK_SIZE {
        inner_key[index] ^= normalized[index];
        outer_key[index] ^= normalized[index];
    }
    let mut inner = Sha256::new();
    inner.update(inner_key);
    inner.update(message);
    let inner_hash = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(outer_key);
    outer.update(inner_hash);
    outer.finalize().to_vec()
}

fn hmac_sha1(key: &[u8], message: &[u8]) -> Vec<u8> {
    const BLOCK_SIZE: usize = 64;
    let mut normalized = if key.len() > BLOCK_SIZE {
        Sha1::digest(key).to_vec()
    } else {
        key.to_vec()
    };
    normalized.resize(BLOCK_SIZE, 0);
    let mut inner_key = [0x36_u8; BLOCK_SIZE];
    let mut outer_key = [0x5c_u8; BLOCK_SIZE];
    for index in 0..BLOCK_SIZE {
        inner_key[index] ^= normalized[index];
        outer_key[index] ^= normalized[index];
    }
    let mut inner = Sha1::new();
    inner.update(inner_key);
    inner.update(message);
    let inner_hash = inner.finalize();
    let mut outer = Sha1::new();
    outer.update(outer_key);
    outer.update(inner_hash);
    outer.finalize().to_vec()
}

fn hex(value: &[u8]) -> String {
    value.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn unix_date(timestamp: i64) -> Result<String, SpeechError> {
    if timestamp < 0 {
        return Err(SpeechError::Protocol("系统时间早于 Unix Epoch".to_string()));
    }
    let days = timestamp / 86_400;
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += if month <= 2 { 1 } else { 0 };
    Ok(format!("{year:04}-{month:02}-{day:02}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_config_and_builds_a_stable_signature() {
        let config = TencentAsrConfig {
            recognition_mode: TencentRecognitionMode::RecordingFile,
            app_id: String::new(),
            realtime_endpoint: default_realtime_endpoint(),
            endpoint: "https://asr.example.test/custom".into(),
            secret_id: "id".into(),
            secret_key: "key".into(),
            region: "ap-shanghai".into(),
            engine_model: "16k_en".into(),
        };
        let signed = sign_request(&config, "CreateRecTask", "{}", 1_700_000_000).unwrap();
        assert_eq!(signed.host, "asr.example.test");
        assert!(signed
            .authorization
            .contains("Credential=id/2023-11-14/asr/tc3_request"));
        assert!(signed.authorization.contains("Signature="));
    }

    #[test]
    fn hmac_matches_a_known_vector() {
        assert_eq!(
            hex(&hmac_sha256(
                b"key",
                b"The quick brown fox jumps over the lazy dog"
            )),
            "f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8"
        );
    }

    #[test]
    fn normalizes_whitespace_and_rejects_quoted_credentials() {
        let normalized = TencentAsrConfig {
            recognition_mode: TencentRecognitionMode::Realtime,
            app_id: " 123456 ".into(),
            realtime_endpoint: " wss://asr.cloud.tencent.com ".into(),
            endpoint: " https://asr.example.test ".into(),
            secret_id: " AKIDexample ".into(),
            secret_key: " key123 ".into(),
            region: " ap-shanghai ".into(),
            engine_model: " 16k_en ".into(),
        }
        .normalized();
        assert_eq!(normalized.secret_key, "key123");
        assert_eq!(normalized.app_id, "123456");
        assert!(normalized.validate().is_ok());

        for engine_model in SUPPORTED_ENGINE_MODELS {
            let mut supported = normalized.clone();
            supported.engine_model = (*engine_model).to_string();
            assert!(supported.validate().is_ok(), "{engine_model}");
        }

        let mut legacy = normalized.clone();
        legacy.engine_model = " 16k_zh_en ".into();
        assert_eq!(legacy.normalized().engine_model, "16k_zh-PY");

        let mut unsupported = normalized.clone();
        unsupported.engine_model = "custom_model".into();
        assert!(unsupported.validate().is_err());

        let mut quoted = normalized;
        quoted.secret_key = "\"key123".into();
        assert!(quoted.validate().is_err());
    }

    #[test]
    fn old_config_defaults_to_realtime_mode() {
        let config: TencentAsrConfig = serde_json::from_str(
            r#"{
                "endpoint":"https://asr.tencentcloudapi.com",
                "secretId":"AKIDexample",
                "secretKey":"key123",
                "region":"ap-shanghai",
                "engineModel":"16k_en"
            }"#,
        )
        .unwrap();
        assert_eq!(config.recognition_mode, TencentRecognitionMode::Realtime);
        assert_eq!(config.realtime_endpoint, "wss://asr.cloud.tencent.com");
        assert!(config.app_id.is_empty());
    }

    #[test]
    fn builds_sorted_realtime_url_and_signature() {
        let config = TencentAsrConfig {
            recognition_mode: TencentRecognitionMode::Realtime,
            app_id: "123456".into(),
            realtime_endpoint: "wss://asr.example.test/gateway".into(),
            endpoint: String::new(),
            secret_id: "AKIDexample".into(),
            secret_key: "secret".into(),
            region: String::new(),
            engine_model: "16k_en".into(),
        };
        let signed = build_realtime_request(&config, "voice-1", 1_700_000_000).unwrap();
        assert!(signed
            .signing_target
            .starts_with("asr.example.test/gateway/asr/v2/123456?convert_num_mode=1&"));
        assert!(signed.signing_target.contains("expired=1700086400"));
        assert!(signed.signing_target.contains("secretid=AKIDexample"));
        assert!(signed.request_url.starts_with("wss://asr.example.test/"));
        assert!(signed.request_url.contains("&signature="));
    }

    #[test]
    fn hmac_sha1_matches_a_known_vector() {
        assert_eq!(
            hex(&hmac_sha1(
                b"key",
                b"The quick brown fox jumps over the lazy dog"
            )),
            "de7c9b85b8b78aa6bc8a7a36f70a90701c9db4d9"
        );
    }

    #[test]
    fn parses_realtime_sentence_and_final_messages() {
        let sentence = Message::Text(
            r#"{"code":0,"final":0,"result":{"index":2,"voice_text_str":"hello"}}"#.into(),
        );
        let response = parse_realtime_response(&sentence).unwrap();
        assert_eq!(response.result.unwrap().voice_text_str, "hello");

        let final_message = Message::Text(r#"{"code":0,"final":1}"#.into());
        assert_eq!(
            parse_realtime_response(&final_message).unwrap().final_flag,
            1
        );
    }

    #[test]
    fn maps_mixed_realtime_model_for_sentence_recognition() {
        assert_eq!(sentence_engine_model("16k_zh_en"), "16k_zh-PY");
        assert_eq!(sentence_engine_model("16k_en"), "16k_en");
    }

    #[test]
    fn reads_pcm_wav_duration() {
        let data_len = 32_000u32;
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + data_len).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&16_000u32.to_le_bytes());
        wav.extend_from_slice(&32_000u32.to_le_bytes());
        wav.extend_from_slice(&2u16.to_le_bytes());
        wav.extend_from_slice(&16u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_len.to_le_bytes());
        wav.resize(wav.len() + data_len as usize, 0);
        assert_eq!(wav_duration_secs(&wav), Some(1.0));
    }
}
