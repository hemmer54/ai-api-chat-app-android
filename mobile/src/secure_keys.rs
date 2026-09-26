use chat_core::ProviderId;

#[cfg(target_os = "android")]
fn credential(provider: ProviderId) -> Result<keyring_core::Entry, String> {
    use android_native_keyring_store::Store;
    use keyring_core::api::CredentialStoreApi;

    let store = Store::new().map_err(|error| error.to_string())?;
    store
        .build(
            "dev.apichat.mobile.provider-api-key",
            provider.as_str(),
            None,
        )
        .map_err(|error| error.to_string())
}

#[cfg(target_os = "android")]
pub fn load_api_key(provider: ProviderId) -> Result<Option<String>, String> {
    match credential(provider)?.get_password() {
        Ok(key) => Ok(Some(key)),
        Err(keyring_core::Error::NoEntry) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

#[cfg(not(target_os = "android"))]
pub fn load_api_key(_provider: ProviderId) -> Result<Option<String>, String> {
    Ok(None)
}

#[cfg(target_os = "android")]
pub fn save_api_key(provider: ProviderId, key: Option<&str>) -> Result<(), String> {
    let credential = credential(provider)?;
    match key.filter(|key| !key.is_empty()) {
        Some(key) => credential
            .set_password(key)
            .map_err(|error| error.to_string()),
        None => match credential.delete_credential() {
            Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(()),
            Err(error) => Err(error.to_string()),
        },
    }
}

#[cfg(not(target_os = "android"))]
pub fn save_api_key(_provider: ProviderId, _key: Option<&str>) -> Result<(), String> {
    Err("secure API key storage is only available on Android".into())
}
