use iced::widget::{button, column, container, mouse_area, row, rule, space, text, Column};
use iced::{mouse, Center, Fill, Length};
use tabkeeper::app::{self, fmt_ago, StarredTile};
use tabkeeper::model::StarredItem;

use crate::style::{self, bold};
use crate::widgets::*;
use super::dashboard::{CardLook, CARD_MIN_WIDTH, GAP};
use super::Drag;
use crate::{App, Message, Route};

/// Starred items as a list in its own order, or as tiles per interest.
pub fn view<'a>(app: &'a App, drag: Option<&Drag>, item_drag: Option<&Drag>) -> Element<'a> {
    let tiles = app.settings.starred_tiles;
    let mut actions: Vec<Element> = Vec::new();
    if tiles {
        let tile_count = app::starred_tiles(&app.starred, &[], &[]).len();
        actions.push(btn("Shuffle", (tile_count > 1).then_some(Message::ShuffleStarredTiles)).into());
    }
    actions.push(view_toggle(tiles));
    let head = page_head("Starred", actions);
    let body = if app.starred.is_empty() {
        panel(muted("No starred items yet. Click ★ next to an item on the Dashboard or under Updates to keep it here."))
            .padding([40, 20])
            .center_x(Fill)
            .into()
    } else if tiles {
        tiles_view(app, drag, item_drag)
    } else {
        list_view(app, drag)
    };
    column![head, body].spacing(18).into()
}

/// "☰ List | ▦ Tiles"
fn view_toggle<'a>(tiles: bool) -> Element<'a> {
    let segment = |label, active: bool, on| {
        button(text(label).font(if active { bold() } else { style::regular() }))
            .padding([6, 12])
            .style(style::segment(active))
            .on_press(Message::SetStarredTiles(on))
    };
    container(row![segment("☰ List", !tiles, false), segment("▦ Tiles", tiles, true)])
        .padding(1)
        .style(style::segmented)
        .into()
}

fn list_view<'a>(app: &'a App, drag: Option<&Drag>) -> Element<'a> {
    let header = cells([text("").into(), muted("Item").into(), muted("Interest").into(), muted("Starred").into()]);
    // As on the Interests page: the separator at the drop position is drawn
    // in the accent color, above the target row when moving up, below it when moving down.
    let from = drag.and_then(|d| app.starred.iter().position(|s| s.id == d.id));
    let marker = match (from, drag.and_then(|d| d.over)) {
        (Some(from), Some(to)) if to < from => Some(to),
        (Some(from), Some(to)) if to > from => Some(to + 1),
        _ => None,
    };
    let separator = |k: usize| -> Element<'a> {
        if marker == Some(k) {
            rule::horizontal(2).style(style::drop_marker).into()
        } else {
            hr()
        }
    };

    let mut rows = Column::new().push(header);
    for (k, s) in app.starred.iter().enumerate() {
        let r = table_row(app, s);
        let r: Element = if from == Some(k) { container(r).style(style::fresh).into() } else { r };
        rows = rows.push(separator(k)).push(mouse_area(r).on_enter(Message::DragOver(Some(k))));
    }
    if marker == Some(app.starred.len()) {
        rows = rows.push(separator(app.starred.len()));
    }

    let table: Element = if drag.is_some() {
        // Releasing outside the table cancels the drag.
        mouse_area(rows).on_exit(Message::DragOver(None)).interaction(mouse::Interaction::Grabbing).into()
    } else {
        rows.into()
    };
    column![panel(table), muted("Drag ⠿ to reorder.")].spacing(18).into()
}

const WIDTHS: [Length; 4] = [Length::Fixed(14.0), Length::FillPortion(3), Length::FillPortion(1), Length::Fixed(90.0)];

/// Lays out one table row with the column widths above.
fn cells<'a>(cells: [Element<'a>; 4]) -> Element<'a> {
    row(cells.into_iter().zip(WIDTHS).map(|(c, w)| container(c).width(w).into())).spacing(10).align_y(Center).padding([7, 4]).into()
}

fn grip<'a>(on_press: Message) -> Element<'a> {
    mouse_area(text("⠿").size(18).style(style::ink_2)).on_press(on_press).interaction(mouse::Interaction::Grab).into()
}

fn table_row<'a>(app: &'a App, s: &'a StarredItem) -> Element<'a> {
    // Deleted interests are shown by the name they had when starred.
    let interest: Element = match app.interest(&s.interest_id) {
        Some(i) => link(text(i.name.as_str()), Route::Editor(i.id.clone())).into(),
        None => muted(s.interest_name.as_str()).into(),
    };
    cells([
        grip(Message::DragStart(s.id.clone())),
        item_row(app, &s.item, false, Some(&s.interest_id)),
        interest,
        muted(fmt_ago(Some(s.starred_at), app.now)).into(),
    ])
}

/// One tile per interest in masonry columns, like the Dashboard's cards. Both
/// the tiles and the items in each can be dragged to reorder, in an order of
/// their own.
fn tiles_view<'a>(app: &'a App, drag: Option<&Drag>, item_drag: Option<&Drag>) -> Element<'a> {
    let n = (((app.content_width() + GAP) / (CARD_MIN_WIDTH + GAP)).floor() as usize).max(1);
    let tiles = app::starred_tiles(&app.starred, &app.settings.starred_tile_order, &app.settings.starred_item_order);
    let from = drag.and_then(|d| tiles.iter().position(|t| t.interest_id == d.id));
    let over = drag.and_then(|d| d.over);
    let mut columns: Vec<Vec<Element>> = (0..n).map(|_| Vec::new()).collect();
    for (k, t) in tiles.into_iter().enumerate() {
        let look = match (from, over) {
            (Some(f), _) if f == k => CardLook::DragSource,
            (Some(_), Some(o)) if o == k => CardLook::DropTarget,
            _ => CardLook::Normal,
        };
        columns[k % n].push(tile(app, t, k, look, item_drag));
    }
    let grid = row(columns.into_iter().map(|c| Column::with_children(c).spacing(GAP).width(Fill).into())).spacing(GAP);
    let grid: Element = if drag.is_some() || item_drag.is_some() {
        // Releasing outside the tiles cancels the drag.
        mouse_area(grid).on_exit(Message::DragOver(None)).interaction(mouse::Interaction::Grabbing).into()
    } else {
        grid.into()
    };
    column![grid, muted("Drag ⠿ to reorder the tiles, and the items within each tile.")].spacing(18).into()
}

fn tile<'a>(app: &'a App, t: StarredTile<'a>, index: usize, look: CardLook, item_drag: Option<&Drag>) -> Element<'a> {
    let interest = app.interest(t.interest_id);
    // Deleted interests are shown by the name they had when starred.
    let name = interest.map_or(t.interest_name, |i| i.name.as_str());
    let image = interest.map(|i| display_image(app, i)).unwrap_or_default();
    let title = text(name).size(15).font(bold());
    let title: Element = match interest {
        Some(i) => button(title).padding(0).style(style::plain_link).on_press(Message::Navigate(Route::Editor(i.id.clone()))).into(),
        None => title.into(),
    };
    let head = row![grip(Message::DragStart(t.interest_id.to_string())), avatar(app, &image, name, 40.0), container(title).width(Fill)]
        .spacing(10)
        .align_y(Center);

    // Item drags stay within their tile. The gap at the drop position is drawn
    // in the accent color, above the target item when moving up, below it when moving down.
    let from = item_drag.and_then(|d| t.items.iter().position(|s| s.id == d.id));
    let marker = match (from, item_drag.and_then(|d| d.over)) {
        (Some(from), Some(to)) if to < from => Some(to),
        (Some(from), Some(to)) if to > from => Some(to + 1),
        _ => None,
    };
    let gap = |j: usize| -> Element<'a> {
        if marker == Some(j) {
            container(rule::horizontal(2).style(style::drop_marker)).height(4).center_y(4).into()
        } else {
            space::vertical().height(4).into()
        }
    };
    let mut items = Column::new();
    for (j, s) in t.items.iter().enumerate() {
        let r = row![grip(Message::ItemDragStart(s.id.clone())), item_row(app, &s.item, false, Some(&s.interest_id))]
            .spacing(6)
            .align_y(Center);
        let r: Element = if from == Some(j) { container(r).style(style::fresh).into() } else { r.into() };
        let r: Element = if from.is_some() { mouse_area(r).on_enter(Message::ItemDragOver(Some(j))).into() } else { r };
        items = items.push(gap(j)).push(r);
    }
    items = items.push(gap(t.items.len()));

    let card = container(column![container(head).padding([12, 14]), hr(), container(items).padding([6, 14]).width(Fill)])
        .width(Fill)
        .style(look.style());
    let mut area = mouse_area(card).on_enter(Message::DragOver(Some(index)));
    if from.is_some() {
        // Leaving the tile while dragging one of its items: no drop target.
        area = area.on_exit(Message::ItemDragOver(None));
    }
    area.into()
}
