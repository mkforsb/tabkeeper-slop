use dioxus::prelude::*;
use tabkeeper::app;
use tabkeeper::model::Interest;

use super::components::*;
use super::state::*;
use super::worker::start_run;
use super::{fmt_ago, fmt_count, Route};

#[component]
pub fn Dashboard() -> Element {
    let interests = INTERESTS();
    let events = EVENTS.read().iter().take(8).cloned().collect::<Vec<_>>();
    let now = NOW();
    let day_ago = now - 86_400_000;
    let updates_24h = EVENTS.read().iter().filter(|e| e.at >= day_ago).count() as u64;
    let failing = {
        let states = STATES.read();
        interests.iter().filter(|i| i.enabled && states.get(&i.id).is_some_and(|s| s.last_error.is_some())).count() as u64
    };
    let enabled = interests.iter().filter(|i| i.enabled).count() as u64;
    let sys = SYSTEM();
    let (has_interests, total) = (!interests.is_empty(), interests.len());
    let cards: Vec<Interest> = app::dashboard_cards(&interests, &SETTINGS.read().dashboard_order).into_iter().cloned().collect();
    let card_ids: Vec<String> = cards.iter().map(|i| i.id.clone()).collect();

    let mut drag = use_signal(|| None::<Drag>);
    let from = drag.read().as_ref().and_then(|d| card_ids.iter().position(|id| *id == d.id));
    let over = drag.read().as_ref().and_then(|d| d.over);
    let card_class = move |k: usize| match (from, over) {
        (Some(f), _) if f == k => "drag-source",
        (Some(_), Some(o)) if o == k => "drop-target",
        _ => "",
    };

    rsx! {
        div { class: "page-head",
            h1 { "Dashboard" }
            div { class: "actions",
                button {
                    class: "btn",
                    disabled: enabled == 0,
                    onclick: move |_| for i in INTERESTS.read().iter().filter(|i| i.enabled) { start_run(i.id.clone()) },
                    "Refresh all"
                }
                button {
                    class: "btn",
                    disabled: enabled < 2,
                    onclick: move |_| app::shuffle_cards(&mut SETTINGS.write().dashboard_order, &INTERESTS.read()),
                    "Shuffle"
                }
                Link { class: "btn btn-primary", to: Route::Editor { id: "new".into() }, "+ New interest" }
            }
        }

        div { class: "tiles",
            Tile { label: "Interests watched", value: fmt_count(enabled), sub: format!("{total} total") }
            Tile { label: "Updates in last 24h", value: fmt_count(updates_24h), sub: format!("{} unread", unread_count(None)) }
            Tile { label: "Refreshes", value: fmt_count(sys.refreshes), sub: format!("{} failed", fmt_count(sys.failures)) }
            Tile {
                label: "Failing now",
                value: fmt_count(failing),
                sub: if failing > 0 { "✕ needs attention".to_string() } else { "✓ all healthy".to_string() },
            }
        }

        if !has_interests {
            EmptyState {}
        } else {
            div { class: "dash",
                div {
                    class: if drag.read().is_some() { "cards dragging" } else { "cards" },
                    onmouseup: move |_| {
                        if let Some(Drag { id, over: Some(to) }) = drag.take() {
                            if let Some(target) = card_ids.get(to) {
                                app::move_card(&mut SETTINGS.write().dashboard_order, &INTERESTS.read(), &id, target);
                            }
                        }
                    },
                    // Releasing outside the cards cancels the drag.
                    onmouseleave: move |_| if drag.peek().is_some() { drag.set(None) },
                    if enabled == 0 {
                        div { class: "panel empty",
                            p { class: "muted", "All interests are disabled. Enable them on the " Link { to: Route::Interests {}, "Interests" } " page to see them here." }
                        }
                    }
                    // Disabled interests are only listed on the Interests page.
                    for (k, i) in cards.into_iter().enumerate() {
                        InterestCard { key: "{i.id}", interest: i, index: k, drag, class: card_class(k) }
                    }
                }
                aside { class: "feed panel",
                    div { class: "panel-head",
                        h2 { "Recent updates" }
                        Link { class: "small", to: Route::Updates {}, "All →" }
                    }
                    if events.is_empty() {
                        p { class: "muted small", "Nothing caught yet. The first refresh of an interest records a baseline; changes after that show up here." }
                    }
                    for e in events {
                        div { key: "{e.id}", class: if e.read { "feed-item" } else { "feed-item unread" },
                            div { class: "feed-meta",
                                Link { to: Route::Editor { id: e.interest_id.clone() }, "{e.interest_name}" }
                                span { class: "muted small", "{fmt_ago(Some(e.at), now)}" }
                            }
                            div { class: "small", "{e.summary}" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Tile(label: String, value: String, sub: String) -> Element {
    rsx! {
        div { class: "tile",
            div { class: "tile-label", "{label}" }
            div { class: "tile-value", "{value}" }
            div { class: "tile-sub", "{sub}" }
        }
    }
}

#[component]
fn InterestCard(interest: Interest, index: usize, drag: Signal<Option<Drag>>, class: &'static str) -> Element {
    let st = STATES.read().get(&interest.id).cloned().unwrap_or_default();
    let unread = unread_count(Some(&interest.id));
    let image = display_image(&interest);
    let id = interest.id.clone();
    let id2 = interest.id.clone();
    let id3 = interest.id.clone();
    let id_drag = interest.id.clone();
    let reversed = interest.reverse_order;
    let running = RUNNING.read().contains(&interest.id);

    rsx! {
        article {
            class: "card {class}",
            onmouseenter: move |_| {
                if drag.peek().is_some() {
                    if let Some(d) = drag.write().as_mut() { d.over = Some(index); }
                }
            },
            header { class: "card-head",
                span {
                    class: "grip",
                    title: "Drag to reorder",
                    onmousedown: move |e| {
                        e.prevent_default();
                        drag.set(Some(Drag { id: id_drag.clone(), over: None }));
                    },
                    "⠿"
                }
                Avatar { src: image, name: interest.name.clone() }
                div { class: "card-title",
                    Link { to: Route::Editor { id: interest.id.clone() }, h3 { "{interest.name}" } }
                    Timing { interest: interest.clone() }
                }
                if unread > 0 {
                    button {
                        class: "badge",
                        title: "Mark as read",
                        onclick: move |_| mark_read(Some(&id2)),
                        "{unread} new"
                    }
                }
            }
            div { class: "card-body",
                if let Some(err) = &st.last_error {
                    div { class: "error-box small", "✕ {err}" }
                }
                match &st.last_output {
                    Some(out) => rsx! { OutputView { output: out.clone(), limit: 4, show_image: false, reverse: reversed } },
                    None if running => rsx! { p { class: "muted small", "Running for the first time…" } },
                    None if st.last_error.is_none() => rsx! { p { class: "muted small", "Not checked yet." } },
                    None => rsx! {},
                }
            }
            footer { class: "card-foot",
                StatusPill { interest: interest.clone() }
                div { class: "actions",
                    button { class: "btn btn-small", title: "Reverse item order", onclick: move |_| set_reversed(&id3, !reversed), "↑↓" }
                    button { class: "btn btn-small", disabled: running, onclick: move |_| start_run(id.clone()), "Refresh" }
                    Link { class: "btn btn-small", to: Route::Editor { id: interest.id.clone() }, "Edit" }
                }
            }
        }
    }
}

#[component]
fn EmptyState() -> Element {
    rsx! {
        div { class: "empty panel",
            h2 { "Nothing to keep tabs on yet" }
            p { "An interest is a script that fetches a page and extracts what matters. Tabkeeper runs it periodically and tells you when something new shows up." }
            div { class: "actions",
                Link { class: "btn btn-primary", to: Route::Editor { id: "new".into() }, "Create an interest" }
                button { class: "btn", onclick: move |_| add_examples(), "Add the example interests" }
            }
        }
    }
}

pub fn add_examples() {
    for i in app::example_interests() {
        upsert_interest(i);
    }
}
