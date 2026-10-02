//! Key/value persistence. Web: `localStorage`. Desktop: one JSON file per key
//! in the user's data directory (the Rust side can't reach the webview's
//! localStorage synchronously, and the workers live on the Rust side).

use serde::{de::DeserializeOwned, Serialize};

const PREFIX: &str = "tabkeeper.";

pub fn load<T: DeserializeOwned + Default>(key: &str) -> T {
    match get_raw(key) {
        Some(s) => serde_json::from_str(&s).unwrap_or_else(|e| {
            eprintln!("tabkeeper: ignoring unreadable stored value '{key}': {e}");
            T::default()
        }),
        None => T::default(),
    }
}

pub fn save<T: Serialize>(key: &str, value: &T) {
    match serde_json::to_string(value) {
        Ok(s) => {
            if let Err(e) = set_raw(key, &s) {
                eprintln!("tabkeeper: failed to save '{key}': {e}");
            }
        }
        Err(e) => eprintln!("tabkeeper: failed to serialize '{key}': {e}"),
    }
}

#[cfg(target_arch = "wasm32")]
fn local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

#[cfg(target_arch = "wasm32")]
pub fn get_raw(key: &str) -> Option<String> {
    local_storage()?.get_item(&format!("{PREFIX}{key}")).ok().flatten()
}

#[cfg(target_arch = "wasm32")]
pub fn set_raw(key: &str, value: &str) -> Result<(), String> {
    local_storage()
        .ok_or("localStorage unavailable")?
        .set_item(&format!("{PREFIX}{key}"), value)
        .map_err(|e| format!("{e:?} (storage quota exceeded?)"))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn data_dir() -> std::path::PathBuf {
    std::env::var_os("TABKEEPER_DATA_DIR")
        .map(Into::into)
        .unwrap_or_else(|| dirs::data_dir().unwrap_or_else(|| ".".into()).join("tabkeeper"))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn get_raw(key: &str) -> Option<String> {
    std::fs::read_to_string(data_dir().join(format!("{PREFIX}{key}.json"))).ok()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn set_raw(key: &str, value: &str) -> Result<(), String> {
    let dir = data_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{PREFIX}{key}.json"));
    let tmp = dir.join(format!(".{PREFIX}{key}.json.tmp"));
    std::fs::write(&tmp, value).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
}
