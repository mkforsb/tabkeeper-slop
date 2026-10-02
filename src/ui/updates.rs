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
                div { key: "{e.id}", class: if e.read { "panel event" } else { "panel event unread" },
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
                            for item in e.items.iter() { ItemRow { key: "{item.id}", item: item.clone(), fresh: true } }
                        }
                    }
                }
            }
        }
    }
}
