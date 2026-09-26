use crate::{
    json_response, ChatMessage, ChatResponse, CompletionSettings, CoreError, ModelInfo, ProviderId,
    ProviderSettings,
};
use reqwest::Client;
use serde_json::json;

pub async fn list_models(
    http: &Client,
    settings: &ProviderSettings,
) -> Result<Vec<ModelInfo>, CoreError> {
    let key = settings
        .api_key
        .as_deref()
        .ok_or(CoreError::MissingApiKey(ProviderId::OpenAi))?;
    let response = http
        .get("https://api.openai.com/v1/models?limit=1000")
        .bearer_auth(key)
        .send()
        .await?;
    let body = json_response(response, ProviderId::OpenAi).await?;
    let models = body["data"]
        .as_array()
        .ok_or_else(|| CoreError::InvalidResponse {
            provider: ProviderId::OpenAi,
            message: "model list response did not contain a data array".into(),
        })?;

    let mut models = models
        .iter()
        .filter_map(|model| {
            let id = model["id"].as_str()?;
            let lower = id.to_ascii_lowercase();
            if !(lower.starts_with("gpt-")
                || lower.starts_with("luna-")
                || lower.starts_with("o1")
                || lower.starts_with("o3")
                || lower.starts_with("o4")
                || lower.starts_with("chatgpt-"))
            {
                return None;
            }
            Some(ModelInfo {
                id: id.to_owned(),
                display_name: id.to_owned(),
                provider: ProviderId::OpenAi,
                context_window: model["context_window"]
                    .as_u64()
                    .and_then(|value| value.try_into().ok()),
                supports_vision: lower.contains("vision") || lower.contains("gpt-4o"),
            })
        })
        .collect::<Vec<_>>();
    models.sort_by(|left, right| left.id.cmp(&right.id));
    models.dedup_by(|left, right| left.id == right.id);
    Ok(models)
}

pub async fn complete(
    http: &Client,
    settings: &ProviderSettings,
    messages: &[ChatMessage],
    options: CompletionSettings,
) -> Result<ChatResponse, CoreError> {
    let key = settings
        .api_key
        .as_deref()
        .ok_or(CoreError::MissingApiKey(ProviderId::OpenAi))?;
    let model = settings.model.clone();

    let response = http
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(key)
        .json(&json!({
            "model": model,
            "messages": messages,
            "stream": false,
            "temperature": options.temperature,
            "max_tokens": options.max_output_tokens
        }))
        .send()
        .await?;
    let body = json_response(response, ProviderId::OpenAi).await?;
    let content = body["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| CoreError::InvalidResponse {
            provider: ProviderId::OpenAi,
            message: "response did not contain assistant content".into(),
        })?;

    Ok(ChatResponse {
        content: content.to_owned(),
        provider: ProviderId::OpenAi,
        model,
    })
}
