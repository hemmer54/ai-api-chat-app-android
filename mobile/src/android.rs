use crate::app::open_main_window;
use gpui::Application;
use gpui_mobile::android::jni;

#[no_mangle]
pub fn android_main(app: android_activity::AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("api-chat-app"),
    );
    jni::install_panic_hook();
    let _platform = jni::init_platform(&app);
    let Some(shared) = jni::shared_platform() else {
        log::error!("Android platform could not be initialized");
        return;
    };

    Application::with_platform(shared.into_rc()).run(|cx| {
        open_main_window(cx);
    });
}
