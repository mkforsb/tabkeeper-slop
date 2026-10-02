use iced::widget::{column, container, row, space, text, Column};
use iced::{alignment, Center, Fill, Length};
use tabkeeper::app::{fmt_ago, fmt_bytes, fmt_count, fmt_duration, fmt_time};
use tabkeeper::model::Interest;

use crate::style;
use crate::widgets::*;
use crate::{App, Route};

pub fn view(app: &App) -> Element<'_> {
    let sys = &app.system;
    let uptime = app.now - app.session_started;
    let fail_rate = if sys.refreshes > 0 { sys.failures as f64 * 100.0 / sys.refreshes as f64 } else { 0.0 };
    let enabled = app.interests.iter().filter(|i| i.enabled).count();
    let storage = stored_size(app);

    let tiles = tiles(vec![
        tile("Refreshes", fmt_count(sys.refreshes), format!("{} fetches", fmt_count(sys.fetches))),
        tile_with("Failures", fmt_count(sys.failures), format!("{fail_rate:.1}% of refreshes"), reset_failures(app)),
        tile("Updates caught", fmt_count(sys.updates), format!("{} in history", app.events.len())),
        tile("Data fetched", fmt_bytes(sys.bytes), format!("{} stored locally", fmt_bytes(storage as u64))),
    ]);

    let worker = format!(
        "{}, {} refresh(es) in flight, concurrency {}",
        if app.settings.paused { "paused" } else { "running" },
        app.running.len(),
        app.settings.concurrency
    );
    let system = panel(
        column![
            h2("System"),
            kv(vec![
                ("Session uptime", fmt_duration(uptime)),
                ("Worker", worker),
                ("First started", fmt_time(sys.first_started_at)),
                ("Launches", fmt_count(sys.launches)),
                ("Interests", format!("{} ({enabled} enabled)", app.interests.len())),
                ("Platform", "desktop (Iced)".into()),
            ]),
        ]
        .spacing(10),
    );

    let header = cells([
        muted("Name").into(),
        num(muted("Refreshes")),
        num(muted("Failures")),
        num(muted("Fail rate")),
        num(muted("Updates")),
        num(muted("Avg time")),
        num(muted("Fetched")),
        muted("Last success").into(),
        muted("Last update").into(),
    ]);
    let mut table = Column::new().push(header);
    for i in &app.interests {
        table = table.push(hr()).push(table_row(app, i));
    }
    let per_interest = panel(column![h2("Per interest"), table].spacing(10));

    column![page_head("Stats", vec![]), tiles, system, per_interest].spacing(18).into()
}

const WIDTHS: [Length; 9] = [
    Length::FillPortion(3),
    Length::Fixed(80.0),
    Length::Fixed(70.0),
    Length::Fixed(110.0),
    Length::Fixed(70.0),
    Length::Fixed(80.0),
    Length::Fixed(80.0),
    Length::Fixed(100.0),
    Length::Fixed(100.0),
];

fn cells<'a>(cells: [Element<'a>; 9]) -> Element<'a> {
    row(cells.into_iter().zip(WIDTHS).map(|(c, w)| container(c).width(w).into())).spacing(10).align_y(Center).padding([7, 4]).into()
}

fn num<'a>(e: impl Into<Element<'a>>) -> Element<'a> {
    container(e).width(Fill).align_x(alignment::Horizontal::Right).into()
}

fn table_row<'a>(app: &'a App, i: &'a Interest) -> Element<'a> {
    let st = app.state(&i.id);
    let s = &st.stats;
    let rate = if s.refreshes > 0 { s.failures as f64 * 100.0 / s.refreshes as f64 } else { 0.0 };
    let avg = s.total_duration_ms.checked_div(s.refreshes).unwrap_or(0);
    let meter = container(
        container(space::horizontal()).width((50.0 * rate.clamp(0.0, 100.0) / 100.0) as f32).height(6).style(style::meter_fill(rate)),
    )
    .width(50)
    .height(6)
    .style(style::meter_track);

    cells([
        link(text(i.name.as_str()), Route::Editor(i.id.clone())).into(),
        num(text(fmt_count(s.refreshes))),
        num(text(fmt_count(s.failures))),
        num(row![meter, text(format!("{rate:.0}%"))].spacing(8).align_y(Center)),
        num(text(fmt_count(s.updates))),
        num(text(format!("{avg} ms"))),
        num(text(fmt_bytes(s.bytes))),
        text(fmt_ago(st.last_success_at, app.now)).style(style::muted).into(),
        text(fmt_ago(st.last_update_at, app.now)).style(style::muted).into(),
    ])
}

/// Approximate stored size in bytes.
fn stored_size(app: &App) -> usize {
    fn len<T: serde::Serialize>(v: &T) -> usize {
        serde_json::to_string(v).map(|s| s.len()).unwrap_or(0)
    }
    len(&app.interests) + len(&app.states) + len(&app.events) + len(&app.settings) + len(&app.system)
}
