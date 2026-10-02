use dioxus::prelude::*;
use tabkeeper::model::Interest;

use super::components::*;
use super::state::*;
use super::worker::start_run;
use super::{fmt_count, Route};

#[component]
pub fn Interests() -> Element {
    let interests = INTERESTS();
    rsx! {
        div { class: "page-head",
            h1 { "Interests" }
            div { class: "actions",
                Link { class: "btn btn-primary", to: Route::Editor { id: "new".into() }, "+ New interest" }
            }
        }
        if interests.is_empty() {
            div { class: "panel empty",
                p { "No interests yet." }
                button { class: "btn", onclick: move |_| super::dashboard::add_examples(), "Add the example interests" }
            }
        } else {
            div { class: "panel table-wrap",
                table { class: "table",
                    thead {
                        tr {
                            th { "" }
                            th { "Name" }
                            th { "Status" }
                            th { "Every" }
                            th { class: "num", "Refreshes" }
                            th { class: "num", "Failures" }
                            th { class: "num", "Updates" }
                            th { "Enabled" }
                            th { "" }
                        }
                    }
                    tbody {
                        for i in interests { Row { key: "{i.id}", interest: i.clone() } }
                    }
                }
            }
        }
    }
}

#[component]
fn Row(interest: Interest) -> Element {
    let st = STATES.read().get(&interest.id).cloned().unwrap_or_default();
    let running = RUNNING.read().contains(&interest.id);
    let mut confirm = use_signal(|| false);
    let (id_run, id_toggle, id_del) = (interest.id.clone(), interest.id.clone(), interest.id.clone());
    let enabled = interest.enabled;

    rsx! {
        tr {
            td { Avatar { src: display_image(&interest), name: interest.name.clone(), class: "avatar avatar-small" } }
            td {
                Link { to: Route::Editor { id: interest.id.clone() }, "{interest.name}" }
                div { Timing { interest: interest.clone() } }
            }
            td { StatusPill { interest: interest.clone() } }
            td { class: "muted", "{interest.interval_mins} min" }
            td { class: "num", "{fmt_count(st.stats.refreshes)}" }
            td { class: "num", "{fmt_count(st.stats.failures)}" }
            td { class: "num", "{fmt_count(st.stats.updates)}" }
            td {
                input {
                    r#type: "checkbox",
                    checked: enabled,
                    onchange: move |e| {
                        if let Some(i) = INTERESTS.write().iter_mut().find(|i| i.id == id_toggle) { i.enabled = e.checked(); }
                    },
                }
            }
            td { class: "row-actions",
                button { class: "btn btn-small", disabled: running, onclick: move |_| start_run(id_run.clone()), "Run" }
                Link { class: "btn btn-small", to: Route::Editor { id: interest.id.clone() }, "Edit" }
                if confirm() {
                    button { class: "btn btn-small btn-danger", onclick: move |_| delete_interest(&id_del), "Confirm" }
                    button { class: "btn btn-small", onclick: move |_| confirm.set(false), "Cancel" }
                } else {
                    button { class: "btn btn-small", onclick: move |_| confirm.set(true), "Delete" }
                }
            }
        }
    }
}
