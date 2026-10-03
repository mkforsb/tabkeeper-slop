use iced::widget::{column, row, text, Column};
use iced::Center;
use tabkeeper::app::{fmt_ago, fmt_time};

use crate::style::{self, bold};
use crate::widgets::*;
use crate::{App, Message, Route};

/// Events are listed in pages of this size; the history can hold hundreds.
pub const PAGE_SIZE: usize = 50;

pub fn view(app: &App, confirm_clear: bool, shown: usize) -> Element<'_> {
    let unread = app.unread_count(None);
    let mut actions: Vec<Element> = vec![btn("Mark all read", (unread > 0).then_some(Message::MarkRead(None))).into()];
    if confirm_clear {
        actions.push(btn("Clear history", Some(Message::ClearEvents)).style(style::btn_danger).into());
        actions.push(btn("Cancel", Some(Message::ConfirmClear(false))).into());
    } else {
        actions.push(btn("Clear…", (!app.events.is_empty()).then_some(Message::ConfirmClear(true))).into());
    }
    let mut page = column![page_head("Updates", actions)].spacing(18);

    if app.events.is_empty() {
        page = page.push(panel(muted("No updates caught yet.")).padding([40, 20]).center_x(iced::Fill));
    }

    let mut timeline = Column::new().spacing(12);
    for e in app.events.iter().take(shown) {
        let mut head = row![
            link(text(e.interest_name.as_str()).font(bold()), Route::Editor(e.interest_id.clone())),
            muted(format!("{} · {}", fmt_ago(Some(e.at), app.now), fmt_time(e.at))),
        ]
        .spacing(10)
        .align_y(Center);
        if !e.read {
            head = head.push(btn_small("Mark read", Some(Message::MarkEventRead(e.id.clone()))));
        }
        let mut body = column![head, text(e.summary.as_str())].spacing(8);
        if !e.items.is_empty() {
            body = body.push(Column::with_children(e.items.iter().map(|i| item_row(app, i, true, Some(&e.interest_id)))).spacing(4));
        }
        timeline = timeline.push(panel(if e.read { body.into() } else { unread_marker(body.into()) }));
    }
    page = page.push(timeline);

    let hidden = app.events.len().saturating_sub(shown);
    if hidden > 0 {
        page = page.push(btn(format!("Show more ({hidden} older)"), Some(Message::ShowMoreEvents)));
    }
    page.into()
}
