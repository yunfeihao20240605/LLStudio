use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AiProtocol {
    OpenAiCompatible,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiConfig {
    pub protocol: AiProtocol,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub system_prompt: String,
}

impl AiConfig {
    pub fn validate(&self) -> Result<(), AiError> {
        if self.base_url.trim().is_empty() {
            return Err(AiError::Configuration("Base URL 不能为空".to_string()));
        }
        if self.api_key.trim().is_empty() {
            return Err(AiError::Configuration("API Key 不能为空".to_string()));
        }
        if self.model.trim().is_empty() {
            return Err(AiError::Configuration("模型名称不能为空".to_string()));
        }
        if self.system_prompt.trim().is_empty() {
            return Err(AiError::Configuration("系统提示词不能为空".to_string()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubtitleContext {
    pub video_path: String,
    pub cue_index: i32,
    pub start_secs: f64,
    pub end_secs: f64,
    pub original_text: String,
    pub translated_text: String,
    pub previous_text: String,
    pub next_text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
}

/// 将模型返回的 Markdown 转换为 Qt `Text.RichText` 可展示的安全 HTML。
/// 原始 Markdown 不在这里修改，调用方可以继续用它保存和发起下一轮请求。
pub fn render_markdown(markdown: &str) -> String {
    let mut html = String::new();
    let mut in_list = false;

    for line in markdown.lines() {
        let line = line.trim_end();
        if line.trim().is_empty() {
            if in_list {
                html.push_str("</ul>");
                in_list = false;
            }
            continue;
        }

        if let Some(content) = line.strip_prefix("### ") {
            close_list(&mut html, &mut in_list);
            html.push_str("<h3>");
            html.push_str(&render_inline(content));
            html.push_str("</h3>");
        } else if let Some(content) = line.strip_prefix("## ") {
            close_list(&mut html, &mut in_list);
            html.push_str("<h2>");
            html.push_str(&render_inline(content));
            html.push_str("</h2>");
        } else if let Some(content) = line.strip_prefix("# ") {
            close_list(&mut html, &mut in_list);
            html.push_str("<h1>");
            html.push_str(&render_inline(content));
            html.push_str("</h1>");
        } else if let Some(content) = line.strip_prefix("> ") {
            close_list(&mut html, &mut in_list);
            html.push_str("<blockquote>");
            html.push_str(&render_inline(content));
            html.push_str("</blockquote>");
        } else if let Some(content) = line
            .strip_prefix("- ")
            .or_else(|| line.strip_prefix("* "))
        {
            if !in_list {
                html.push_str("<ul>");
                in_list = true;
            }
            html.push_str("<li>");
            html.push_str(&render_inline(content));
            html.push_str("</li>");
        } else {
            close_list(&mut html, &mut in_list);
            html.push_str("<p>");
            html.push_str(&render_inline(line));
            html.push_str("</p>");
        }
    }
    close_list(&mut html, &mut in_list);
    html
}

fn close_list(html: &mut String, in_list: &mut bool) {
    if *in_list {
        html.push_str("</ul>");
        *in_list = false;
    }
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn render_inline(value: &str) -> String {
    let mut rendered = escape_html(value);
    rendered = replace_delimited(&rendered, "`", "<code>", "</code>");
    rendered = replace_delimited(&rendered, "**", "<strong>", "</strong>");
    rendered = replace_delimited(&rendered, "__", "<strong>", "</strong>");
    rendered = replace_delimited(&rendered, "*", "<em>", "</em>");
    rendered = replace_delimited(&rendered, "_", "<em>", "</em>");
    rendered.replace("  ", "<br/>")
}

fn replace_delimited(value: &str, delimiter: &str, open: &str, close: &str) -> String {
    let mut result = String::new();
    let mut remaining = value;
    let mut is_open = true;
    while let Some(index) = remaining.find(delimiter) {
        result.push_str(&remaining[..index]);
        result.push_str(if is_open { open } else { close });
        remaining = &remaining[index + delimiter.len()..];
        is_open = !is_open;
    }
    result.push_str(remaining);
    result
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Conversation {
    pub subtitle_key: String,
    pub context: SubtitleContext,
    pub messages: Vec<ChatMessage>,
}

impl Conversation {
    pub fn new(context: SubtitleContext) -> Self {
        let subtitle_key = format!("{}:{}", context.video_path, context.cue_index);
        Self {
            subtitle_key,
            context,
            messages: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiResponse {
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiStreamEvent {
    Delta(String),
    Done,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiError {
    Configuration(String),
    Transport(String),
    Protocol(String),
    Cancelled,
}

impl std::fmt::Display for AiError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Configuration(message) | Self::Transport(message) | Self::Protocol(message) => {
                formatter.write_str(message)
            }
            Self::Cancelled => formatter.write_str("AI 请求已取消"),
        }
    }
}

impl std::error::Error for AiError {}

pub trait AiProvider {
    fn send(&self, config: &AiConfig, messages: &[ChatMessage]) -> Result<AiResponse, AiError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenAiCompatibleProvider;

impl OpenAiCompatibleProvider {
    pub fn endpoint(base_url: &str) -> String {
        let trimmed = base_url.trim().trim_end_matches('/');
        if trimmed.ends_with("/chat/completions") {
            trimmed.to_string()
        } else {
            format!("{trimmed}/chat/completions")
        }
    }

    pub fn request_body(config: &AiConfig, messages: &[ChatMessage]) -> Result<String, AiError> {
        config.validate()?;
        let payload = serde_json::json!({
            "model": config.model.trim(),
            "messages": messages.iter().map(|message| serde_json::json!({
                "role": match message.role {
                    MessageRole::System => "system",
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                },
                "content": message.content,
            })).collect::<Vec<_>>(),
            "stream": false,
        });
        serde_json::to_string(&payload)
            .map_err(|error| AiError::Protocol(format!("生成 AI 请求失败：{error}")))
    }

    pub fn parse_response(body: &str) -> Result<AiResponse, AiError> {
        let value: serde_json::Value = serde_json::from_str(body)
            .map_err(|error| AiError::Protocol(format!("解析 AI 响应失败：{error}")))?;
        let content = value["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| AiError::Protocol("AI 响应缺少 choices.message.content".to_string()))?;
        Ok(AiResponse {
            content: content.to_string(),
        })
    }

    pub fn send_blocking(
        &self,
        config: &AiConfig,
        messages: &[ChatMessage],
    ) -> Result<AiResponse, AiError> {
        use std::process::Command;

        let body = Self::request_body(config, messages)?;
        let output = Command::new("curl")
            .args([
                "--silent",
                "--show-error",
                "--fail-with-body",
                "--max-time",
                "120",
            ])
            .arg(Self::endpoint(&config.base_url))
            .args(["-H", "Content-Type: application/json"])
            .arg("-H")
            .arg(format!("Authorization: Bearer {}", config.api_key.trim()))
            .args(["--data-raw", &body])
            .output()
            .map_err(|error| AiError::Transport(format!("启动 curl 失败：{error}")))?;
        if !output.status.success() {
            let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let body = String::from_utf8_lossy(&output.stdout).trim().to_string();
            return Err(AiError::Transport(if !body.is_empty() {
                body
            } else {
                detail
            }));
        }
        Self::parse_response(&String::from_utf8_lossy(&output.stdout))
    }

    pub fn stream_blocking<F>(
        &self,
        config: &AiConfig,
        messages: &[ChatMessage],
        mut on_event: F,
    ) -> Result<(), AiError>
    where
        F: FnMut(AiStreamEvent),
    {
        use std::io::{BufRead, BufReader};
        use std::process::{Command, Stdio};
        let body = Self::stream_request_body(config, messages)?;
        let mut child = Command::new("curl")
            .args([
                "--silent",
                "--show-error",
                "--fail-with-body",
                "--no-buffer",
                "--max-time",
                "120",
            ])
            .arg(Self::endpoint(&config.base_url))
            .args(["-H", "Content-Type: application/json"])
            .arg("-H")
            .arg(format!("Authorization: Bearer {}", config.api_key.trim()))
            .args(["--data-raw", &body])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| AiError::Transport(format!("启动 curl 失败：{error}")))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AiError::Transport("读取 AI 流失败".to_string()))?;
        let mut done_received = false;
        for line in BufReader::new(stdout).lines() {
            let line =
                line.map_err(|error| AiError::Transport(format!("读取 AI 流失败：{error}")))?;
            let Some(data) = line.strip_prefix("data:").map(str::trim) else {
                continue;
            };
            if data == "[DONE]" {
                on_event(AiStreamEvent::Done);
                done_received = true;
                break;
            }
            let value: serde_json::Value = serde_json::from_str(data)
                .map_err(|error| AiError::Protocol(format!("解析 AI 流失败：{error}")))?;
            if let Some(content) = value["choices"][0]["delta"]["content"].as_str() {
                if !content.is_empty() {
                    on_event(AiStreamEvent::Delta(content.to_string()));
                }
            }
        }
        let status = child
            .wait()
            .map_err(|error| AiError::Transport(format!("等待 AI 请求失败：{error}")))?;
        if !status.success() {
            return Err(AiError::Transport("AI 服务返回失败状态".to_string()));
        }
        if !done_received {
            on_event(AiStreamEvent::Done);
        }
        Ok(())
    }

    fn stream_request_body(config: &AiConfig, messages: &[ChatMessage]) -> Result<String, AiError> {
        config.validate()?;
        let mut value: serde_json::Value =
            serde_json::from_str(&Self::request_body(config, messages)?)
                .map_err(|error| AiError::Protocol(format!("生成 AI 请求失败：{error}")))?;
        value["stream"] = serde_json::Value::Bool(true);
        serde_json::to_string(&value)
            .map_err(|error| AiError::Protocol(format!("生成 AI 请求失败：{error}")))
    }
}

impl AiProvider for OpenAiCompatibleProvider {
    fn send(&self, config: &AiConfig, messages: &[ChatMessage]) -> Result<AiResponse, AiError> {
        self.send_blocking(config, messages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> AiConfig {
        AiConfig {
            protocol: AiProtocol::OpenAiCompatible,
            base_url: "https://example.test/v1".into(),
            api_key: "key".into(),
            model: "demo".into(),
            system_prompt: "coach".into(),
        }
    }

    #[test]
    fn builds_openai_compatible_endpoint_and_body() {
        assert_eq!(
            OpenAiCompatibleProvider::endpoint("https://example.test/v1/"),
            "https://example.test/v1/chat/completions"
        );
        let body = OpenAiCompatibleProvider::request_body(
            &config(),
            &[ChatMessage {
                role: MessageRole::User,
                content: "怎么读？".into(),
            }],
        )
        .unwrap();
        assert!(body.contains("\"model\":\"demo\""));
        assert!(body.contains("\"role\":\"user\""));
    }

    #[test]
    fn parses_openai_compatible_response() {
        let response = OpenAiCompatibleProvider::parse_response(
            r#"{"choices":[{"message":{"content":"Try this."}}]}"#,
        )
        .unwrap();
        assert_eq!(response.content, "Try this.");
    }

    #[test]
    fn renders_markdown_as_safe_rich_text() {
        let html = render_markdown("> **Human** `are`\n\n- link <script>");
        assert!(html.contains("<blockquote><strong>Human</strong> <code>are</code></blockquote>"));
        assert!(html.contains("&lt;script&gt;"));
        assert!(!html.contains("<script>"));
    }
}
