use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chat_core::{AiClient, ChatMessage, CoreError, ProviderConfig, ProviderId, ProviderSettings};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{env, net::SocketAddr, sync::Arc};
use thiserror::Error;
use tower_http::services::ServeDir;
use tracing::info;

#[derive(Clone)]
struct AppState {
    ai: AiClient,
    config: ProviderConfig,
}

fn config_from_environment() -> ProviderConfig {
    ProviderConfig {
        openai: ProviderSettings {
            api_key: env_key("OPENAI_API_KEY"),
            model: env_string("OPENAI_MODEL", "luna-5.6"),
        },
        gemini: ProviderSettings {
            api_key: env_key("GEMINI_API_KEY"),
            model: env_string("GEMINI_MODEL", "gemini-3.1-pro-preview"),
        },
        anthropic: ProviderSettings {
            api_key: env_key("ANTHROPIC_API_KEY"),
            model: env_string("ANTHROPIC_MODEL", "claude-haiku-4-5"),
        },
    }
}

fn env_key(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.trim().is_empty())
}

fn env_string(name: &str, default: &str) -> String {
    env::var(name).unwrap_or_else(|_| default.to_owned())
}

#[derive(Debug, Deserialize)]
struct ChatRequest {
    provider: String,
    messages: Vec<ChatMessage>,
    #[serde(default)]
    settings: Option<chat_core::AgentSettings>,
}

#[derive(Debug, Serialize)]
struct ProviderInfo {
    id: &'static str,
    name: &'static str,
    model: String,
    configured: bool,
}

#[derive(Debug, Error)]
enum AppError {
    #[error("{0}")]
    Core(#[from] CoreError),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match &self {
            Self::Core(CoreError::InvalidRequest(_)) | Self::Core(CoreError::MissingApiKey(_)) => {
                StatusCode::BAD_REQUEST
            }
            Self::Core(CoreError::Provider { .. })
            | Self::Core(CoreError::InvalidResponse { .. })
            | Self::Core(CoreError::Transport(_)) => StatusCode::BAD_GATEWAY,
        };
        (status, Json(json!({ "error": self.to_string() }))).into_response()
    }
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let config = config_from_environment();
    let state = Arc::new(AppState {
        ai: AiClient::new(Client::new(), config.clone()),
        config,
    });

    let app = Router::new()
        .route("/api/chat", post(chat))
        .route("/api/providers", get(providers))
        .fallback_service(ServeDir::new("static"))
        .with_state(state);

    let address = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!(%address, "API chat app listening");
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn providers(State(state): State<Arc<AppState>>) -> Json<Vec<ProviderInfo>> {
    Json(
        ProviderId::ALL
            .into_iter()
            .map(|provider| {
                let settings = state.config.settings(provider);
                ProviderInfo {
                    id: provider.as_str(),
                    name: provider.display_name(),
                    model: settings.model.clone(),
                    configured: settings.api_key.is_some(),
                }
            })
            .collect(),
    )
}

async fn chat(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ChatRequest>,
) -> Result<Json<chat_core::ChatResponse>, AppError> {
    let provider = request.provider.parse::<ProviderId>()?;
    let response = match request.settings {
        Some(settings) if settings.provider == provider => {
            state
                .ai
                .complete_agent(&settings, &request.messages)
                .await?
                .response
        }
        Some(_) => {
            return Err(AppError::Core(CoreError::InvalidRequest(
                "request provider and settings provider must match".into(),
            )))
        }
        None => state.ai.complete(provider, &request.messages).await?,
    };
    Ok(Json(response))
}
