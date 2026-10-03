use dioxus::prelude::*;
use tabkeeper::app;
use tabkeeper::model::StarredItem;

use super::components::*;
use super::state::*;
use super::{fmt_ago, fmt_time, Route};

/// Starred items as a list in its own order, or as tiles per interest.
#[component]
pub fn Starred() -> Element {
    let empty = STARRED.read().is_empty();
    let tiles = SETTINGS.read().starred_tiles;
    let tile_count = app::starred_tiles(&STARRED.read(), &[], &[]).len();

    rsx! {
        div { class: "page-head",
            h1 { "Starred" }
            div { class: "actions",
                if tiles {
                    button {
                        class: "btn",
                        disabled: tile_count < 2,
                        onclick: move |_| app::shuffle_starred_tiles(&mut SETTINGS.write().starred_tile_order, &STARRED.read()),
                        "Shuffle"
                    }
                }
                div { class: "segmented", role: "group", "aria-label": "View",
                    button {
                        class: if !tiles { "active" },
                        "aria-pressed": !tiles,
                        onclick: move |_| SETTINGS.write().starred_tiles = false,
                        "☰ List"
                    }
                    button {
                        class: if tiles { "active" },
                        "aria-pressed": tiles,
                        onclick: move |_| SETTINGS.write().starred_tiles = true,
                        "▦ Tiles"
                    }
                }
            }
        }
        if empty {
            div { class: "panel empty",
                p { class: "muted", "No starred items yet. Click ★ next to an item on the Dashboard or under Updates to keep it here." }
            }
        } else if tiles {
            StarredTiles {}
        } else {
            StarredList {}
        }
    }
}

#[component]
fn StarredList() -> Element {
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

/// One tile per interest, laid out like the Dashboard's cards. Both the tiles
/// and the items in each can be dragged to reorder, in an order of their own.
#[component]
fn StarredTiles() -> Element {
    let tiles: Vec<(String, String, Vec<StarredItem>)> = {
        let (starred, settings) = (STARRED.read(), SETTINGS.read());
        app::starred_tiles(&starred, &settings.starred_tile_order, &settings.starred_item_order)
            .into_iter()
            .map(|t| (t.interest_id.to_string(), t.interest_name.to_string(), t.items.into_iter().cloned().collect()))
            .collect()
    };
    let tile_ids: Vec<String> = tiles.iter().map(|t| t.0.clone()).collect();

    // A tile being dragged, and an item being dragged within its tile.
    let mut drag = use_signal(|| None::<Drag>);
    let mut item_drag = use_signal(|| None::<Drag>);
    let from = drag.read().as_ref().and_then(|d| tile_ids.iter().position(|id| *id == d.id));
    let over = drag.read().as_ref().and_then(|d| d.over);
    let tile_class = move |k: usize| match (from, over) {
        (Some(f), _) if f == k => "drag-source",
        (Some(_), Some(o)) if o == k => "drop-target",
        _ => "",
    };
    let dragging = drag.read().is_some() || item_drag.read().is_some();

    rsx! {
        div {
            class: if dragging { "cards dragging" } else { "cards" },
            onmouseup: move |_| {
                if let Some(Drag { id, over: Some(to) }) = drag.take() {
                    if let Some(target) = tile_ids.get(to) {
                        app::move_starred_tile(&mut SETTINGS.write().starred_tile_order, &STARRED.read(), &id, target);
                    }
                }
                // An item released outside its tile stays put.
                item_drag.set(None);
            },
            // Releasing outside the tiles cancels the drag.
            onmouseleave: move |_| {
                if drag.peek().is_some() { drag.set(None) }
                if item_drag.peek().is_some() { item_drag.set(None) }
            },
            for (k, (interest_id, name, items)) in tiles.into_iter().enumerate() {
                StarredTile { key: "{interest_id}", interest_id, name, items, index: k, drag, item_drag, class: tile_class(k) }
            }
        }
        p { class: "muted small", "Drag ⠿ to reorder the tiles, and the items within each tile." }
    }
}

#[component]
fn StarredTile(
    interest_id: String,
    /// The interest's name when its items were starred, if it's been deleted since.
    name: String,
    items: Vec<StarredItem>,
    index: usize,
    drag: Signal<Option<Drag>>,
    item_drag: Signal<Option<Drag>>,
    class: &'static str,
) -> Element {
    let interest = interest(&interest_id);
    let image = interest.as_ref().map(display_image).unwrap_or_default();
    let name = interest.as_ref().map_or(name, |i| i.name.clone());
    let item_ids: Vec<String> = items.iter().map(|s| s.id.clone()).collect();
    // Item drags stay within their tile.
    let from = item_drag.read().as_ref().and_then(|d| item_ids.iter().position(|id| *id == d.id));
    let over = from.and(item_drag.read().as_ref().and_then(|d| d.over));
    let item_class = move |j: usize| match (from, over) {
        (Some(f), _) if f == j => "tile-item drag-source",
        (Some(f), Some(o)) if o == j && o < f => "tile-item drop-above",
        (Some(f), Some(o)) if o == j && o > f => "tile-item drop-below",
        _ => "tile-item",
    };
    let id_drag = interest_id.clone();
    let ids_drop = item_ids;

    rsx! {
        article {
            class: "card {class}",
            onmouseenter: move |_| {
                if drag.peek().is_some() {
                    if let Some(d) = drag.write().as_mut() { d.over = Some(index); }
                }
                // Dragging an item from another tile over this one: no drop target.
                if from.is_none() && item_drag.peek().as_ref().is_some_and(|d| d.over.is_some()) {
                    if let Some(d) = item_drag.write().as_mut() { d.over = None; }
                }
            },
            onmouseup: move |_| {
                if from.is_some() {
                    if let Some(Drag { id, over: Some(to) }) = item_drag.take() {
                        if let Some(target) = ids_drop.get(to) {
                            app::move_tile_item(&mut SETTINGS.write().starred_item_order, &STARRED.read(), &id, target);
                        }
                    }
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
                Avatar { src: image, name: name.clone() }
                div { class: "card-title",
                    match &interest {
                        Some(i) => rsx! { Link { to: Route::Editor { id: i.id.clone() }, h3 { "{name}" } } },
                        None => rsx! { h3 { "{name}" } },
                    }
                }
            }
            div { class: "card-body",
                div { class: "items",
                    for (j, s) in items.into_iter().enumerate() {
                        div {
                            key: "{s.id}",
                            class: item_class(j),
                            onmouseenter: move |_| {
                                if from.is_some() {
                                    if let Some(d) = item_drag.write().as_mut() { d.over = Some(j); }
                                }
                            },
                            span {
                                class: "grip",
                                title: "Drag to reorder",
                                onmousedown: {
                                    let id = s.id.clone();
                                    move |e: MouseEvent| {
                                        e.prevent_default();
                                        e.stop_propagation();
                                        item_drag.set(Some(Drag { id: id.clone(), over: None }));
                                    }
                                },
                                "⠿"
                            }
                            ItemRow { item: s.item.clone(), interest_id: s.interest_id.clone() }
                        }
                    }
                }
            }
        }
    }
}
