use iced::widget::{checkbox, column, container, mouse_area, row, rule, text, Column};
use iced::{alignment, mouse, Center, Fill, Length};
use tabkeeper::app::fmt_count;
use tabkeeper::model::Interest;

use crate::style;
use crate::widgets::*;
use crate::{App, Message, Route};

/// An interest being dragged to a new position.
#[derive(Debug, Clone)]
pub struct Drag {
    pub id: String,
    /// The row under the cursor, where the interest will be moved to.
    pub over: Option<usize>,
}

pub fn view<'a>(app: &'a App, confirm_delete: Option<&str>, drag: Option<&Drag>) -> Element<'a> {
    let head = page_head("Interests", vec![btn_primary("+ New interest", Some(Message::Navigate(Route::Editor("new".into())))).into()]);
    if app.interests.is_empty() {
        let empty = panel(
            column![text("No interests yet."), btn("Add the example interests", Some(Message::AddExamples))]
                .spacing(12)
                .align_x(Center),
        )
        .padding([40, 20]);
        return column![head, empty].spacing(18).into();
    }

    let header = cells([
        text("").into(),
        text("").into(),
        muted("Name").into(),
        muted("Status").into(),
        muted("Every").into(),
        num(muted("Refreshes")),
        num(muted("Failures")),
        num(muted("Updates")),
        muted("Enabled").into(),
        text("").into(),
    ]);
    // While dragging, the separator at the drop position is drawn in the
    // accent color: above the target row when moving up, below it when moving down.
    let from = drag.and_then(|d| app.interests.iter().position(|i| i.id == d.id));
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
    for (k, i) in app.interests.iter().enumerate() {
        let r = table_row(app, i, confirm_delete == Some(i.id.as_str()));
        let r: Element = if from == Some(k) { container(r).style(style::fresh).into() } else { r };
        rows = rows.push(separator(k)).push(mouse_area(r).on_enter(Message::DragOver(Some(k))));
    }
    if marker == Some(app.interests.len()) {
        rows = rows.push(separator(app.interests.len()));
    }

    let table: Element = if drag.is_some() {
        // Releasing outside the table cancels the drag.
        mouse_area(rows).on_exit(Message::DragOver(None)).interaction(mouse::Interaction::Grabbing).into()
    } else {
        rows.into()
    };
    column![head, panel(table), muted("Drag ⠿ to reorder. The dashboard shows interests in this order.")].spacing(18).into()
}

const WIDTHS: [Length; 10] = [
    Length::Fixed(14.0),
    Length::Fixed(40.0),
    Length::FillPortion(3),
    Length::Fixed(120.0),
    Length::Fixed(70.0),
    Length::Fixed(80.0),
    Length::Fixed(70.0),
    Length::Fixed(70.0),
    Length::Fixed(70.0),
    Length::Fixed(220.0),
];

/// Lays out one table row with the column widths above.
fn cells<'a>(cells: [Element<'a>; 10]) -> Element<'a> {
    row(cells.into_iter().zip(WIDTHS).map(|(c, w)| container(c).width(w).into())).spacing(10).align_y(Center).padding([7, 4]).into()
}

fn num<'a>(e: impl Into<Element<'a>>) -> Element<'a> {
    container(e).width(Fill).align_x(alignment::Horizontal::Right).into()
}

fn table_row<'a>(app: &'a App, interest: &'a Interest, confirming: bool) -> Element<'a> {
    let st = app.state(&interest.id);
    let running = app.running.contains(&interest.id);
    let id = &interest.id;

    let mut actions = row![
        btn_small("Run", (!running).then(|| Message::Run(id.clone()))),
        btn_small("Edit", Some(Message::Navigate(Route::Editor(id.clone())))),
    ]
    .spacing(6);
    actions = if confirming {
        actions
            .push(btn_small("Confirm", Some(Message::Delete(id.clone()))).style(style::btn_danger))
            .push(btn_small("Cancel", Some(Message::ConfirmDelete(None))))
    } else {
        actions.push(btn_small("Delete", Some(Message::ConfirmDelete(Some(id.clone())))))
    };

    let grip = mouse_area(text("⠿").size(18).style(style::ink_2))
        .on_press(Message::DragStart(id.clone()))
        .interaction(mouse::Interaction::Grab);

    cells([
        grip.into(),
        avatar(app, &display_image(app, interest), &interest.name, 28.0),
        column![link(text(interest.name.as_str()), Route::Editor(id.clone())), timing(app, interest)].spacing(2).into(),
        status_pill(app, interest),
        text(format!("{} min", interest.interval_mins)).style(style::muted).into(),
        num(text(fmt_count(st.stats.refreshes))),
        num(text(fmt_count(st.stats.failures))),
        num(text(fmt_count(st.stats.updates))),
        checkbox(interest.enabled).on_toggle(move |on| Message::SetEnabled(id.clone(), on)).size(16).into(),
        container(actions).width(Fill).align_x(alignment::Horizontal::Right).into(),
    ])
}

