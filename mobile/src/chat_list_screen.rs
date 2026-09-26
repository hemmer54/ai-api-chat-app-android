use crate::{
    app::{midnight_theme, CHAT_FONT_FAMILY},
    ChatApp,
};
use gpui::{div, prelude::*, px, rgb, Context, MouseButton};

pub fn render(app: &ChatApp, cx: &mut Context<ChatApp>) -> impl IntoElement {
    let theme = midnight_theme();
    let conversations = app.conversations.clone();
    let provider = app.settings.provider.display_name();
    let model = app
        .provider_config
        .settings(app.settings.provider)
        .model
        .clone();

    div()
        .flex()
        .flex_col()
        .flex_1()
        .bg(rgb(theme.surface))
        .font_family(CHAT_FONT_FAMILY)
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .px_4()
                .py_3()
                .border_b_1()
                .border_color(rgb(theme.outline_variant))
                .child(
                    div()
                        .text_xs()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(rgb(theme.primary))
                        .child("THREADS"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(theme.on_surface_variant))
                        .child(format!("{} active", conversations.len())),
                ),
        )
        .when(conversations.is_empty(), |list| {
            list.child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .items_center()
                    .justify_center()
                    .px_4()
                    .child(
                        div()
                            .w_full()
                            .max_w(px(560.0))
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_3()
                            .p_4()
                            .rounded_xl()
                            .border_1()
                            .border_color(rgb(theme.outline_variant))
                            .bg(rgb(theme.surface_container_low))
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(rgb(theme.tertiary))
                                    .child("NEW THREAD"),
                            )
                            .child(
                                div()
                                    .text_xl()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_center()
                                    .child("Start with a clean context"),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(theme.on_surface_variant))
                                    .text_center()
                                    .child("Your messages stay on this device until you send them to the selected provider."),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .justify_between()
                                    .p_3()
                                    .rounded_lg()
                                    .bg(rgb(theme.primary_container))
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_1()
                                            .child(div().text_xs().text_color(rgb(theme.on_primary_container)).child(provider))
                                            .child(div().text_sm().text_color(rgb(theme.on_primary_container)).child(model.clone())),
                                    )
                                    .child(
                                        div()
                                            .px_3()
                                            .py_2()
                                            .rounded_lg()
                                            .bg(rgb(theme.primary))
                                            .text_color(rgb(theme.on_primary))
                                            .child("Open")
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(|this, _, _, cx| {
                                                    this.start_new_conversation();
                                                    cx.notify();
                                                }),
                                            ),
                                    ),
                            ),
                    ),
            )
        })
        .when(!conversations.is_empty(), |list| {
            list.child(
                div()
                    .px_4()
                    .py_3()
                    .text_xs()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(rgb(theme.on_surface_variant))
                    .child("RECENT CHATS"),
            )
            .children(conversations.iter().map(|conversation| {
                let id = conversation.id;
                let avatar = conversation
                    .title
                    .chars()
                    .next()
                    .unwrap_or('C')
                    .to_uppercase()
                    .to_string();
                let avatar_color = match id % 4 {
                    0 => 0x4F8A5B,
                    1 => 0x3E6B49,
                    2 => 0x6A8459,
                    _ => 0x6C6658,
                };
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_3()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(theme.outline_variant))
                    .child(
                        div()
                            .w(px(52.0))
                            .h(px(52.0))
                            .rounded_full()
                            .bg(rgb(avatar_color))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .text_lg()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(rgb(theme.on_primary))
                                    .child(avatar),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .gap_1()
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .text_base()
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .child(conversation.title.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgb(theme.on_surface_variant))
                                            .child(if conversation.messages.is_empty() {
                                                "New"
                                            } else {
                                                "Recent"
                                            }),
                                    ),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(theme.on_surface_variant))
                                    .child(conversation.preview.clone()),
                            ),
                    )
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            this.open_conversation(id);
                            cx.notify();
                        }),
                    )
            }))
        })
}
