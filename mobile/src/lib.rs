mod app;
mod chat_list_screen;
mod chat_screen;
mod secure_keys;
mod settings_screen;

#[cfg(target_os = "android")]
mod android;

pub use app::ChatApp;
