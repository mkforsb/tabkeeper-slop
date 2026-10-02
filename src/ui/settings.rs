use dioxus::prelude::*;

use super::state::*;

#[component]
pub fn SettingsPage() -> Element {
    let s = SETTINGS();
    let mut export = use_signal(String::new);
    let mut import = use_signal(String::new);
    let mut message = use_signal(|| None::<String>);
    let mut permission = use_signal(tabkeeper::notify::permission);
    let mut confirm_reset = use_signal(|| false);
    let is_web = cfg!(target_arch = "wasm32");
    // Text fields are uncontrolled (see the editor); bump to refill them from state.
    let mut rev = use_signal(|| 0u32);

    rsx! {
        div { class: "page-head", h1 { "Settings" } }

        for r in [rev()] {
            div { key: "{r}", class: "panel form",
                h2 { "Refreshing" }
                div { class: "form-row",
                    label { class: "narrow", "Concurrent refreshes"
                        input { r#type: "number", min: "1", max: "16", initial_value: "{s.concurrency}",
                            oninput: move |e| if let Ok(v) = e.value().parse::<u32>() { SETTINGS.write().concurrency = v.clamp(1, 16); },
                            // Show the clamped value once editing is done.
                            onchange: move |e| if e.value().parse().ok() != Some(SETTINGS.peek().concurrency) { rev += 1 } }
                    }
                    label { class: "narrow", "Updates kept in history"
                        input { r#type: "number", min: "10", initial_value: "{s.max_events}",
                            oninput: move |e| if let Ok(v) = e.value().parse::<u32>() { SETTINGS.write().max_events = v.max(10); },
                            onchange: move |e| if e.value().parse().ok() != Some(SETTINGS.peek().max_events) { rev += 1 } }
                    }
                }
                label { class: "check",
                    input { r#type: "checkbox", checked: s.paused, onchange: move |e| SETTINGS.write().paused = e.checked() }
                    "Pause all background refreshing"
                }
                if is_web {
                    label { "CORS proxy"
                        input { initial_value: "{s.cors_proxy}", placeholder: "https://your-proxy.example/?url={{url}}",
                            oninput: move |e| SETTINGS.write().cors_proxy = e.value() }
                    }
                    p { class: "small muted",
                        "Browsers block reading other sites unless they allow it (CORS), and most sites don't. In the web build every fetch goes through this proxy if set. "
                        "{{url}} is replaced by the encoded target URL; without it the target is appended. "
                        "Public proxies see everything you fetch, so prefer one you run yourself. The desktop app doesn't need a proxy."
                    }
                }
            }

            div { class: "panel form",
                h2 { "Notifications" }
                label { class: "check",
                    input { r#type: "checkbox", checked: s.notifications, onchange: move |e| SETTINGS.write().notifications = e.checked() }
                    "Show desktop notifications for updates (per-interest toggle in the editor)"
                }
                div { class: "actions",
                    if is_web {
                        span { class: "small muted", "Browser permission: {permission}" }
                        button { class: "btn btn-small",
                            onclick: move |_| async move {
                                match tabkeeper::notify::request_permission().await {
                                    Ok(p) => permission.set(p),
                                    Err(e) => message.set(Some(e)),
                                }
                            },
                            "Request permission"
                        }
                    }
                    button { class: "btn btn-small",
                        onclick: move |_| async move {
                            let r = tabkeeper::notify::send("Tabkeeper".into(), "Notifications are working.".into()).await;
                            message.set(Some(match r { Ok(()) => "Test notification sent.".into(), Err(e) => format!("Notification failed: {e}") }));
                        },
                        "Send test notification"
                    }
                }
                if let Some(e) = NOTIFY_ERROR() { div { class: "error-box small", "Last notification failed: {e}" } }
            }

            div { class: "panel form",
                h2 { "Data" }
                p { class: "small muted",
                    if is_web { "Everything is stored in this browser's localStorage. Export regularly if it matters to you." }
                    else { "Stored as JSON files in {data_dir()}." }
                }
                div { class: "actions",
                    button { class: "btn btn-small", onclick: move |_| export.set(export_json()), "Export" }
                }
                if !export().is_empty() {
                    textarea { class: "code", rows: 8, readonly: true, value: "{export}" }
                }
                label { "Import (replaces all current data)"
                    textarea { class: "code", rows: 4, placeholder: "Paste an export here", oninput: move |e| import.set(e.value()) }
                }
                div { class: "actions",
                    button { class: "btn btn-small", disabled: import().trim().is_empty(),
                        onclick: move |_| {
                            match import_json(&import()) {
                                Ok(n) => { message.set(Some(format!("Imported {n} interests."))); import.set(String::new()); rev += 1; }
                                Err(e) => message.set(Some(format!("Import failed: {e}"))),
                            }
                        },
                        "Import"
                    }
                    if confirm_reset() {
                        button { class: "btn btn-small btn-danger",
                            onclick: move |_| { let _ = import_json("{}"); confirm_reset.set(false); rev += 1; message.set(Some("All data erased.".into())); },
                            "Erase everything" }
                        button { class: "btn btn-small", onclick: move |_| confirm_reset.set(false), "Cancel" }
                    } else {
                        button { class: "btn btn-small", onclick: move |_| confirm_reset.set(true), "Reset all data…" }
                    }
                }
            }
        }

        if let Some(m) = message() { div { class: "toast", onclick: move |_| message.set(None), "{m}" } }
    }
}

fn data_dir() -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        tabkeeper::storage::data_dir().display().to_string()
    }
    #[cfg(target_arch = "wasm32")]
    {
        String::new()
    }
}
