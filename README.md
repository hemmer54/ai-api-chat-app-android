# API Chat App

A small multi-provider AI chat app. The provider logic is written in Rust and is shared by the desktop/browser harness and the Android client.

## Providers

- OpenAI
- Google Gemini
- Anthropic Claude

## Agent features

`crates/chat-core` contains the provider-independent agent layer:

- Typed conversation messages and provider IDs
- Temperature and output-token controls
- Custom system prompts
- Context limits by message count and character count
- Zed-inspired context compaction: when limits are reached, older turns are summarized by the selected provider and recent turns are preserved

The existing Axum endpoint remains backwards-compatible. It accepts an optional `settings` object in `/api/chat` when agent controls are needed.

## Desktop/browser harness

1. Copy `.env.example` to `.env` and add one or more provider keys.
2. Start the server:

```bash
cargo run -p api-chat-app
```

3. Open <http://127.0.0.1:3000>.

Models can be changed with `OPENAI_MODEL`, `GEMINI_MODEL`, and `ANTHROPIC_MODEL` in `.env`.

## Android client

The Android app uses Rust + GPUI Mobile. Provider API keys are stored with `android-native-keyring-store` and `keyring-core`: ciphertext is kept in app-private Android `SharedPreferences`, encrypted with a key managed by Android Keystore. The `android-activity` runtime initializes the NDK application context required by the credential store.

The first shell is in `mobile/` and includes:

- NativeActivity entry point
- GPUI conversation inbox and dedicated conversation screen
- Conversation titles and previews derived from chat messages
- Provider-specific API model discovery and selection
- Custom model IDs when a provider model is not returned by its discovery endpoint
- Agent settings for generation and context-compaction controls
- Mobile text input handling

The app sends non-streaming requests through `chat-core`, displays assistant responses, reports provider errors, and applies context compaction through the selected provider. Model discovery queries configured providers; custom model IDs are also supported. Saved API keys are restored on launch and can be saved or removed in Agent settings. Edits remain in memory until saved or until leaving settings.

## Privacy and data handling

- No analytics, telemetry, advertising, crash-reporting, or usage-reporting SDKs are included.
- Conversations, model favorites, and agent settings are held in memory only and are discarded when the app process exits.
- API keys are stored as encrypted credentials in Android app-private `SharedPreferences`. The Android Keystore-managed encryption key is separate from the stored ciphertext; API keys are never written in plaintext preferences or files.
- Use **Save key securely** to persist the current provider key. **Remove key** deletes it from the device. Keys are also saved when leaving Agent settings or before an API request.
- Android backup is disabled. The app does not create a local conversation database or conversation export.
- The only app network requests are explicit provider model discovery and completion requests to OpenAI, Google Gemini, or Anthropic.
- Prompts, conversation context, model IDs, and API keys are sent over HTTPS only to the provider selected by the user as required by that provider's API. Keys are decrypted in app memory when used to authorize a request.
- The app does not send data to an API Chat tracking or analytics server. Provider-side retention is governed by the selected provider's policy.
- Keystore-backed storage protects keys at rest, but cannot protect a key while the app is using it on a compromised/rooted device or from malicious code running with the app's privileges. Use provider-side key restrictions, quotas, and rotation where available.
- The development/debug APK is debuggable. Do not use a debug build for long-lived production credentials; produce and distribute a signed, non-debuggable release build for real use.

### Toolchain

- Android SDK/API 37
- Android NDK r27 or newer
- Rust targets `aarch64-linux-android` and `x86_64-linux-android` (for the Windows emulator)
- `cargo-ndk`
- Java 17
- Gradle

### Build on Windows

After Android Studio has installed the SDK and NDK, set these variables if they are not already configured:

```cmd
setx ANDROID_HOME "%LOCALAPPDATA%\Android\Sdk"
setx ANDROID_SDK_ROOT "%LOCALAPPDATA%\Android\Sdk"
```

Open a new terminal and build the optimized Rust shared library. The font environment variable works around an upstream desktop `pkg-config` assumption in the GPUI font dependency during Android cross-compilation:

```bash
set RUST_FONTCONFIG_DLOPEN=1
cargo ndk -t arm64-v8a -P 37 -o mobile/android/gradle/app/src/main/jniLibs build -p api-chat-mobile --release
rustup target add x86_64-linux-android
cargo ndk -t x86_64 -P 37 -o mobile/android/gradle/app/src/main/jniLibs build -p api-chat-mobile --release
```

The second native build is for the usual Windows `x86_64` emulator. Then package the APK:

```bash
cd mobile/android/gradle
gradle clean assembleDebug
```

The APK is written to `mobile/android/gradle/app/build/outputs/apk/debug/`. The first build should remove old files from `mobile/android/gradle/app/src/main/jniLibs/arm64-v8a/` if the native library hash changes.

## Structure

- `src/main.rs` — Axum desktop/testing server
- `static/index.html` — browser UI
- `crates/chat-core/` — reusable provider clients and agent behavior
- `mobile/src/` — Rust/GPUI Android client
- `mobile/android/gradle/` — Android APK packaging

- `FSEX302.ttf` — bundled Fixedsys Excelsior font used for the composer and conversation/AI text
- `jh_fallout-webfont.ttf` — bundled JH Fallout font used for general-purpose mobile UI text

The provider implementations were written independently. Zed and GPUI Mobile are used as architectural references; Zed source is not copied into this project.
