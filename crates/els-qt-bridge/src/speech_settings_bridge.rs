use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use els_speech_tencent::{TencentAsrConfig, TencentRecognitionMode, PROVIDER_KIND};

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, provider_kind, cxx_name = "providerKind")]
        #[qproperty(QString, recognition_mode, cxx_name = "recognitionMode")]
        #[qproperty(QString, app_id, cxx_name = "appId")]
        #[qproperty(QString, realtime_endpoint, cxx_name = "realtimeEndpoint")]
        #[qproperty(QString, endpoint)]
        #[qproperty(QString, secret_id, cxx_name = "secretId")]
        #[qproperty(QString, secret_key, cxx_name = "secretKey")]
        #[qproperty(QString, region)]
        #[qproperty(QString, engine_model, cxx_name = "engineModel")]
        #[qproperty(QString, status_message, cxx_name = "statusMessage")]
        type SpeechSettingsBridge = super::SpeechSettingsBridgeRust;

        #[qinvokable]
        #[cxx_name = "saveConfig"]
        fn save_config(self: Pin<&mut SpeechSettingsBridge>) -> bool;
    }
}

pub struct SpeechSettingsBridgeRust {
    provider_kind: QString,
    recognition_mode: QString,
    app_id: QString,
    realtime_endpoint: QString,
    endpoint: QString,
    secret_id: QString,
    secret_key: QString,
    region: QString,
    engine_model: QString,
    status_message: QString,
    repository: els_storage::SpeechSettingsRepository,
}

impl Default for SpeechSettingsBridgeRust {
    fn default() -> Self {
        let repository = els_storage::SpeechSettingsRepository::default();
        let config = repository.load_active().ok().flatten().and_then(|profile| {
            serde_json::from_str::<TencentAsrConfig>(&profile.config_json).ok()
        });
        Self {
            provider_kind: QString::from(PROVIDER_KIND),
            recognition_mode: QString::from(
                match config.as_ref().map(|value| value.recognition_mode) {
                    Some(TencentRecognitionMode::SentenceRecognition) => "sentenceRecognition",
                    Some(TencentRecognitionMode::RecordingFile) => "recordingFile",
                    _ => "realtime",
                },
            ),
            app_id: QString::from(
                config
                    .as_ref()
                    .map(|value| value.app_id.as_str())
                    .unwrap_or(""),
            ),
            realtime_endpoint: QString::from(
                config
                    .as_ref()
                    .map(|value| value.realtime_endpoint.as_str())
                    .unwrap_or("wss://asr.cloud.tencent.com"),
            ),
            endpoint: QString::from(
                config
                    .as_ref()
                    .map(|value| value.endpoint.as_str())
                    .unwrap_or("https://asr.tencentcloudapi.com"),
            ),
            secret_id: QString::from(
                config
                    .as_ref()
                    .map(|value| value.secret_id.as_str())
                    .unwrap_or(""),
            ),
            secret_key: QString::from(
                config
                    .as_ref()
                    .map(|value| value.secret_key.as_str())
                    .unwrap_or(""),
            ),
            region: QString::from(
                config
                    .as_ref()
                    .map(|value| value.region.as_str())
                    .unwrap_or(""),
            ),
            engine_model: QString::from(
                config
                    .as_ref()
                    .map(|value| value.engine_model.as_str())
                    .unwrap_or("16k_en"),
            ),
            status_message: QString::from(""),
            repository,
        }
    }
}

impl qobject::SpeechSettingsBridge {
    fn save_config(mut self: Pin<&mut Self>) -> bool {
        if self.rust().provider_kind.to_string() != PROVIDER_KIND {
            self.as_mut()
                .set_status_message(QString::from("当前版本不支持该语音识别提供商"));
            return false;
        }
        let config = TencentAsrConfig {
            recognition_mode: match self.rust().recognition_mode.to_string().as_str() {
                "sentenceRecognition" => TencentRecognitionMode::SentenceRecognition,
                "recordingFile" => TencentRecognitionMode::RecordingFile,
                _ => TencentRecognitionMode::Realtime,
            },
            app_id: self.rust().app_id.to_string(),
            realtime_endpoint: self.rust().realtime_endpoint.to_string(),
            endpoint: self.rust().endpoint.to_string(),
            secret_id: self.rust().secret_id.to_string(),
            secret_key: self.rust().secret_key.to_string(),
            region: self.rust().region.to_string(),
            engine_model: self.rust().engine_model.to_string(),
        }
        .normalized();
        if let Err(error) = config.validate() {
            self.as_mut()
                .set_status_message(QString::from(&error.to_string()));
            return false;
        }
        let config_json = match serde_json::to_string(&config) {
            Ok(value) => value,
            Err(error) => {
                self.as_mut()
                    .set_status_message(QString::from(&format!("保存配置失败：{error}")));
                return false;
            }
        };
        let profile = els_storage::SpeechProviderProfile {
            id: "tencent-default".to_string(),
            name: "腾讯云".to_string(),
            provider_kind: PROVIDER_KIND.to_string(),
            config_json,
            enabled: true,
        };
        match self.as_mut().rust_mut().repository.save_active(&profile) {
            Ok(()) => {
                self.as_mut()
                    .set_status_message(QString::from("语音识别配置已保存"));
                true
            }
            Err(error) => {
                self.as_mut()
                    .set_status_message(QString::from(&format!("保存配置失败：{error}")));
                false
            }
        }
    }
}
