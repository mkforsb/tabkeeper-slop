use iced::widget::{button, column, container, row, space, text, Column};
use iced::{Center, Fill, Top};
use tabkeeper::app::{fmt_ago, fmt_count};
use tabkeeper::model::Interest;

use crate::style::{self, bold, SMALL};
use crate::widgets::*;
use crate::{App, Message, Route};

const CARD_MIN_WIDTH: f32 = 320.0;
const FEED_WIDTH: f32 = 300.0;
const GAP: f32 = 14.0;

pub fn view(app: &App) -> Element<'_> {
    let now = app.now;
    let day_ago = now - 86_400_000;
    let updates_24h = app.events.iter().filter(|e| e.at >= day_ago).count() as u64;
    let failing = app.interests.iter().filter(|i| i.enabled && app.state(&i.id).last_error.is_some()).count() as u64;
    let enabled = app.interests.iter().filter(|i| i.enabled).count() as u64;
    let total = app.interests.len();
    let has_interests = total > 0;

    let head = page_head(
        "Dashboard",
        vec![
            btn("Refresh all", has_interests.then_some(Message::RefreshAll)).into(),
            btn_primary("+ New interest", Some(Message::Navigate(Route::Editor("new".into())))).into(),
        ],
    );
    let tiles = tiles(vec![
        tile("Interests watched", fmt_count(enabled), format!("{total} total")),
        tile("Updates in last 24h", fmt_count(updates_24h), format!("{} unread", app.unread_count(None))),
        tile("Refreshes", fmt_count(app.system.refreshes), format!("{} failed", fmt_count(app.system.failures))),
        tile(
            "Failing now",
            fmt_count(failing),
            if failing > 0 { "✕ needs attention".into() } else { "✓ all healthy".into() },
        ),
    ]);
    let body = if has_interests { dash(app) } else { empty_state() };
    column![head, tiles, body].spacing(18).into()
}

/// Interest cards in masonry columns, with the recent-updates feed beside
/// them (or below, when the window is narrow).
fn dash(app: &App) -> Element<'_> {
    let width = app.content_width();
    let side_by_side = width >= 900.0;
    let cards_width = if side_by_side { width - FEED_WIDTH - 16.0 } else { width };
    let n = (((cards_width + GAP) / (CARD_MIN_WIDTH + GAP)).floor() as usize).max(1);

    let mut columns: Vec<Vec<Element>> = (0..n).map(|_| Vec::new()).collect();
    for (k, i) in app.interests.iter().enumerate() {
        columns[k % n].push(card(app, i));
    }
    let cards = row(columns.into_iter().map(|c| Column::with_children(c).spacing(GAP).width(Fill).into())).spacing(GAP);

    if side_by_side {
        row![cards.width(Fill), feed(app).width(FEED_WIDTH)].spacing(16).align_y(Top).into()
    } else {
        column![cards, feed(app)].spacing(16).into()
    }
}

fn card<'a>(app: &'a App, interest: &'a Interest) -> Element<'a> {
    let st = app.state(&interest.id);
    let unread = app.unread_count(Some(&interest.id));
    let running = app.running.contains(&interest.id);

    let mut head = row![
        avatar(app, &display_image(app, interest), &interest.name, 40.0),
        column![
            button(text(interest.name.as_str()).size(15).font(bold()))
                .padding(0)
                .style(style::plain_link)
                .on_press(Message::Navigate(Route::Editor(interest.id.clone()))),
            timing(app, interest),
        ]
        .spacing(2)
        .width(Fill),
    ]
    .spacing(10)
    .align_y(Center);
    if unread > 0 {
        head = head.push(
            button(text(format!("{unread} new")).size(11.5).font(bold()))
                .padding([1, 8])
                .style(style::badge)
                .on_press(Message::MarkRead(Some(interest.id.clone()))),
        );
    }

    let mut body = column![].spacing(6);
    if let Some(err) = &st.last_error {
        body = body.push(error_box(err));
    }
    body = match &st.last_output {
        Some(out) => body.push(output_view(app, out, 4, false, interest.reverse_order)),
        None if running => body.push(muted("Running for the first time…")),
        None if st.last_error.is_none() => body.push(muted("Not checked yet.")),
        None => body,
    };

    let foot = row![
        status_pill(app, interest),
        space::horizontal(),
        btn_small("↑↓", Some(Message::SetReversed(interest.id.clone(), !interest.reverse_order))),
        btn_small("Refresh", (!running).then(|| Message::Run(interest.id.clone()))),
        btn_small("Edit", Some(Message::Navigate(Route::Editor(interest.id.clone())))),
    ]
    .spacing(6)
    .align_y(Center);

    container(column![
        container(head).padding([12, 14]),
        hr(),
        container(body).padding([10, 14]).width(Fill),
        hr(),
        container(foot).padding([8, 14]),
    ])
    .width(Fill)
    .style(style::panel)
    .into()
}

fn feed(app: &App) -> iced::widget::Container<'_, Message> {
    let mut col = column![row![h2("Recent updates"), space::horizontal(), link(text("All →").size(SMALL), Route::Updates)]
        .align_y(Center)]
    .spacing(8);
    if app.events.is_empty() {
        col = col.push(muted(
            "Nothing caught yet. The first refresh of an interest records a baseline; changes after that show up here.",
        ));
    }
    for e in app.events.iter().take(8) {
        let entry = column![
            row![
                link(text(e.interest_name.as_str()), Route::Editor(e.interest_id.clone())),
                space::horizontal(),
                muted(fmt_ago(Some(e.at), app.now)),
            ]
            .align_y(Center),
            small(e.summary.as_str()),
        ]
        .spacing(2)
        .width(Fill);
        col = col.push(hr());
        col = col.push(if e.read { entry.into() } else { unread_marker(entry.into()) });
    }
    panel(col)
}

fn empty_state<'a>() -> Element<'a> {
    panel(
        column![
            h2("Nothing to keep tabs on yet"),
            text("An interest is a script that fetches a page and extracts what matters. Tabkeeper runs it periodically and tells you when something new shows up.")
                .style(style::ink_2),
            row![
                btn_primary("Create an interest", Some(Message::Navigate(Route::Editor("new".into())))),
                btn("Add the example interests", Some(Message::AddExamples)),
            ]
            .spacing(8),
        ]
        .spacing(12)
        .align_x(Center),
    )
    .padding([40, 20])
    .into()
}
