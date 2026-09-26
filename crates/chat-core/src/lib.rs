mod anthropic;
mod gemini;
mod openai;

use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use thiserror::Error;

pub const MAX_MESSAGES: usize = 100;
pub const MAX_MESSAGE_CHARS: usize = 32_000;
pub const DEFAULT_CONTEXT_MAX_MESSAGES: usize = 40;
pub const DEFAULT_CONTEXT_MAX_CHARS: usize = 24_000;
pub const DEFAULT_CONTEXT_PRESERVE_RECENT: usize = 12;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderId {
    #[serde(rename = "openai")]
    OpenAi,
    Gemini,
    #[serde(rename = "anthropic")]
    Anthropic,
}

impl ProviderId {
    pub const ALL: [Self; 3] = [Self::OpenAi, Self::Gemini, Self::Anthropic];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenAi => "openai",
            Self::Gemini => "gemini",
            Self::Anthropic => "anthropic",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::OpenAi => "OpenAI",
            Self::Gemini => "Gemini",
            Self::Anthropic => "Claude",
        }
    }
}

impl fmt::Display for ProviderId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for ProviderId {
    type Err = CoreError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "openai" => Ok(Self::OpenAi),
            "gemini" => Ok(Self::Gemini),
            "anthropic" | "claude" => Ok(Self::Anthropic),
            other => Err(CoreError::InvalidRequest(format!(
                "unsupported provider: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSettings {
    pub max_messages: usize,
    pub max_chars: usize,
    pub preserve_recent_messages: usize,
}

impl Default for ContextSettings {
    fn default() -> Self {
        Self {
            max_messages: DEFAULT_CONTEXT_MAX_MESSAGES,
            max_chars: DEFAULT_CONTEXT_MAX_CHARS,
            preserve_recent_messages: DEFAULT_CONTEXT_PRESERVE_RECENT,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSettings {
    pub provider: ProviderId,
    pub temperature: f32,
    pub max_output_tokens: u32,
    pub context: ContextSettings,
    pub system_prompt: String,
}

impl Default for AgentSettings {
    fn default() -> Self {
        Self {
            provider: ProviderId::OpenAi,
            temperature: 0.7,
            max_output_tokens: 1024,
            context: ContextSettings::default(),
            system_prompt: String::new(),
        }
    }
}

impl AgentSettings {
    fn validate(&self) -> Result<(), CoreError> {
        if !(0.0..=2.0).contains(&self.temperature) {
            return Err(CoreError::InvalidRequest(
                "temperature must be between 0 and 2".into(),
            ));
        }
        if self.max_output_tokens == 0 {
            return Err(CoreError::InvalidRequest(
                "max_output_tokens must be greater than zero".into(),
            ));
        }
        if self.context.max_messages == 0
            || self.context.max_chars == 0
            || self.context.preserve_recent_messages == 0
        {
            return Err(CoreError::InvalidRequest(
                "context limits must be greater than zero".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CompletionSettings {
    pub temperature: f32,
    pub max_output_tokens: u32,
}

impl Default for CompletionSettings {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            max_output_tokens: 1024,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompactionResult {
    pub messages: Vec<ChatMessage>,
    pub summary: Option<String>,
}

impl CompactionResult {
    pub fn compacted(&self) -> bool {
        self.summary.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct AgentResponse {
    pub response: ChatResponse,
    pub messages: Vec<ChatMessage>,
    pub compacted: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ProviderConfig {
    pub openai: ProviderSettings,
    pub gemini: ProviderSettings,
    pub anthropic: ProviderSettings,
}

#[derive(Debug, Clone, Default)]
pub struct ProviderSettings {
    pub api_key: Option<String>,
    pub model: String,
}

impl ProviderConfig {
    pub fn settings(&self, provider: ProviderId) -> &ProviderSettings {
        match provider {
            ProviderId::OpenAi => &self.openai,
            ProviderId::Gemini => &self.gemini,
            ProviderId::Anthropic => &self.anthropic,
        }
    }

    pub fn settings_mut(&mut self, provider: ProviderId) -> &mut ProviderSettings {
        match provider {
            ProviderId::OpenAi => &mut self.openai,
            ProviderId::Gemini => &mut self.gemini,
            ProviderId::Anthropic => &mut self.anthropic,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelInfo {
    pub id: String,
    pub display_name: String,
    pub provider: ProviderId,
    pub context_window: Option<u32>,
    pub supports_vision: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatResponse {
    pub content: String,
    pub provider: ProviderId,
    pub model: String,
}

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("{0} API key is not configured")]
    MissingApiKey(ProviderId),
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("{provider} returned HTTP {status}: {message}")]
    Provider {
        provider: ProviderId,
        status: reqwest::StatusCode,
        message: String,
    },
    #[error("invalid {provider} response: {message}")]
    InvalidResponse {
        provider: ProviderId,
        message: String,
    },
    #[error("provider request failed: {0}")]
    Transport(#[from] reqwest::Error),
}

#[derive(Clone)]
pub struct AiClient {
    http: Client,
    config: ProviderConfig,
}

impl AiClient {
    pub fn new(http: Client, config: ProviderConfig) -> Self {
        Self { http, config }
    }

    pub async fn complete(
        &self,
        provider: ProviderId,
        messages: &[ChatMessage],
    ) -> Result<ChatResponse, CoreError> {
        self.complete_with_options(provider, messages, CompletionSettings::default())
            .await
    }

    pub async fn complete_with_options(
        &self,
        provider: ProviderId,
        messages: &[ChatMessage],
        options: CompletionSettings,
    ) -> Result<ChatResponse, CoreError> {
        validate_messages(messages)?;
        validate_model_id(&self.config.settings(provider).model)?;

        match provider {
            ProviderId::OpenAi => {
                openai::complete(&self.http, &self.config.openai, messages, options).await
            }
            ProviderId::Gemini => {
                gemini::complete(&self.http, &self.config.gemini, messages, options).await
            }
            ProviderId::Anthropic => {
                anthropic::complete(&self.http, &self.config.anthropic, messages, options).await
            }
        }
    }

    pub async fn list_models(&self, provider: ProviderId) -> Result<Vec<ModelInfo>, CoreError> {
        match provider {
            ProviderId::OpenAi => openai::list_models(&self.http, &self.config.openai).await,
            ProviderId::Gemini => gemini::list_models(&self.http, &self.config.gemini).await,
            ProviderId::Anthropic => {
                anthropic::list_models(&self.http, &self.config.anthropic).await
            }
        }
    }

    pub async fn complete_agent(
        &self,
        settings: &AgentSettings,
        messages: &[ChatMessage],
    ) -> Result<AgentResponse, CoreError> {
        settings.validate()?;
        let mut request_messages = messages.to_vec();
        if !settings.system_prompt.trim().is_empty() {
            request_messages.insert(
                0,
                ChatMessage {
                    role: Role::System,
                    content: settings.system_prompt.trim().to_owned(),
                },
            );
        }

        let compaction = self
            .compact_if_needed(settings.provider, &request_messages, &settings.context)
            .await?;
        let response = self
            .complete_with_options(
                settings.provider,
                &compaction.messages,
                CompletionSettings {
                    temperature: settings.temperature,
                    max_output_tokens: settings.max_output_tokens,
                },
            )
            .await?;

        let compacted = compaction.compacted();
        Ok(AgentResponse {
            response,
            messages: compaction.messages,
            compacted,
        })
    }

    pub async fn compact_if_needed(
        &self,
        provider: ProviderId,
        messages: &[ChatMessage],
        settings: &ContextSettings,
    ) -> Result<CompactionResult, CoreError> {
        validate_messages(messages)?;
        if !needs_compaction(messages, settings) {
            return Ok(CompactionResult {
                messages: messages.to_vec(),
                summary: None,
            });
        }

        let non_system_count = messages
            .iter()
            .filter(|message| message.role != Role::System)
            .count();
        let preserve_count = settings
            .preserve_recent_messages
            .min(non_system_count.saturating_sub(1));
        let split_at = non_system_count.saturating_sub(preserve_count);
        let mut older = Vec::new();
        let mut recent = Vec::new();
        let mut seen_non_system = 0;
        for message in messages {
            if message.role == Role::System {
                continue;
            }
            if seen_non_system < split_at {
                older.push(message);
            } else {
                recent.push(message.clone());
            }
            seen_non_system += 1;
        }

        if older.is_empty() {
            return Ok(CompactionResult {
                messages: messages.to_vec(),
                summary: None,
            });
        }

        let transcript = older
            .iter()
            .map(|message| format!("{:?}: {}", message.role, message.content))
            .collect::<Vec<_>>()
            .join("\n\n");
        let summary_request = vec![
            ChatMessage {
                role: Role::System,
                content: "Summarize the earlier conversation for another AI assistant. Preserve decisions, facts, constraints, unresolved questions, user preferences, and important technical details. Be concise and do not invent information.".into(),
            },
            ChatMessage {
                role: Role::User,
                content: format!(
                    "Earlier conversation to compact:\n\n{}",
                    truncate(&transcript)
                ),
            },
        ];
        let summary_response = self
            .complete_with_options(
                provider,
                &summary_request,
                CompletionSettings {
                    temperature: 0.2,
                    max_output_tokens: 1024,
                },
            )
            .await?;

        let mut compacted = messages
            .iter()
            .filter(|message| message.role == Role::System)
            .cloned()
            .collect::<Vec<_>>();
        compacted.push(ChatMessage {
            role: Role::System,
            content: format!(
                "Earlier conversation summary (automatically compacted):\n{}",
                summary_response.content
            ),
        });
        compacted.extend(recent);

        Ok(CompactionResult {
            messages: compacted,
            summary: Some(summary_response.content),
        })
    }
}

fn needs_compaction(messages: &[ChatMessage], settings: &ContextSettings) -> bool {
    messages.len() > settings.max_messages
        || messages
            .iter()
            .map(|message| message.content.chars().count())
            .sum::<usize>()
            > settings.max_chars
}

fn validate_model_id(model: &str) -> Result<(), CoreError> {
    if model.is_empty() || model.chars().count() > 256 {
        return Err(CoreError::InvalidRequest(
            "model ID must contain between 1 and 256 characters".into(),
        ));
    }
    if !model.chars().all(|character| {
        character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | ':' | '/')
    }) {
        return Err(CoreError::InvalidRequest(
            "model ID contains unsupported characters".into(),
        ));
    }
    Ok(())
}

fn validate_messages(messages: &[ChatMessage]) -> Result<(), CoreError> {
    if messages.is_empty() {
        return Err(CoreError::InvalidRequest(
            "at least one message is required".into(),
        ));
    }
    if messages.len() > MAX_MESSAGES {
        return Err(CoreError::InvalidRequest(format!(
            "at most {MAX_MESSAGES} messages are allowed"
        )));
    }
    if messages
        .iter()
        .any(|message| message.content.trim().is_empty())
    {
        return Err(CoreError::InvalidRequest(
            "message content cannot be empty".into(),
        ));
    }
    if messages
        .iter()
        .any(|message| message.content.chars().count() > MAX_MESSAGE_CHARS)
    {
        return Err(CoreError::InvalidRequest(format!(
            "messages cannot exceed {MAX_MESSAGE_CHARS} characters"
        )));
    }
    Ok(())
}

async fn json_response(
    response: Response,
    provider: ProviderId,
) -> Result<serde_json::Value, CoreError> {
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        return Err(CoreError::Provider {
            provider,
            status,
            message: truncate(&body),
        });
    }

    serde_json::from_str(&body).map_err(|error| CoreError::InvalidResponse {
        provider,
        message: error.to_string(),
    })
}

fn truncate(value: &str) -> String {
    const MAX_ERROR_CHARS: usize = 4_000;
    value.chars().take(MAX_ERROR_CHARS).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_ids_are_restricted_to_provider_safe_characters() {
        assert!(validate_model_id("luna-5.6").is_ok());
        assert!(validate_model_id("provider/model_name:v1").is_ok());
        assert!(validate_model_id("models/gemini-3.1-pro-preview").is_ok());
        assert!(validate_model_id("models/foo?key=secret").is_err());
        assert!(validate_model_id("model with spaces").is_err());
    }

    #[test]
    fn default_agent_settings_are_valid() {
        assert!(AgentSettings::default().validate().is_ok());
    }

    #[test]
    fn invalid_agent_settings_are_rejected() {
        let mut settings = AgentSettings::default();
        settings.temperature = 2.1;
        assert!(settings.validate().is_err());

        settings.temperature = 0.7;
        settings.context.max_chars = 0;
        assert!(settings.validate().is_err());
    }

    #[test]
    fn compaction_threshold_uses_message_count_or_character_count() {
        let messages = vec![
            ChatMessage {
                role: Role::User,
                content: "hello".into(),
            },
            ChatMessage {
                role: Role::Assistant,
                content: "world".into(),
            },
        ];
        let mut settings = ContextSettings::default();
        assert!(!needs_compaction(&messages, &settings));

        settings.max_messages = 1;
        assert!(needs_compaction(&messages, &settings));

        settings.max_messages = 40;
        settings.max_chars = 9;
        assert!(needs_compaction(&messages, &settings));
    }
}
