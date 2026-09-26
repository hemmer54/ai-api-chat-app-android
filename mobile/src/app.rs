use chat_core::{
    AgentSettings, AiClient, ChatMessage, ModelInfo, ProviderConfig, ProviderId, ProviderSettings,
    Role,
};
#[cfg(target_os = "android")]
use gpui::App;
use gpui::{div, prelude::*, px, rgb, Context, MouseButton, Render, Window};
use gpui_mobile::components::material::{MaterialTheme, TextField};
use gpui_mobile::{set_system_chrome, StatusBarContentStyle, SystemChromeStyle};
#[cfg(target_os = "android")]
use jni::objects::JValue;
#[cfg(target_os = "android")]
use std::borrow::Cow;
use std::cell::RefCell;

#[cfg(target_os = "android")]
const CHAT_FONT: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../FSEX302.ttf"));

#[cfg(target_os = "android")]
const MISC_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../jh_fallout-webfont.ttf"
));

use crate::{chat_list_screen, chat_screen, secure_keys, settings_screen};

const AGENT_MD: &str = include_str!("../../agent.md");

pub(crate) const MISC_FONT_FAMILY: &str = "JH_Fallout";
pub(crate) const CHAT_FONT_FAMILY: &str = "Fixedsys Excelsior";

pub(crate) fn midnight_theme() -> MaterialTheme {
    let mut theme = MaterialTheme::from_appearance(true);
    theme.surface = 0x10100F;
    theme.surface_container_low = 0x171614;
    theme.surface_container = 0x211F1B;
    theme.primary = 0x4F8A5B;
    theme.on_primary = 0x08110B;
    theme.primary_container = 0x1E3A28;
    theme.on_primary_container = 0xD5E8D5;
    theme.on_surface = 0xE6E1D8;
    theme.on_surface_variant = 0xA39C91;
    theme.outline = 0x4B6250;
    theme.outline_variant = 0x2B3A2E;
    theme.tertiary = 0xB6A36A;
    theme.error = 0xE07A7A;
    theme
}

pub(crate) fn default_model(provider: ProviderId) -> &'static str {
    match provider {
        ProviderId::OpenAi => "luna-5.6",
        ProviderId::Gemini => "gemini-3.1-pro-preview",
        ProviderId::Anthropic => "claude-haiku-4-5",
    }
}

fn newest_model(provider: ProviderId, models: &[ModelInfo]) -> Option<String> {
    let candidates = models.iter().filter(|model| model.provider == provider);
    let preferred = candidates
        .clone()
        .find(|model| model.id.eq_ignore_ascii_case(default_model(provider)));
    if let Some(model) = preferred {
        return Some(model.id.clone());
    }

    if provider == ProviderId::Anthropic && default_model(provider).contains("haiku") {
        if let Some(model) = candidates
            .clone()
            .filter(|model| model.id.to_ascii_lowercase().contains("haiku"))
            .max_by_key(|model| model.id.clone())
        {
            return Some(model.id.clone());
        }
    }

    // Luna is a user-selected OpenAI-compatible default. Keep it as the
    // default when the provider does not advertise it in its model list.
    if provider == ProviderId::OpenAi {
        return None;
    }

    candidates
        .max_by_key(|model| {
            let id = model.id.to_ascii_lowercase();
            let family_rank = match provider {
                ProviderId::OpenAi if id.starts_with("gpt-") => 500,
                ProviderId::OpenAi if id.starts_with("o4") => 400,
                ProviderId::OpenAi if id.starts_with("o3") => 300,
                ProviderId::Gemini if id.contains("pro") => 300,
                ProviderId::Gemini if id.contains("flash") => 200,
                ProviderId::Anthropic if id.contains("opus") => 300,
                ProviderId::Anthropic if id.contains("sonnet") => 200,
                _ => 100,
            };
            let version_rank = id
                .split(|character: char| !character.is_ascii_digit())
                .filter_map(|part| part.parse::<u32>().ok())
                .fold(0u32, |score, number| {
                    score.saturating_mul(100).saturating_add(number)
                });
            (family_rank, version_rank, id)
        })
        .map(|model| model.id.clone())
}

const MAX_PENDING_TEXT_EVENTS: usize = 128;
const MAX_MESSAGE_INPUT_CHARS: usize = 32_000;
const MAX_API_KEY_INPUT_CHARS: usize = 512;
const MAX_MODEL_INPUT_CHARS: usize = 256;

thread_local! {
    static PENDING_TEXT: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static FOCUS_REQUESTED: RefCell<Option<InputTarget>> = const { RefCell::new(None) };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InputTarget {
    None,
    Message,
    ApiKey,
    Model,
    ModelSearch,
}

pub(crate) fn install_keyboard_callback() {
    gpui_mobile::set_text_input_callback(Some(Box::new(|text: &str| {
        PENDING_TEXT.with(|pending| {
            if pending.borrow().len() < MAX_PENDING_TEXT_EVENTS {
                pending.borrow_mut().push(text.to_owned());
            }
        });
    })));
    gpui_mobile::TEXT_INPUT_DIRTY.store(true, std::sync::atomic::Ordering::Release);
}

fn clear_pending_text() {
    PENDING_TEXT.with(|pending| pending.borrow_mut().clear());
}

pub(crate) fn request_focus(target: InputTarget) {
    clear_pending_text();
    FOCUS_REQUESTED.with(|focused| *focused.borrow_mut() = Some(target));
}

#[derive(Debug, Clone)]
pub(crate) struct Conversation {
    pub id: u64,
    pub title: String,
    pub preview: String,
    pub messages: Vec<ChatMessage>,
}

pub struct ChatApp {
    pub(crate) conversations: Vec<Conversation>,
    pub(crate) active_conversation: Option<u64>,
    next_conversation_id: u64,
    pub(crate) messages: Vec<ChatMessage>,
    pub(crate) draft: TextField,
    pub(crate) api_key_field: TextField,
    pub(crate) model_field: TextField,
    pub(crate) model_search_field: TextField,
    pub(crate) input_target: InputTarget,
    pub(crate) available_models: Vec<ModelInfo>,
    pub(crate) favorite_models: Vec<String>,
    pub(crate) model_picker_open: bool,
    pub(crate) loading_models: bool,
    pub(crate) settings: AgentSettings,
    pub(crate) provider_config: ProviderConfig,
    pub(crate) show_settings: bool,
    pub(crate) pending: bool,
    pub(crate) status: String,
}

impl Default for ChatApp {
    fn default() -> Self {
        let loaded_keys = ProviderId::ALL.map(secure_keys::load_api_key);
        let key_store_error = loaded_keys
            .iter()
            .find_map(|result| result.as_ref().err())
            .cloned();
        let key_for = |index: usize| loaded_keys[index].as_ref().ok().cloned().flatten();
        let provider_config = ProviderConfig {
            openai: ProviderSettings {
                api_key: key_for(0),
                model: default_model(ProviderId::OpenAi).into(),
            },
            gemini: ProviderSettings {
                api_key: key_for(1),
                model: default_model(ProviderId::Gemini).into(),
            },
            anthropic: ProviderSettings {
                api_key: key_for(2),
                model: default_model(ProviderId::Anthropic).into(),
            },
        };

        let mut settings = AgentSettings::default();
        settings.system_prompt = AGENT_MD.trim().to_owned();

        Self {
            conversations: Vec::new(),
            active_conversation: None,
            next_conversation_id: 1,
            messages: Vec::new(),
            draft: TextField::new(""),
            api_key_field: TextField::new(""),
            model_field: TextField::new(default_model(ProviderId::OpenAi)),
            model_search_field: TextField::new(""),
            input_target: InputTarget::None,
            available_models: Vec::new(),
            favorite_models: Vec::new(),
            model_picker_open: false,
            loading_models: false,
            settings,
            provider_config,
            show_settings: false,
            pending: false,
            status: key_store_error
                .map(|error| format!("Could not access encrypted API key storage: {error}"))
                .unwrap_or_else(|| "Add an API key in Agent settings to begin".into()),
        }
    }
}

impl ChatApp {
    pub fn new() -> Self {
        Self::default()
    }

    fn drain_text(&mut self) {
        FOCUS_REQUESTED.with(|requested| {
            if let Some(target) = requested.borrow_mut().take() {
                self.input_target = target;
            }
        });

        if self.input_target != InputTarget::None {
            let max_chars = match self.input_target {
                InputTarget::Message => MAX_MESSAGE_INPUT_CHARS,
                InputTarget::ApiKey => MAX_API_KEY_INPUT_CHARS,
                InputTarget::Model | InputTarget::ModelSearch => MAX_MODEL_INPUT_CHARS,
                InputTarget::None => 0,
            };
            PENDING_TEXT.with(|pending| {
                for text in pending.borrow_mut().drain(..) {
                    let field = match self.input_target {
                        InputTarget::Message => &mut self.draft,
                        InputTarget::ApiKey => &mut self.api_key_field,
                        InputTarget::Model => &mut self.model_field,
                        InputTarget::ModelSearch => &mut self.model_search_field,
                        InputTarget::None => unreachable!(),
                    };
                    match text.as_str() {
                        "\x08" | "\x7f" => field.delete_at_cursor(),
                        "\x1b[D" => field.move_cursor_left(),
                        "\x1b[C" => field.move_cursor_right(),
                        "\x1b[H" => field.move_cursor_to_start(),
                        "\x1b[F" => field.move_cursor_to_end(),
                        text => field.insert_at_cursor(text),
                    }
                    cap_text_field(field, max_chars);
                }
            });
        } else {
            clear_pending_text();
        }

        if self.input_target == InputTarget::ApiKey {
            self.update_current_api_key_in_memory();
        }
    }

    pub(crate) fn current_api_key(&self) -> &str {
        &self.api_key_field.text
    }

    fn update_current_api_key_in_memory(&mut self) {
        if !self.show_settings && self.input_target != InputTarget::ApiKey {
            return;
        }
        let key = self.api_key_field.text.trim().to_owned();
        self.provider_config
            .settings_mut(self.settings.provider)
            .api_key = (!key.is_empty()).then_some(key);
    }

    pub(crate) fn save_current_api_key(&mut self) {
        self.update_current_api_key_in_memory();
        let provider = self.settings.provider;
        let key = self.provider_config.settings(provider).api_key.as_deref();
        self.status = match secure_keys::save_api_key(provider, key) {
            Ok(()) if key.is_some() => "API key saved in encrypted Android Keystore storage".into(),
            Ok(()) => "Saved API key removed from this device".into(),
            Err(error) => format!("Could not securely save API key: {error}"),
        };
    }

    pub(crate) fn remove_current_api_key(&mut self) {
        self.api_key_field.text.clear();
        self.api_key_field.cursor = 0;
        self.api_key_field.selection = None;
        self.save_current_api_key();
        self.input_target = InputTarget::None;
        clear_pending_text();
        gpui_mobile::hide_keyboard();
    }

    fn load_current_api_key(&mut self) {
        self.api_key_field.text = self
            .provider_config
            .settings(self.settings.provider)
            .api_key
            .clone()
            .unwrap_or_default();
        self.api_key_field.cursor = self.api_key_field.text.len();
        self.api_key_field.selection = None;
    }

    pub(crate) fn save_current_model(&mut self) {
        let model = self.model_field.text.trim().to_owned();
        if model.is_empty() {
            return;
        }
        self.provider_config
            .settings_mut(self.settings.provider)
            .model = model.clone();
        self.model_field.text = model;
        self.model_field.cursor = self.model_field.text.len();
        self.model_field.selection = None;
    }

    fn load_current_model(&mut self) {
        self.model_field.text = self
            .provider_config
            .settings(self.settings.provider)
            .model
            .clone();
        self.model_field.cursor = self.model_field.text.len();
        self.model_field.selection = None;
    }

    pub(crate) fn toggle_model_picker(&mut self) {
        self.model_picker_open = !self.model_picker_open;
        self.input_target = InputTarget::None;
        clear_pending_text();
        gpui_mobile::hide_keyboard();
    }

    pub(crate) fn refresh_models(&mut self, cx: &mut Context<Self>) {
        self.save_current_api_key();
        if self.loading_models {
            return;
        }

        let providers = ProviderId::ALL
            .into_iter()
            .filter(|provider| self.provider_config.settings(*provider).api_key.is_some())
            .collect::<Vec<_>>();
        if providers.is_empty() {
            self.status = "Configure at least one provider API key before loading models".into();
            return;
        }

        self.loading_models = true;
        self.status = format!("Loading models from {} providers...", providers.len());
        let config = self.provider_config.clone();
        cx.notify();
        cx.spawn(async move |this, cx| {
            let client = AiClient::new(reqwest::Client::new(), config);
            let mut all_models = Vec::new();
            let mut failed_providers = Vec::new();
            for provider in providers.iter().copied() {
                match client.list_models(provider).await {
                    Ok(mut models) => all_models.append(&mut models),
                    Err(_) => failed_providers.push(provider.display_name()),
                }
            }
            let _ = this.update(cx, |app, cx| {
                app.loading_models = false;
                let current_provider = app.settings.provider;
                let current_model = app.provider_config.settings(current_provider).model.clone();
                let count = all_models.len();
                app.available_models = all_models;
                if current_model == default_model(current_provider) {
                    if let Some(latest) = newest_model(current_provider, &app.available_models) {
                        app.use_provider_model(current_provider, latest.clone());
                    }
                }
                app.status = if failed_providers.is_empty() {
                    format!("Loaded {count} models from {} providers", providers.len())
                } else {
                    format!(
                        "Loaded {count} models; unavailable: {}",
                        failed_providers.join(", ")
                    )
                };
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn toggle_model_favorite(&mut self, model: String) {
        if let Some(index) = self
            .favorite_models
            .iter()
            .position(|favorite| favorite == &model)
        {
            self.favorite_models.remove(index);
        } else {
            self.favorite_models.push(model);
        }
    }

    pub(crate) fn use_provider_model(&mut self, provider: ProviderId, model: String) {
        if self.settings.provider != provider {
            self.select_provider(provider);
        }
        self.use_model(model);
    }

    pub(crate) fn use_model(&mut self, model: String) {
        let model = model.trim().to_owned();
        if model.is_empty() {
            return;
        }
        self.model_field.text = model.clone();
        self.model_field.cursor = model.len();
        self.model_field.selection = None;
        self.provider_config
            .settings_mut(self.settings.provider)
            .model = model.clone();
        self.model_picker_open = false;
        self.input_target = InputTarget::None;
        clear_pending_text();
        gpui_mobile::hide_keyboard();
        self.status = format!("Selected {model}");
    }

    fn save_active_conversation(&mut self) {
        let Some(id) = self.active_conversation else {
            return;
        };
        if let Some(conversation) = self.conversations.iter_mut().find(|chat| chat.id == id) {
            conversation.messages = self.messages.clone();
            if let Some(last_user) = self
                .messages
                .iter()
                .rev()
                .find(|message| message.role == Role::User)
            {
                conversation.preview = preview_text(&last_user.content);
                if conversation.title == "New chat" {
                    conversation.title = title_text(&last_user.content);
                }
            }
        }
    }

    pub(crate) fn start_new_conversation(&mut self) {
        self.save_active_conversation();
        let id = self.next_conversation_id;
        self.next_conversation_id += 1;
        self.conversations.insert(
            0,
            Conversation {
                id,
                title: "New chat".into(),
                preview: "Start a conversation".into(),
                messages: Vec::new(),
            },
        );
        self.active_conversation = Some(id);
        self.messages.clear();
        self.draft.text.clear();
        self.draft.cursor = 0;
        self.draft.selection = None;
        self.input_target = InputTarget::None;
        self.model_picker_open = false;
        clear_pending_text();
        self.show_settings = false;
        self.status = "New conversation".into();
        gpui_mobile::hide_keyboard();
    }

    pub(crate) fn open_conversation(&mut self, id: u64) {
        self.save_active_conversation();
        if let Some(conversation) = self.conversations.iter().find(|chat| chat.id == id) {
            self.active_conversation = Some(id);
            self.messages = conversation.messages.clone();
            self.show_settings = false;
            self.model_picker_open = false;
            self.input_target = InputTarget::None;
            clear_pending_text();
            self.status = "Ready".into();
        }
        gpui_mobile::hide_keyboard();
    }

    pub(crate) fn show_conversation_list(&mut self) {
        self.save_active_conversation();
        self.active_conversation = None;
        self.show_settings = false;
        self.model_picker_open = false;
        self.input_target = InputTarget::None;
        clear_pending_text();
        self.status = "Your conversations".into();
        gpui_mobile::hide_keyboard();
    }

    pub(crate) fn send_message(&mut self, cx: &mut Context<Self>) {
        self.save_current_api_key();
        if self.pending {
            return;
        }
        let content = self.draft.text.trim().to_owned();
        if content.is_empty() {
            return;
        }
        if self
            .provider_config
            .settings(self.settings.provider)
            .api_key
            .is_none()
        {
            self.show_settings = true;
            self.status = format!(
                "Add your {} API key in Agent settings first",
                self.settings.provider.display_name()
            );
            return;
        }

        if self.active_conversation.is_none() {
            self.start_new_conversation();
        }
        let conversation_id = self.active_conversation.expect("conversation started");
        self.messages.push(ChatMessage {
            role: Role::User,
            content,
        });
        self.save_active_conversation();
        self.draft.text.clear();
        self.draft.cursor = 0;
        self.draft.selection = None;
        self.input_target = InputTarget::None;
        clear_pending_text();
        self.pending = true;
        self.status = format!("{} is thinking...", self.settings.provider.display_name());
        gpui_mobile::hide_keyboard();

        let provider_config = self.provider_config.clone();
        let settings = self.settings.clone();
        let messages = self.messages.clone();
        cx.spawn(async move |this, cx| {
            let client = AiClient::new(reqwest::Client::new(), provider_config);
            let result = client.complete_agent(&settings, &messages).await;
            let _ = this.update(cx, |app, cx| {
                app.pending = false;
                match result {
                    Ok(agent_response) => {
                        if agent_response.compacted {
                            app.status = "Context compacted automatically".into();
                        } else {
                            app.status = format!("{} replied", agent_response.response.model);
                        }
                        let mut updated_messages = agent_response.messages;
                        updated_messages.push(ChatMessage {
                            role: Role::Assistant,
                            content: agent_response.response.content,
                        });
                        if let Some(conversation) = app
                            .conversations
                            .iter_mut()
                            .find(|conversation| conversation.id == conversation_id)
                        {
                            conversation.messages = updated_messages.clone();
                            if let Some(last_message) = updated_messages
                                .iter()
                                .rev()
                                .find(|message| message.role == Role::Assistant)
                            {
                                conversation.preview = preview_text(&last_message.content);
                            }
                        }
                        if app.active_conversation == Some(conversation_id) {
                            app.messages = updated_messages;
                        }
                    }
                    Err(error) => {
                        app.status = error.to_string();
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn select_provider(&mut self, provider: ProviderId) {
        self.save_current_api_key();
        self.save_current_model();
        self.settings.provider = provider;
        self.model_picker_open = false;
        self.input_target = InputTarget::None;
        clear_pending_text();
        self.model_search_field.text.clear();
        self.model_search_field.cursor = 0;
        self.load_current_api_key();
        self.load_current_model();
        self.status = format!("Using {}", provider.display_name());
    }

    pub(crate) fn toggle_settings(&mut self) {
        self.save_current_api_key();
        self.save_active_conversation();
        self.show_settings = !self.show_settings;
        self.model_picker_open = false;
        self.input_target = InputTarget::None;
        clear_pending_text();
        gpui_mobile::hide_keyboard();
        if self.show_settings {
            self.load_current_api_key();
        } else {
            gpui_mobile::hide_keyboard();
        }
    }
}

fn preview_text(content: &str) -> String {
    let preview = content.chars().take(84).collect::<String>();
    if content.chars().count() > 84 {
        format!("{preview}...")
    } else {
        preview
    }
}

fn title_text(content: &str) -> String {
    let title = content.split_whitespace().collect::<Vec<_>>().join(" ");
    let title = title.chars().take(36).collect::<String>();
    if content.chars().count() > 36 {
        format!("{title}...")
    } else {
        title
    }
}

fn cap_text_field(field: &mut TextField, max_chars: usize) {
    if field.text.chars().count() <= max_chars {
        return;
    }
    field.text = field.text.chars().take(max_chars).collect();
    field.cursor = field.cursor.min(field.text.len());
    field.selection = None;
}

impl Render for ChatApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.drain_text();
        let theme = midnight_theme();
        let show_settings = self.show_settings;
        let provider = self.settings.provider.display_name();
        let model = self
            .provider_config
            .settings(self.settings.provider)
            .model
            .clone();
        let active_id = self.active_conversation;
        let is_inbox = active_id.is_none() && !show_settings;
        let title = if show_settings {
            "Agent settings".to_owned()
        } else if let Some(id) = active_id {
            self.conversations
                .iter()
                .find(|conversation| conversation.id == id)
                .map(|conversation| conversation.title.clone())
                .unwrap_or_else(|| "Conversation".into())
        } else {
            "AI Chat".into()
        };
        let safe_top = android_safe_top_inset();

        set_system_chrome(&SystemChromeStyle {
            status_bar_color: Some(0x101610),
            status_bar_style: StatusBarContentStyle::Light,
            navigation_bar_color: Some(theme.surface),
        });

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(theme.surface))
            .text_color(rgb(theme.on_surface))
            .font_family(CHAT_FONT_FAMILY)
            .pb(px(android_keyboard_bottom_inset()))
            .child(div().w_full().h(px(safe_top.max(28.0))).bg(rgb(0x101610)))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_3()
                    .px_4()
                    .py_3()
                    .bg(rgb(0x101610))
                    .text_color(rgb(theme.on_surface))
                    .font_family(MISC_FONT_FAMILY)
                    .when(!is_inbox, |bar| {
                        bar.child(
                            div()
                                .px_2()
                                .py_1()
                                .text_lg()
                                .text_color(rgb(theme.primary))
                                .child("<")
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|this, _, _, cx| {
                                        if this.show_settings {
                                            this.toggle_settings();
                                        } else {
                                            this.show_conversation_list();
                                        }
                                        cx.notify();
                                    }),
                                ),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .gap_1()
                            .child(
                                div()
                                    .text_lg()
                                    .text_color(rgb(theme.on_surface))
                                    .font_family(MISC_FONT_FAMILY)
                                    .text_xl()
                                    .when(is_inbox, |title| title.text_color(rgb(0xD5E8D5)))
                                    .child(title),
                            )
                            .when(active_id.is_some() && !show_settings, |header| {
                                header.child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(theme.on_surface_variant))
                                        .child(format!("{provider} / {model}")),
                                )
                            }),
                    )
                    .child(if is_inbox {
                        div()
                            .px_3()
                            .py_2()
                            .rounded_lg()
                            .bg(rgb(theme.primary))
                            .text_color(rgb(theme.on_primary))
                            .child("+ New")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    this.start_new_conversation();
                                    cx.notify();
                                }),
                            )
                            .into_any_element()
                    } else if active_id.is_some() && !show_settings {
                        div()
                            .px_3()
                            .py_2()
                            .rounded_lg()
                            .bg(rgb(theme.primary))
                            .text_color(rgb(theme.on_primary))
                            .text_sm()
                            .child("Settings")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_settings();
                                    cx.notify();
                                }),
                            )
                            .into_any_element()
                    } else {
                        div().into_any_element()
                    }),
            )
            .child(if show_settings {
                settings_screen::render(self, cx).into_any_element()
            } else if active_id.is_some() {
                chat_screen::render(self, provider, cx).into_any_element()
            } else {
                chat_list_screen::render(self, cx).into_any_element()
            })
    }
}

#[cfg(target_os = "android")]
fn android_safe_top_inset() -> f32 {
    gpui_mobile::android::jni::platform()
        .and_then(|platform| platform.primary_window())
        .map(|window| window.safe_area_insets_logical().top)
        .unwrap_or(0.0)
}

#[cfg(not(target_os = "android"))]
fn android_safe_top_inset() -> f32 {
    0.0
}

#[cfg(target_os = "android")]
fn android_keyboard_bottom_inset() -> f32 {
    let physical_inset = gpui_mobile::android::jni::with_env(|env| {
        let activity = gpui_mobile::android::jni::activity(env)?;
        let window = env
            .call_method(
                &activity,
                jni::jni_str!("getWindow"),
                jni::jni_sig!("()Landroid/view/Window;"),
                &[],
            )
            .and_then(|value: jni::objects::JValueOwned| value.l())
            .map_err(|error| error.to_string())?;
        let decor = env
            .call_method(
                &window,
                jni::jni_str!("getDecorView"),
                jni::jni_sig!("()Landroid/view/View;"),
                &[],
            )
            .and_then(|value: jni::objects::JValueOwned| value.l())
            .map_err(|error| error.to_string())?;
        let rect_class = env
            .find_class(jni::jni_str!("android/graphics/Rect"))
            .map_err(|error| error.to_string())?;
        let rect = env
            .new_object(&rect_class, jni::jni_sig!("()V"), &[])
            .map_err(|error| error.to_string())?;
        env.call_method(
            &decor,
            jni::jni_str!("getWindowVisibleDisplayFrame"),
            jni::jni_sig!("(Landroid/graphics/Rect;)V"),
            &[JValue::Object(&rect)],
        )
        .map_err(|error| error.to_string())?;
        let view_height = env
            .call_method(
                &decor,
                jni::jni_str!("getHeight"),
                jni::jni_sig!("()I"),
                &[],
            )
            .and_then(|value: jni::objects::JValueOwned| value.i())
            .map_err(|error| error.to_string())?;
        let visible_bottom = env
            .get_field(&rect, jni::jni_str!("bottom"), jni::jni_sig!("I"))
            .and_then(|value: jni::objects::JValueOwned| value.i())
            .map_err(|error| error.to_string())?;
        Ok::<i32, String>((view_height - visible_bottom).max(0))
    })
    .unwrap_or(0);

    let scale = gpui_mobile::android::jni::platform()
        .and_then(|platform| platform.primary_window())
        .map(|window| window.scale_factor())
        .unwrap_or(1.0);
    physical_inset as f32 / scale
}

#[cfg(not(target_os = "android"))]
fn android_keyboard_bottom_inset() -> f32 {
    0.0
}

#[cfg(target_os = "android")]
pub fn open_main_window(cx: &mut App) {
    if let Err(error) = cx
        .text_system()
        .add_fonts(vec![Cow::Borrowed(CHAT_FONT), Cow::Borrowed(MISC_FONT)])
    {
        log::warn!("failed to load bundled app fonts: {error:#}");
    }

    let _ = cx.open_window(
        gpui::WindowOptions {
            window_bounds: None,
            ..Default::default()
        },
        |_, cx| cx.new(|_| ChatApp::new()),
    );
    cx.activate(true);
}
