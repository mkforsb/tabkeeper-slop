use dioxus::prelude::*;
use tabkeeper::app;
use tabkeeper::model::Interest;

use super::components::*;
use super::state::*;
use super::worker::start_run;
use super::{fmt_count, Route};

#[component]
pub fn Interests() -> Element {
    let interests = INTERESTS();
    let mut drag = use_signal(|| None::<Drag>);
    let from = drag.read().as_ref().and_then(|d| interests.iter().position(|i| i.id == d.id));
    let over = drag.read().as_ref().and_then(|d| d.over);
    // The source row is highlighted; the target row gets a line on the side
    // the interest will land.
    let row_class = move |k: usize| match (from, over) {
        (Some(f), _) if f == k => "drag-source",
        (Some(f), Some(o)) if o == k && o < f => "drop-above",
        (Some(f), Some(o)) if o == k && o > f => "drop-below",
        _ => "",
    };

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
                table {
                    class: if drag.read().is_some() { "table dragging" } else { "table" },
                    onmouseup: move |_| {
                        if let Some(Drag { id, over: Some(to) }) = drag.take() {
                            app::move_interest(&mut INTERESTS.write(), &id, to);
                        }
                    },
                    // Releasing outside the table cancels the drag.
                    onmouseleave: move |_| if drag.peek().is_some() { drag.set(None) },
                    thead {
                        tr {
                            th { "" }
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
                        for (k, i) in interests.into_iter().enumerate() {
                            Row { key: "{i.id}", interest: i, index: k, drag, class: row_class(k) }
                        }
                    }
                }
            }
            p { class: "muted small", "Drag ⠿ to reorder." }
        }
    }
}

#[component]
fn Row(interest: Interest, index: usize, drag: Signal<Option<Drag>>, class: &'static str) -> Element {
    let st = STATES.read().get(&interest.id).cloned().unwrap_or_default();
    let running = RUNNING.read().contains(&interest.id);
    let mut confirm = use_signal(|| false);
    let (id_run, id_toggle, id_del) = (interest.id.clone(), interest.id.clone(), interest.id.clone());
    let enabled = interest.enabled;
    let id_drag = interest.id.clone();

    rsx! {
        tr {
            class,
            onmouseenter: move |_| {
                if drag.peek().is_some() {
                    if let Some(d) = drag.write().as_mut() { d.over = Some(index); }
                }
            },
            td {
                class: "grip",
                title: "Drag to reorder",
                onmousedown: move |e| {
                    e.prevent_default();
                    drag.set(Some(Drag { id: id_drag.clone(), over: None }));
                },
                "⠿"
            }
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
            td {
                div { class: "row-actions",
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
}
