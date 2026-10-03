use iced::widget::{container, mouse_area, row, rule, text, Column};
use iced::{mouse, Center, Fill, Length};
use tabkeeper::app::fmt_ago;
use tabkeeper::model::StarredItem;

use crate::style;
use crate::widgets::*;
use super::Drag;
use crate::{App, Message, Route};

pub fn view<'a>(app: &'a App, drag: Option<&Drag>) -> Element<'a> {
    let head = page_head("Starred", vec![]);
    if app.starred.is_empty() {
        let empty = panel(muted("No starred items yet. Click ★ next to an item on the Dashboard or under Updates to keep it here."))
            .padding([40, 20])
            .center_x(Fill);
        return iced::widget::column![head, empty].spacing(18).into();
    }

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
    iced::widget::column![head, panel(table), muted("Drag ⠿ to reorder.")].spacing(18).into()
}

const WIDTHS: [Length; 4] = [Length::Fixed(14.0), Length::FillPortion(3), Length::FillPortion(1), Length::Fixed(90.0)];

/// Lays out one table row with the column widths above.
fn cells<'a>(cells: [Element<'a>; 4]) -> Element<'a> {
    row(cells.into_iter().zip(WIDTHS).map(|(c, w)| container(c).width(w).into())).spacing(10).align_y(Center).padding([7, 4]).into()
}

fn table_row<'a>(app: &'a App, s: &'a StarredItem) -> Element<'a> {
    let grip = mouse_area(text("⠿").size(18).style(style::ink_2))
        .on_press(Message::DragStart(s.id.clone()))
        .interaction(mouse::Interaction::Grab);
    // Deleted interests are shown by the name they had when starred.
    let interest: Element = match app.interest(&s.interest_id) {
        Some(i) => link(text(i.name.as_str()), Route::Editor(i.id.clone())).into(),
        None => muted(s.interest_name.as_str()).into(),
    };
    cells([
        grip.into(),
        item_row(app, &s.item, false, Some(&s.interest_id)),
        interest,
        muted(fmt_ago(Some(s.starred_at), app.now)).into(),
    ])
}
