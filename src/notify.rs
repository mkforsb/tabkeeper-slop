//! Desktop notifications: notify-rust natively, the Notification API on web.

#[cfg(not(target_arch = "wasm32"))]
pub async fn send(title: String, body: String) -> Result<(), String> {
    tabkeeper::platform::run_blocking(move || {
        notify_rust::Notification::new()
            .appname("Tabkeeper")
            .summary(&title)
            .body(&body)
            .show()
            .map(|_| ())
            .map_err(|e| e.to_string())
    })
    .await
}

#[cfg(target_arch = "wasm32")]
pub async fn send(title: String, body: String) -> Result<(), String> {
    use web_sys::{Notification, NotificationOptions, NotificationPermission};
    if Notification::permission() != NotificationPermission::Granted {
        return Err("notification permission not granted (see Settings)".into());
    }
    let opts = NotificationOptions::new();
    opts.set_body(&body);
    Notification::new_with_options(&title, &opts).map(|_| ()).map_err(|e| format!("{e:?}"))
}

/// Asks the browser for notification permission. Native platforms need none.
#[cfg(target_arch = "wasm32")]
pub async fn request_permission() -> Result<String, String> {
    let promise = web_sys::Notification::request_permission().map_err(|e| format!("{e:?}"))?;
    let v = wasm_bindgen_futures::JsFuture::from(promise).await.map_err(|e| format!("{e:?}"))?;
    Ok(v.as_string().unwrap_or_default())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn request_permission() -> Result<String, String> {
    Ok("granted".into())
}

pub fn permission() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        match web_sys::Notification::permission() {
            web_sys::NotificationPermission::Granted => "granted".into(),
            web_sys::NotificationPermission::Denied => "denied".into(),
            _ => "default".into(),
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        "granted".into()
    }
}
