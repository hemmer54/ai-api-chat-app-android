use crate::{
    json_response, ChatMessage, ChatResponse, CompletionSettings, CoreError, ModelInfo, ProviderId,
    ProviderSettings, Role,
};
use reqwest::Client;
use serde_json::{json, Value};

pub async fn list_models(
    http: &Client,
    settings: &ProviderSettings,
) -> Result<Vec<ModelInfo>, CoreError> {
    let key = settings
        .api_key
        .as_deref()
        .ok_or(CoreError::MissingApiKey(ProviderId::Gemini))?;
    let mut page_token: Option<String> = None;
    let mut models = Vec::new();

    loop {
        let mut request = http
            .get("https://generativelanguage.googleapis.com/v1beta/models")
            .header("x-goog-api-key", key)
            .query(&[("pageSize", "1000")]);
        if let Some(token) = page_token.as_deref() {
            request = request.query(&[("pageToken", token)]);
        }
        let response = request.send().await?;
        let body = json_response(response, ProviderId::Gemini).await?;
        if let Some(model_values) = body["models"].as_array() {
            for model in model_values {
                let supports_generate = model["supportedGenerationMethods"]
                    .as_array()
                    .map(|methods| {
                        methods
                            .iter()
                            .any(|method| method.as_str() == Some("generateContent"))
                    })
                    .unwrap_or(false);
                if !supports_generate {
                    continue;
                }
                if let Some(name) = model["name"].as_str() {
                    let id = name.strip_prefix("models/").unwrap_or(name).to_owned();
                    models.push(ModelInfo {
                        display_name: model["displayName"].as_str().unwrap_or(&id).to_owned(),
                        id,
                        provider: ProviderId::Gemini,
                        context_window: model["inputTokenLimit"]
                            .as_u64()
                            .and_then(|value| value.try_into().ok()),
                        supports_vision: model["supportedGenerationMethods"]
                            .as_array()
                            .map(|methods| {
                                methods
                                    .iter()
                                    .any(|method| method.as_str() == Some("generateContent"))
                            })
                            .unwrap_or(false),
                    });
                }
            }
        }
        page_token = body["nextPageToken"].as_str().map(str::to_owned);
        if page_token.is_none() {
            break;
        }
    }

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
        .ok_or(CoreError::MissingApiKey(ProviderId::Gemini))?;
    let model = settings.model.clone();

    let mut system_parts = Vec::new();
    let contents: Vec<Value> = messages
        .iter()
        .filter_map(|message| match message.role {
            Role::System => {
                system_parts.push(message.content.as_str());
                None
            }
            Role::User | Role::Assistant => Some(json!({
                "role": if message.role == Role::Assistant { "model" } else { "user" },
                "parts": [{ "text": message.content }]
            })),
        })
        .collect();

    if contents.is_empty() {
        return Err(CoreError::InvalidRequest(
            "Gemini requires at least one user or assistant message".into(),
        ));
    }

    let mut payload = json!({
        "contents": contents,
        "generationConfig": {
            "temperature": options.temperature,
            "maxOutputTokens": options.max_output_tokens
        }
    });
    if !system_parts.is_empty() {
        payload["systemInstruction"] = json!({
            "parts": [{ "text": system_parts.join("\n\n") }]
        });
    }

    let url =
        format!("https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent");
    let response = http
        .post(url)
        .header("x-goog-api-key", key)
        .json(&payload)
        .send()
        .await?;
    let body = json_response(response, ProviderId::Gemini).await?;
    let content = body["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .ok_or_else(|| CoreError::InvalidResponse {
            provider: ProviderId::Gemini,
            message: "response did not contain generated text".into(),
        })?;

    Ok(ChatResponse {
        content: content.to_owned(),
        provider: ProviderId::Gemini,
        model,
    })
}
