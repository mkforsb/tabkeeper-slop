use dioxus::prelude::*;

use super::components::ItemRow;
use super::state::*;
use super::{fmt_ago, fmt_time, Route};

#[component]
pub fn Updates() -> Element {
    let events = EVENTS();
    let now = NOW();
    let mut confirm_clear = use_signal(|| false);
    let unread = unread_count(None);
    // The update clicked in the Dashboard's Recent updates, if that's how we
    // got here: scrolled to and highlighted.
    let target = use_hook(|| SHOW_EVENT.write().take());

    rsx! {
        div { class: "page-head",
            h1 { "Updates" }
            div { class: "actions",
                button { class: "btn", disabled: unread == 0, onclick: move |_| mark_read(None), "Mark all read" }
                if confirm_clear() {
                    button { class: "btn btn-danger", onclick: move |_| { EVENTS.write().clear(); confirm_clear.set(false); }, "Clear history" }
                    button { class: "btn", onclick: move |_| confirm_clear.set(false), "Cancel" }
                } else {
                    button { class: "btn", disabled: events.is_empty(), onclick: move |_| confirm_clear.set(true), "Clear…" }
                }
            }
        }
        if events.is_empty() {
            div { class: "panel empty", p { class: "muted", "No updates caught yet." } }
        }
        div { class: "timeline",
            for e in events {
                div {
                    key: "{e.id}",
                    class: format!(
                        "panel event{}{}",
                        if e.read { "" } else { " unread" },
                        if target.as_deref() == Some(e.id.as_str()) { " target" } else { "" },
                    ),
                    onmounted: {
                        let scroll = target.as_deref() == Some(e.id.as_str());
                        move |m: MountedEvent| async move {
                            if scroll {
                                let _ = m.scroll_to(ScrollBehavior::Instant).await;
                            }
                        }
                    },
                    div { class: "event-head",
                        Link { class: "event-name", to: Route::Editor { id: e.interest_id.clone() }, "{e.interest_name}" }
                        span { class: "muted small", title: "{fmt_time(e.at)}", "{fmt_ago(Some(e.at), now)}" }
                        if !e.read {
                            button {
                                class: "btn btn-small",
                                onclick: {
                                    let id = e.id.clone();
                                    move |_| if let Some(ev) = EVENTS.write().iter_mut().find(|x| x.id == id) { ev.read = true; }
                                },
                                "Mark read"
                            }
                        }
                    }
                    p { "{e.summary}" }
                    if !e.items.is_empty() {
                        div { class: "items",
                            for item in e.items.iter() { ItemRow { key: "{item.id}", item: item.clone(), fresh: true, interest_id: e.interest_id.clone() } }
                        }
                    }
                }
            }
        }
    }
}
