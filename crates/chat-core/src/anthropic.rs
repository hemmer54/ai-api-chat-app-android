use crate::{
    json_response, ChatMessage, ChatResponse, CompletionSettings, CoreError, ModelInfo, ProviderId,
    ProviderSettings, Role,
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
        .ok_or(CoreError::MissingApiKey(ProviderId::Anthropic))?;
    let mut before_id: Option<String> = None;
    let mut result = Vec::new();

    loop {
        let mut request = http
            .get("https://api.anthropic.com/v1/models")
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01")
            .query(&[("limit", "1000")]);
        if let Some(id) = before_id.as_deref() {
            request = request.query(&[("before_id", id)]);
        }
        let response = request.send().await?;
        let body = json_response(response, ProviderId::Anthropic).await?;
        let page = body["data"]
            .as_array()
            .ok_or_else(|| CoreError::InvalidResponse {
                provider: ProviderId::Anthropic,
                message: "model list response did not contain a data array".into(),
            })?;

        result.extend(page.iter().filter_map(|model| {
            let id = model["id"].as_str()?.to_owned();
            Some(ModelInfo {
                display_name: model["display_name"].as_str().unwrap_or(&id).to_owned(),
                id,
                provider: ProviderId::Anthropic,
                context_window: model["context_window"]
                    .as_u64()
                    .and_then(|value| value.try_into().ok()),
                supports_vision: true,
            })
        }));
        if body["has_more"].as_bool() != Some(true) {
            break;
        }
        before_id = page
            .last()
            .and_then(|model| model["id"].as_str())
            .map(str::to_owned);
        if before_id.is_none() {
            break;
        }
    }

    result.sort_by(|left, right| left.id.cmp(&right.id));
    result.dedup_by(|left, right| left.id == right.id);
    Ok(result)
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
        .ok_or(CoreError::MissingApiKey(ProviderId::Anthropic))?;
    let model = settings.model.clone();
    let system = messages
        .iter()
        .filter(|message| message.role == Role::System)
        .map(|message| message.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    let conversation = messages
        .iter()
        .filter(|message| message.role != Role::System)
        .collect::<Vec<_>>();

    if conversation.is_empty() {
        return Err(CoreError::InvalidRequest(
            "Claude requires at least one user or assistant message".into(),
        ));
    }

    let mut payload = json!({
        "model": model,
        "max_tokens": options.max_output_tokens,
        "messages": conversation,
        "temperature": options.temperature
    });
    if !system.is_empty() {
        payload["system"] = json!(system);
    }

    let response = http
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", key)
        .header("anthropic-version", "2023-06-01")
        .json(&payload)
        .send()
        .await?;
    let body = json_response(response, ProviderId::Anthropic).await?;
    let content =
        body["content"][0]["text"]
            .as_str()
            .ok_or_else(|| CoreError::InvalidResponse {
                provider: ProviderId::Anthropic,
                message: "response did not contain assistant content".into(),
            })?;

    Ok(ChatResponse {
        content: content.to_owned(),
        provider: ProviderId::Anthropic,
        model,
    })
}
