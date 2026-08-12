use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use els_ai_core::{
    render_markdown, AiConfig, AiProtocol, AiStreamEvent, ChatMessage, Conversation, MessageRole,
    OpenAiCompatibleProvider, SubtitleContext,
};
use std::sync::mpsc::{self, Receiver};

const DEFAULT_SYSTEM_PROMPT: &str = "你是一名英语语音学习教练。请基于当前字幕，帮助用户学习发音、连读、弱读、重音和语调。回答要具体、简洁，并给出可模仿的读法。";

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }
    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, base_url, cxx_name = "baseUrl")]
        #[qproperty(QString, api_key, cxx_name = "apiKey")]
        #[qproperty(QString, model)]
        #[qproperty(QString, system_prompt, cxx_name = "systemPrompt")]
        #[qproperty(QString, current_original, cxx_name = "currentOriginal")]
        #[qproperty(QString, current_translated, cxx_name = "currentTranslated")]
        #[qproperty(QString, messages_json, cxx_name = "messagesJson")]
        #[qproperty(bool, is_sending, cxx_name = "isSending")]
        #[qproperty(QString, error_message, cxx_name = "errorMessage")]
        type AiTutorBridge = super::AiTutorBridgeRust;
        #[qinvokable]
        #[cxx_name = "setSubtitleContext"]
        fn set_subtitle_context(
            self: Pin<&mut AiTutorBridge>,
            video_path: &QString,
            cue_index: i32,
            start_secs: f64,
            end_secs: f64,
            original_text: &QString,
            translated_text: &QString,
            previous_text: &QString,
            next_text: &QString,
        ) -> bool;
        #[qinvokable]
        #[cxx_name = "saveConfig"]
        fn save_config(self: Pin<&mut AiTutorBridge>) -> bool;
        #[qinvokable]
        #[cxx_name = "sendMessage"]
        fn send_message(self: Pin<&mut AiTutorBridge>, message: &QString) -> bool;
        #[qinvokable]
        fn poll(self: Pin<&mut AiTutorBridge>) -> bool;
        #[qinvokable]
        #[cxx_name = "clearConversation"]
        fn clear_conversation(self: Pin<&mut AiTutorBridge>) -> bool;
        #[qinvokable]
        #[cxx_name = "restoreDefaultPrompt"]
        fn restore_default_prompt(self: Pin<&mut AiTutorBridge>) -> bool;
    }
}

pub struct AiTutorBridgeRust {
    base_url: QString,
    api_key: QString,
    model: QString,
    system_prompt: QString,
    current_original: QString,
    current_translated: QString,
    messages_json: QString,
    is_sending: bool,
    error_message: QString,
    context: Option<SubtitleContext>,
    conversation: Option<Conversation>,
    result_receiver: Option<Receiver<Result<AiStreamEvent, String>>>,
    settings: els_storage::AiSettingsRepository,
}

impl Default for AiTutorBridgeRust {
    fn default() -> Self {
        let settings = els_storage::AiSettingsRepository::open_default()
            .unwrap_or_else(|_| els_storage::AiSettingsRepository::disabled());
        let config = settings.load().unwrap_or_else(|_| AiConfig {
            protocol: AiProtocol::OpenAiCompatible,
            base_url: String::new(),
            api_key: String::new(),
            model: String::new(),
            system_prompt: DEFAULT_SYSTEM_PROMPT.to_string(),
        });
        Self {
            base_url: QString::from(&config.base_url),
            api_key: QString::from(&config.api_key),
            model: QString::from(&config.model),
            system_prompt: QString::from(if config.system_prompt.is_empty() {
                DEFAULT_SYSTEM_PROMPT
            } else {
                &config.system_prompt
            }),
            current_original: QString::from(""),
            current_translated: QString::from(""),
            messages_json: QString::from("[]"),
            is_sending: false,
            error_message: QString::from(""),
            context: None,
            conversation: None,
            result_receiver: None,
            settings,
        }
    }
}

impl qobject::AiTutorBridge {
    fn set_subtitle_context(
        mut self: Pin<&mut Self>,
        video_path: &QString,
        cue_index: i32,
        start_secs: f64,
        end_secs: f64,
        original_text: &QString,
        translated_text: &QString,
        previous_text: &QString,
        next_text: &QString,
    ) -> bool {
        if cue_index < 0 || original_text.to_string().trim().is_empty() {
            self.as_mut().rust_mut().context = None;
            self.as_mut().rust_mut().conversation = None;
            self.as_mut().set_current_original(QString::from(""));
            self.as_mut().set_current_translated(QString::from(""));
            self.as_mut().refresh_messages();
            return true;
        }
        let context = SubtitleContext {
            video_path: video_path.to_string(),
            cue_index,
            start_secs,
            end_secs,
            original_text: original_text.to_string(),
            translated_text: translated_text.to_string(),
            previous_text: previous_text.to_string(),
            next_text: next_text.to_string(),
        };
        let changed = self
            .rust()
            .context
            .as_ref()
            .map(|old| (&old.video_path, old.cue_index))
            != Some((&context.video_path, context.cue_index));
        self.as_mut().rust_mut().context = Some(context.clone());
        self.as_mut()
            .set_current_original(QString::from(&context.original_text));
        self.as_mut()
            .set_current_translated(QString::from(&context.translated_text));
        if changed {
            let messages = self
                .rust()
                .settings
                .load_conversation(&context.video_path, context.cue_index)
                .unwrap_or_default();
            self.as_mut().rust_mut().conversation = Some(Conversation {
                subtitle_key: format!("{}:{}", context.video_path, context.cue_index),
                context,
                messages,
            });
            self.as_mut().refresh_messages();
        }
        true
    }

    fn save_config(mut self: Pin<&mut Self>) -> bool {
        let config = self.config();
        match self.as_mut().rust_mut().settings.save(&config) {
            Ok(()) => {
                self.as_mut().set_error(QString::from(""));
                true
            }
            Err(error) => {
                self.as_mut()
                    .set_error(QString::from(&format!("保存 AI 配置失败：{error}")));
                false
            }
        }
    }

    fn send_message(mut self: Pin<&mut Self>, message: &QString) -> bool {
        if self.rust().is_sending {
            return false;
        }
        let question = message.to_string().trim().to_string();
        if question.is_empty() {
            return false;
        }
        let config = self.config();
        if let Err(error) = config.validate() {
            self.as_mut().set_error(QString::from(&error.to_string()));
            return false;
        }
        let Some(context) = self.rust().context.clone() else {
            self.as_mut().set_error(QString::from("请先选择一条字幕"));
            return false;
        };
        let mut rust = self.as_mut().rust_mut();
        let conversation = rust
            .conversation
            .get_or_insert_with(|| Conversation::new(context.clone()));
        conversation.messages.push(ChatMessage {
            role: MessageRole::User,
            content: question,
        });
        let mut messages = vec![
            ChatMessage {
                role: MessageRole::System,
                content: config.system_prompt.clone(),
            },
            ChatMessage {
                role: MessageRole::User,
                content: format!(
                    "当前字幕：{}\n翻译：{}\n请围绕这条字幕回答。",
                    context.original_text, context.translated_text
                ),
            },
        ];
        messages.extend(conversation.messages.clone());
        conversation.messages.push(ChatMessage {
            role: MessageRole::Assistant,
            content: String::new(),
        });
        drop(rust);
        let (sender, receiver) = mpsc::channel();
        self.as_mut().rust_mut().result_receiver = Some(receiver);
        self.as_mut().set_is_sending(true);
        self.as_mut().set_error(QString::from(""));
        self.as_mut().refresh_messages();
        std::thread::spawn(move || {
            let result = OpenAiCompatibleProvider.stream_blocking(&config, &messages, |event| {
                let _ = sender.send(Ok(event));
            });
            if let Err(error) = result {
                let _ = sender.send(Err(error.to_string()));
            }
        });
        true
    }

    fn poll(mut self: Pin<&mut Self>) -> bool {
        let Some(receiver) = self.as_mut().rust_mut().result_receiver.take() else {
            return false;
        };
        match receiver.try_recv() {
            Ok(Ok(AiStreamEvent::Delta(delta))) => {
                if let Some(conversation) = self.as_mut().rust_mut().conversation.as_mut() {
                    if let Some(last) = conversation.messages.last_mut() {
                        last.content.push_str(&delta);
                    }
                }
                self.as_mut().refresh_messages();
                self.as_mut().rust_mut().result_receiver = Some(receiver);
                true
            }
            Ok(Ok(AiStreamEvent::Done)) => {
                self.as_mut().set_is_sending(false);
                self.as_mut().persist_conversation();
                true
            }
            Ok(Err(error)) => {
                self.as_mut().set_is_sending(false);
                self.as_mut().set_error(QString::from(&error));
                true
            }
            Err(mpsc::TryRecvError::Empty) => {
                self.as_mut().rust_mut().result_receiver = Some(receiver);
                false
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.as_mut().set_is_sending(false);
                self.as_mut().set_error(QString::from("AI 请求线程已断开"));
                false
            }
        }
    }

    fn clear_conversation(mut self: Pin<&mut Self>) -> bool {
        let context = self.rust().context.clone();
        if let Some(context) = context.as_ref() {
            if let Err(error) = self.as_mut().rust_mut().settings.delete_conversation(
                &context.video_path,
                context.cue_index,
            ) {
                self.as_mut()
                    .set_error(QString::from(&format!("清空对话失败：{error}")));
                return false;
            }
        }
        self.as_mut().rust_mut().conversation = context.map(Conversation::new);
        self.as_mut().refresh_messages();
        true
    }
    fn restore_default_prompt(mut self: Pin<&mut Self>) -> bool {
        self.as_mut()
            .set_system_prompt(QString::from(DEFAULT_SYSTEM_PROMPT));
        true
    }
}

impl qobject::AiTutorBridge {
    fn config(&self) -> AiConfig {
        AiConfig {
            protocol: AiProtocol::OpenAiCompatible,
            base_url: self.rust().base_url.to_string(),
            api_key: self.rust().api_key.to_string(),
            model: self.rust().model.to_string(),
            system_prompt: self.rust().system_prompt.to_string(),
        }
    }
    fn set_error(mut self: Pin<&mut Self>, message: QString) {
        self.as_mut().set_error_message(message);
    }
    fn refresh_messages(mut self: Pin<&mut Self>) {
        let messages = self
            .rust()
            .conversation
            .as_ref()
            .map(|conversation| conversation.messages.clone())
            .unwrap_or_default();
        let display_messages = messages
            .iter()
            .map(|message| DisplayChatMessage {
                role: match message.role {
                    MessageRole::User => "User",
                    MessageRole::Assistant => "Assistant",
                    MessageRole::System => "System",
                },
                content: message.content.clone(),
                rendered_content: render_markdown(&message.content),
            })
            .collect::<Vec<_>>();
        let json = serde_json::to_string(&display_messages).unwrap_or_else(|_| "[]".to_string());
        self.as_mut().set_messages_json(QString::from(&json));
    }
    fn persist_conversation(mut self: Pin<&mut Self>) -> bool {
        let Some(conversation) = self.rust().conversation.clone() else {
            return false;
        };
        match self.as_mut().rust_mut().settings.save_conversation(
            &conversation.context.video_path,
            conversation.context.cue_index,
            &conversation.messages,
        ) {
            Ok(()) => true,
            Err(error) => {
                self.as_mut()
                    .set_error(QString::from(&format!("保存对话失败：{error}")));
                false
            }
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DisplayChatMessage {
    role: &'static str,
    content: String,
    rendered_content: String,
}
