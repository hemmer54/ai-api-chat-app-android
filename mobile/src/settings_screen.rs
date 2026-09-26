use crate::app::{
    install_keyboard_callback, midnight_theme, request_focus, InputTarget, CHAT_FONT_FAMILY,
};
use crate::ChatApp;
use chat_core::{ModelInfo, ProviderId};
use gpui::{div, prelude::*, px, rgb, Context, MouseButton};
use gpui_mobile::components::material::{MaterialTheme, TextInput};
use gpui_mobile::KeyboardType;

pub fn render(app: &ChatApp, cx: &mut Context<ChatApp>) -> impl IntoElement {
    let theme = midnight_theme();
    let settings = &app.settings;
    let key_length = app.current_api_key().len();
    let masked_key = "*".repeat(key_length);
    let key_focused = app.input_target == InputTarget::ApiKey;
    let model_focused = app.input_target == InputTarget::Model;
    let search_focused = app.input_target == InputTarget::ModelSearch;
    let status = app.status.clone();
    let selected_model = app
        .provider_config
        .settings(settings.provider)
        .model
        .clone();
    let key_state = if key_length == 0 {
        "Not configured"
    } else {
        "Configured"
    };
    let search = app.model_search_field.text.to_ascii_lowercase();
    let preset_models = preset_models(settings.provider);
    let matching_presets = preset_models
        .iter()
        .filter(|model| {
            model.id.to_ascii_lowercase().contains(&search)
                || model.display_name.to_ascii_lowercase().contains(&search)
        })
        .cloned()
        .collect::<Vec<_>>();
    let matching_models = app
        .available_models
        .iter()
        .filter(|model| model.provider == settings.provider)
        .filter(|model| {
            model.id.to_ascii_lowercase().contains(&search)
                || model.display_name.to_ascii_lowercase().contains(&search)
        })
        .cloned()
        .collect::<Vec<_>>();

    div()
        .id("agent-settings-scroll")
        .flex()
        .flex_col()
        .flex_1()
        .gap_2()
        .font_family(CHAT_FONT_FAMILY)
        .px_4()
        .py_3()
        .overflow_y_scroll()
        .child(
            div()
                .pb_2()
                .child(div().text_xl().font_weight(gpui::FontWeight::SEMIBOLD).child("Agent settings")),
        )
        .child(section_card(
            "Provider",
            div()
                .flex()
                .flex_row()
                .gap_2()
                .children(ProviderId::ALL.into_iter().map(|provider| {
                    let selected = provider == settings.provider;
                    let configured = app.provider_config.settings(provider).api_key.is_some();
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .items_start()
                        .px_2()
                        .py_2()
                        .rounded_lg()
                        .border_1()
                        .border_color(rgb(if selected { theme.primary } else { theme.outline_variant }))
                        .bg(rgb(if selected { theme.primary_container } else { theme.surface_container_low }))
                        .text_color(rgb(if selected { theme.on_primary_container } else { theme.on_surface }))
                        .child(provider.display_name())
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(if configured { theme.tertiary } else { theme.on_surface_variant }))
                                .child(if configured { "Ready" } else { "Key needed" }),
                        )
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _, _, cx| {
                                this.select_provider(provider);
                                cx.notify();
                            }),
                        )
                })),
            theme,
        ))
        .child(section_card(
            "Model",
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(div().text_xs().text_color(rgb(theme.on_surface_variant)).child("ACTIVE MODEL"))
                                .child(div().text_base().child(selected_model.clone())),
                        )
                        .child(action_button(
                            if app.model_picker_open { "Close" } else { "Choose model" },
                            theme,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_model_picker();
                                cx.notify();
                            }),
                        )),
                )
                .when(app.model_picker_open, |panel| {
                    panel
                        .p_3()
                        .rounded_xl()
                        .border_1()
                        .border_color(rgb(theme.outline_variant))
                        .bg(rgb(theme.surface))
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .justify_between()
                                .p_2()
                                .rounded_lg()
                                .bg(rgb(theme.surface_container))
                                .child(div().text_xs().font_weight(gpui::FontWeight::SEMIBOLD).child("MODEL SELECTOR"))
                                .child(action_button(
                                    if app.loading_models { "Loading..." } else { "Refresh all" },
                                    theme,
                                    cx.listener(|this, _, _, cx| this.refresh_models(cx)),
                                )),
                        )
                        .when(search_focused, |panel| {
                            panel.child(
                                TextInput::<ChatApp>::new("model-search", theme)
                                    .label("Search")
                                    .value(&app.model_search_field.text)
                                    .cursor(app.model_search_field.cursor)
                                    .selection(app.model_search_field.normalized_selection())
                                    .placeholder("Filter models")
                                    .keyboard_type(KeyboardType::Default)
                                    .focused(true)
                                    .on_tap_notify(|_| {
                                        request_focus(InputTarget::ModelSearch);
                                        install_keyboard_callback();
                                        gpui_mobile::show_keyboard_with_type(KeyboardType::Default);
                                    })
                                    .render(cx),
                            )
                        })
                        .when(!search_focused, |panel| {
                            panel.child(action_button(
                                "Search models",
                                theme,
                                cx.listener(|_, _, _, cx| {
                                    request_focus(InputTarget::ModelSearch);
                                    install_keyboard_callback();
                                    gpui_mobile::show_keyboard_with_type(KeyboardType::Default);
                                    cx.notify();
                                }),
                            ))
                        })
                        .when(!matching_presets.is_empty(), |panel| {
                            panel.child(
                                div()
                                    .id("preset-results")
                                    .h(px(180.0))
                                    .overflow_y_scroll()
                                    .flex()
                                    .flex_col()
                                    .child(model_group_title("Presets", theme))
                                    .children(matching_presets.into_iter().map(|model| {
                                        let favorite = app.favorite_models.iter().any(|id| id == &model.id);
                                        model_row(model, &selected_model, favorite, theme, cx)
                                    })),
                            )
                        })
                        .when(!matching_models.is_empty(), |panel| {
                            let favorite_models = matching_models
                                .iter()
                                .filter(|model| app.favorite_models.iter().any(|favorite| favorite == &model.id))
                                .cloned()
                                .collect::<Vec<_>>();
                            let other_models = matching_models
                                .iter()
                                .filter(|model| !app.favorite_models.iter().any(|favorite| favorite == &model.id))
                                .cloned()
                                .collect::<Vec<_>>();
                            panel.child(
                                div()
                                    .id("model-results")
                                    .h(px(220.0))
                                    .overflow_y_scroll()
                                    .flex()
                                    .flex_col()
                                    .when(!favorite_models.is_empty(), |results| {
                                        results
                                            .child(model_group_title("Favorites", theme))
                                            .children(favorite_models.into_iter().take(20).map(|model| {
                                                model_row(model, &selected_model, true, theme, cx)
                                            }))
                                    })
                                    .child(model_group_title("Available models", theme))
                                    .children(other_models.into_iter().take(80).map(|model| {
                                        model_row(model, &selected_model, false, theme, cx)
                                    })),
                            )
                        })
                        .when(app.available_models.is_empty() && !app.loading_models, |panel| {
                            panel.child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(theme.on_surface_variant))
                                    .child("Presets are ready. Refresh after adding an API key for more models."),
                            )
                        })
                        .when(model_focused, |panel| {
                            panel
                                .child(
                                    TextInput::<ChatApp>::new("custom-model", theme)
                                        .label("Model ID")
                                        .value(&app.model_field.text)
                                        .cursor(app.model_field.cursor)
                                        .selection(app.model_field.normalized_selection())
                                        .placeholder("provider/model-name")
                                        .keyboard_type(KeyboardType::Default)
                                        .focused(true)
                                        .on_tap_notify(|_| {
                                            request_focus(InputTarget::Model);
                                            install_keyboard_callback();
                                            gpui_mobile::show_keyboard_with_type(KeyboardType::Default);
                                        })
                                        .render(cx),
                                )
                                .child(action_button(
                                    "Use model ID",
                                    theme,
                                    cx.listener(|this, _, _, cx| {
                                        let custom_model = this.model_field.text.clone();
                                        this.use_model(custom_model);
                                        cx.notify();
                                    }),
                                ))
                        })
                        .when(!model_focused, |panel| {
                            panel.child(action_button(
                                "Enter model ID",
                                theme,
                                cx.listener(|_, _, _, cx| {
                                    request_focus(InputTarget::Model);
                                    install_keyboard_callback();
                                    gpui_mobile::show_keyboard_with_type(KeyboardType::Default);
                                    cx.notify();
                                }),
                            ))
                        })
                }),
            theme,
        ))
        .child(section_card(
            "Authentication",
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .justify_between()
                        .items_center()
                        .child(div().text_sm().child(format!("{} API key", settings.provider.display_name())))
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(if key_length == 0 { theme.error } else { theme.tertiary }))
                                .child(key_state),
                        ),
                )
                .child(
                    TextInput::<ChatApp>::new("api-key-input", theme)
                        .label("API key")
                        .value(&masked_key)
                        .cursor(key_length)
                        .placeholder("Paste your provider API key")
                        .keyboard_type(KeyboardType::Default)
                        .focused(key_focused)
                        .on_tap_notify(|_| {
                            request_focus(InputTarget::ApiKey);
                            install_keyboard_callback();
                            gpui_mobile::show_keyboard_with_type(KeyboardType::Default);
                        })
                        .render(cx),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .gap_2()
                        .child(action_button(
                            "Save key securely",
                            theme,
                            cx.listener(|this, _, _, cx| {
                                this.save_current_api_key();
                                gpui_mobile::hide_keyboard();
                                this.input_target = InputTarget::None;
                                cx.notify();
                            }),
                        ))
                        .when(key_length > 0, |row| {
                            row.child(action_button(
                                "Remove key",
                                theme,
                                cx.listener(|this, _, _, cx| {
                                    this.remove_current_api_key();
                                    cx.notify();
                                }),
                            ))
                        }),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(theme.on_surface_variant))
                        .child("Encrypted with Android Keystore. Plaintext exists in app memory while in use."),
                ),
            theme,
        ))
        .child(section_card(
            "Generation",
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(control_row(
                    "Temperature",
                    format!("{:.1}", settings.temperature),
                    "Response variation",
                    theme,
                    cx.listener(|this, _, _, cx| {
                        this.settings.temperature = match this.settings.temperature {
                            value if value < 0.5 => 0.7,
                            value if value < 0.9 => 1.0,
                            _ => 0.2,
                        };
                        cx.notify();
                    }),
                ))
                .child(control_row(
                    "Output limit",
                    format!("{} tokens", settings.max_output_tokens),
                    "Maximum response length",
                    theme,
                    cx.listener(|this, _, _, cx| {
                        this.settings.max_output_tokens = match this.settings.max_output_tokens {
                            512 => 1024,
                            1024 => 2048,
                            _ => 512,
                        };
                        cx.notify();
                    }),
                )),
            theme,
        ))
        .child(section_card(
            "Context",
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(theme.on_surface_variant))
                        .child("Older turns are summarized automatically before the context becomes too large."),
                )
                .child(control_row(
                    "Compact at",
                    format!("{} messages", settings.context.max_messages),
                    format!("{} characters", settings.context.max_chars),
                    theme,
                    cx.listener(|this, _, _, cx| {
                        this.settings.context.max_messages = match this.settings.context.max_messages {
                            16 => 40,
                            40 => 80,
                            _ => 16,
                        };
                        this.settings.context.max_chars = match this.settings.context.max_chars {
                            12_000 => 24_000,
                            24_000 => 48_000,
                            _ => 12_000,
                        };
                        cx.notify();
                    }),
                ))
                .child(control_row(
                    "Keep recent",
                    format!("{} messages", settings.context.preserve_recent_messages),
                    "Always preserve recent turns",
                    theme,
                    cx.listener(|this, _, _, cx| {
                        this.settings.context.preserve_recent_messages =
                            match this.settings.context.preserve_recent_messages {
                                8 => 12,
                                12 => 20,
                                _ => 8,
                            };
                        cx.notify();
                    }),
                )),
            theme,
        ))
        .child(
            div()
                .p_3()
                .rounded_lg()
                .border_1()
                .border_color(rgb(theme.outline_variant))
                .bg(rgb(theme.surface_container))
                .text_xs()
                .text_color(rgb(theme.on_surface_variant))
                .child(status),
        )
}

fn preset_models(provider: ProviderId) -> Vec<ModelInfo> {
    let models = match provider {
        ProviderId::OpenAi => vec![
            ("luna-5.6", "Luna 5.6", 256_000, false),
            ("gpt-5.2", "GPT 5.2", 272_000, true),
            ("gpt-5-mini", "GPT 5 Mini", 128_000, true),
        ],
        ProviderId::Gemini => vec![
            ("gemini-3.1-pro-preview", "Gemini 3.1 Pro", 1_000_000, true),
            ("gemini-3-flash-preview", "Gemini 3 Flash", 1_000_000, true),
        ],
        ProviderId::Anthropic => vec![
            ("claude-haiku-4-5", "Claude Haiku 4.5", 200_000, true),
            ("claude-sonnet-4-latest", "Claude Sonnet 4", 200_000, true),
            ("claude-opus-4-latest", "Claude Opus 4", 200_000, true),
        ],
    };

    models
        .into_iter()
        .map(
            |(id, display_name, context_window, supports_vision)| ModelInfo {
                id: id.into(),
                display_name: display_name.into(),
                provider,
                context_window: Some(context_window),
                supports_vision,
            },
        )
        .collect()
}

fn model_group_title(title: &str, theme: MaterialTheme) -> impl IntoElement {
    div()
        .pt_2()
        .text_xs()
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(rgb(theme.on_surface_variant))
        .child(title.to_owned())
}

fn model_row(
    model: ModelInfo,
    selected_model: &str,
    favorite: bool,
    theme: MaterialTheme,
    cx: &mut Context<ChatApp>,
) -> impl IntoElement {
    let selected = model.id == selected_model;
    let model_id = model.id.clone();
    let favorite_model_id = model.id.clone();
    let model_provider = model.provider;
    let display_name = model.display_name.clone();
    let model_id_label = model.id.clone();

    div()
        .flex()
        .flex_row()
        .items_center()
        .gap_2()
        .px_3()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(rgb(if selected {
            theme.primary
        } else {
            theme.outline_variant
        }))
        .bg(rgb(if selected {
            theme.primary_container
        } else {
            theme.surface_container
        }))
        .text_color(rgb(if selected {
            theme.on_primary_container
        } else {
            theme.on_surface
        }))
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .gap_1()
                .child(div().font_family(CHAT_FONT_FAMILY).child(display_name))
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(theme.on_surface_variant))
                        .child(format!(
                            "{} / {}",
                            model_provider.display_name(),
                            model_id_label
                        )),
                ),
        )
        .child(
            div()
                .px_2()
                .py_1()
                .child(if favorite { "FAV" } else { "SAVE" })
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _, _, cx| {
                        this.toggle_model_favorite(favorite_model_id.clone());
                        cx.notify();
                    }),
                ),
        )
        .when(selected, |row| row.child(div().text_xs().child("Selected")))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _, _, cx| {
                this.use_provider_model(model_provider, model_id.clone());
                cx.notify();
            }),
        )
}

fn section_card(title: &str, content: impl IntoElement, theme: MaterialTheme) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .py_2()
        .font_family(CHAT_FONT_FAMILY)
        .border_b_1()
        .border_color(rgb(theme.outline_variant))
        .child(
            div()
                .text_xs()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(rgb(theme.primary))
                .child(title.to_owned()),
        )
        .child(content)
}

fn action_button(
    title: &str,
    theme: MaterialTheme,
    handler: impl Fn(&gpui::MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .px_3()
        .py_2()
        .rounded_lg()
        .border_1()
        .border_color(rgb(theme.outline))
        .bg(rgb(theme.surface_container))
        .text_sm()
        .child(title.to_owned())
        .on_mouse_down(MouseButton::Left, handler)
}

fn control_row(
    label: &str,
    value: String,
    description: impl Into<String>,
    theme: MaterialTheme,
    handler: impl Fn(&gpui::MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .p_3()
        .rounded_lg()
        .border_1()
        .border_color(rgb(theme.outline_variant))
        .bg(rgb(theme.surface_container))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_sm()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .child(label.to_owned()),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(theme.on_surface_variant))
                        .child(description.into()),
                ),
        )
        .child(
            div()
                .px_3()
                .py_2()
                .rounded_lg()
                .border_1()
                .border_color(rgb(theme.outline))
                .bg(rgb(theme.primary_container))
                .text_color(rgb(theme.on_primary_container))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .child(value),
        )
        .on_mouse_down(MouseButton::Left, handler)
}
