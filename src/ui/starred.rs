use dioxus::prelude::*;
use tabkeeper::app;
use tabkeeper::model::StarredItem;

use super::components::*;
use super::state::*;
use super::{fmt_ago, fmt_time, Route};

#[component]
pub fn Starred() -> Element {
    let starred = STARRED();
    let mut drag = use_signal(|| None::<Drag>);
    let from = drag.read().as_ref().and_then(|d| starred.iter().position(|s| s.id == d.id));
    let over = drag.read().as_ref().and_then(|d| d.over);
    // As on the Interests page: the source row is highlighted, the target row
    // gets a line on the side the item will land.
    let row_class = move |k: usize| match (from, over) {
        (Some(f), _) if f == k => "drag-source",
        (Some(f), Some(o)) if o == k && o < f => "drop-above",
        (Some(f), Some(o)) if o == k && o > f => "drop-below",
        _ => "",
    };

    rsx! {
        div { class: "page-head", h1 { "Starred" } }
        if starred.is_empty() {
            div { class: "panel empty",
                p { class: "muted", "No starred items yet. Click ★ next to an item on the Dashboard or under Updates to keep it here." }
            }
        } else {
            div { class: "panel table-wrap",
                table {
                    class: if drag.read().is_some() { "table dragging" } else { "table" },
                    onmouseup: move |_| {
                        if let Some(Drag { id, over: Some(to) }) = drag.take() {
                            app::move_starred(&mut STARRED.write(), &id, to);
                        }
                    },
                    // Releasing outside the table cancels the drag.
                    onmouseleave: move |_| if drag.peek().is_some() { drag.set(None) },
                    thead {
                        tr {
                            th { "" }
                            th { "Item" }
                            th { "Interest" }
                            th { "Starred" }
                        }
                    }
                    tbody {
                        for (k, s) in starred.into_iter().enumerate() {
                            Row { key: "{s.id}", starred: s, index: k, drag, class: row_class(k) }
                        }
                    }
                }
            }
            p { class: "muted small", "Drag ⠿ to reorder." }
        }
    }
}

#[component]
fn Row(starred: StarredItem, index: usize, drag: Signal<Option<Drag>>, class: &'static str) -> Element {
    let now = NOW();
    // Deleted interests are shown by the name they had when starred.
    let interest = interest(&starred.interest_id);
    let id_drag = starred.id.clone();

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
            td { ItemRow { item: starred.item.clone(), interest_id: starred.interest_id.clone() } }
            td {
                match interest {
                    Some(i) => rsx! { Link { to: Route::Editor { id: i.id.clone() }, "{i.name}" } },
                    None => rsx! { span { class: "muted", "{starred.interest_name}" } },
                }
            }
            td { class: "muted small nowrap", title: "{fmt_time(starred.starred_at)}", "{fmt_ago(Some(starred.starred_at), now)}" }
        }
    }
}
