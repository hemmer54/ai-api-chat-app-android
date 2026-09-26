use crate::app::{
    install_keyboard_callback, midnight_theme, request_focus, InputTarget, CHAT_FONT_FAMILY,
};
use crate::ChatApp;
use gpui::{div, prelude::*, px, rgb, Context, MouseButton};
use gpui_mobile::components::material::TextInput;
use gpui_mobile::KeyboardType;

pub fn render(app: &ChatApp, provider: &str, cx: &mut Context<ChatApp>) -> impl IntoElement {
    let theme = midnight_theme();
    let draft = app.draft.text.clone();
    let cursor = app.draft.cursor;
    let selection = app.draft.normalized_selection();
    let messages = app
        .messages
        .iter()
        .filter(|message| message.role != chat_core::Role::System)
        .cloned()
        .collect::<Vec<_>>();
    let focused = app.input_target == InputTarget::Message;
    let status = app.status.clone();
    let status_line = if app.pending {
        "Generating response...".to_owned()
    } else {
        status.clone()
    };
    let model = app
        .provider_config
        .settings(app.settings.provider)
        .model
        .clone();

    div()
        .flex()
        .flex_col()
        .flex_1()
        .px_4()
        .font_family(CHAT_FONT_FAMILY)
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .py_3()
                .border_b_1()
                .border_color(rgb(theme.outline_variant))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_base()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .child(provider.to_owned()),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(theme.on_surface_variant))
                                .child(model.clone()),
                        ),
                )
                .child(
                    div()
                        .px_3()
                        .py_2()
                        .rounded_lg()
                        .border_1()
                        .border_color(rgb(theme.outline))
                        .bg(rgb(theme.surface_container))
                        .text_sm()
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .font_family(CHAT_FONT_FAMILY)
                        .child(format!("Model: {model}"))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_settings();
                                cx.notify();
                            }),
                        ),
                ),
        )
        .child(
            div()
                .id("messages")
                .flex()
                .flex_col()
                .flex_1()
                .gap_3()
                .py_4()
                .overflow_y_scroll()
                .child(if messages.is_empty() {
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .items_center()
                        .justify_center()
                        .gap_2()
                        .child(
                            div()
                                .text_xl()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .child("Start a conversation"),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(theme.on_surface_variant))
                                .child("Your provider key stays on this device."),
                        )
                        .into_any_element()
                } else {
                    messages
                        .into_iter()
                        .fold(div().flex().flex_col().gap_3(), |list, message| {
                            let is_user = message.role == chat_core::Role::User;
                            list.child(
                                div()
                                    .flex()
                                    .justify_start()
                                    .when(is_user, |row| row.justify_end())
                                    .child(
                                        div()
                                            .max_w(px(340.0))
                                            .p_3()
                                            .rounded_lg()
                                            .border_1()
                                            .border_color(rgb(if is_user {
                                                theme.primary
                                            } else {
                                                theme.outline_variant
                                            }))
                                            .bg(rgb(if is_user {
                                                theme.primary_container
                                            } else {
                                                theme.surface_container_low
                                            }))
                                            .text_color(rgb(if is_user {
                                                theme.on_primary_container
                                            } else {
                                                theme.on_surface
                                            }))
                                            .font_family(CHAT_FONT_FAMILY)
                                            .child(message.content),
                                    ),
                            )
                        })
                        .into_any_element()
                }),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .pb_2()
                .text_xs()
                .text_color(rgb(theme.on_surface_variant))
                .child(status_line),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .items_end()
                .gap_2()
                .mb_3()
                .p_2()
                .rounded_xl()
                .border_1()
                .border_color(rgb(if focused {
                    theme.primary
                } else {
                    theme.outline_variant
                }))
                .bg(rgb(theme.surface_container_low))
                .child(
                    div()
                        .flex_1()
                        .h(px(50.0))
                        .px_2()
                        .rounded_lg()
                        .bg(rgb(theme.surface))
                        .font_family(CHAT_FONT_FAMILY)
                        .child(
                            TextInput::<ChatApp>::new("chat-input", theme)
                                .value(&draft)
                                .cursor(cursor)
                                .selection(selection)
                                .placeholder("Type a message")
                                .keyboard_type(KeyboardType::Default)
                                .focused(focused)
                                .on_tap_notify(|_event| {
                                    request_focus(InputTarget::Message);
                                    install_keyboard_callback();
                                    gpui_mobile::show_keyboard_with_type(KeyboardType::Default);
                                })
                                .render(cx),
                        ),
                )
                .child(
                    div()
                        .w(px(54.0))
                        .h(px(48.0))
                        .rounded_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(rgb(theme.primary))
                        .text_color(rgb(theme.on_primary))
                        .text_xs()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .child("SEND")
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.send_message(cx);
                            }),
                        ),
                ),
        )
}
