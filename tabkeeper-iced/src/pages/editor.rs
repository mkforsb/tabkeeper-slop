use std::fmt;

use iced::highlighter;
use iced::widget::{center, checkbox, column, mouse_area, opaque, pick_list, row, space, text, text_editor, Column};
use iced::{Bottom, Center, Fill, FillPortion, Task, Top};
use tabkeeper::app::{self, fmt_ago, fmt_bytes, fmt_count};
use tabkeeper::engine;
use tabkeeper::model::Interest;
use tabkeeper::script::{self, RunInput, RunReport, API_HELP};
use tabkeeper::templates::{self, TEMPLATES};

use super::Page;
use crate::style::{self, mono, SMALL};
use crate::widgets::*;
use crate::{App, Message, Route};

pub struct Editor {
    is_new: bool,
    draft: Interest,
    saved: Interest,
    /// The interval field's text; `draft.interval_mins` holds its last valid value.
    interval: String,
    script: text_editor::Content,
    test: Option<RunReport>,
    testing: bool,
    /// The "Run headless" text, shown in a dialog.
    headless: Option<text_editor::Content>,
    headless_running: bool,
    confirm_delete: bool,
    flash: Option<&'static str>,
    show_log: bool,
    show_output: bool,
    show_reference: bool,
}

/// A template in the picker (an index into [`TEMPLATES`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TemplateChoice(usize);

impl fmt::Display for TemplateChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(TEMPLATES[self.0].label)
    }
}

#[derive(Debug, Clone)]
pub enum Msg {
    Name(String),
    Interval(String),
    ImageUrl(String),
    Enabled(bool),
    Notify(bool),
    Reverse(bool),
    Template(TemplateChoice),
    Script(text_editor::Action),
    Save,
    Test,
    /// Interest id and report.
    TestFinished(String, Box<RunReport>),
    RunHeadless,
    /// Interest id and the text to show.
    HeadlessFinished(String, String),
    HeadlessAction(text_editor::Action),
    CloseHeadless,
    RunNow,
    ConfirmDelete(bool),
    Delete,
    ToggleLog,
    ToggleOutput,
    ToggleReference,
}

impl Editor {
    /// The editor for interest `id`, or for a new one if `id` is "new".
    pub fn open(app: &App, id: &str) -> Option<Self> {
        let is_new = id == "new";
        let initial = if is_new {
            Interest { script: templates::find("page-section").map(|t| t.script.to_string()).unwrap_or_default(), ..Default::default() }
        } else {
            app.interest(id)?.clone()
        };
        Some(Self {
            is_new,
            interval: initial.interval_mins.to_string(),
            script: text_editor::Content::with_text(&initial.script),
            draft: initial.clone(),
            saved: initial,
            test: None,
            testing: false,
            headless: None,
            headless_running: false,
            confirm_delete: false,
            flash: None,
            show_log: false,
            show_output: false,
            show_reference: is_new,
        })
    }

    fn dirty(&self) -> bool {
        self.draft != self.saved
    }
}

pub fn update(app: &mut App, msg: Msg) -> Task<Message> {
    let Page::Editor(ed) = &mut app.page else { return Task::none() };
    match msg {
        Msg::Name(v) => {
            ed.draft.name = v;
            ed.flash = None;
        }
        Msg::Interval(v) => {
            if v.chars().all(|c| c.is_ascii_digit()) {
                if let Ok(n) = v.parse() {
                    ed.draft.interval_mins = n;
                }
                ed.interval = v;
                ed.flash = None;
            }
        }
        Msg::ImageUrl(v) => {
            ed.draft.image_url = v;
            ed.flash = None;
        }
        Msg::Enabled(on) => ed.draft.enabled = on,
        Msg::Notify(on) => ed.draft.notify = on,
        Msg::Reverse(on) => ed.draft.reverse_order = on,
        Msg::Template(TemplateChoice(i)) => {
            let t = &TEMPLATES[i];
            ed.draft.script = t.script.to_string();
            ed.draft.interval_mins = t.interval_mins;
            if ed.draft.name.trim().is_empty() {
                ed.draft.name = t.name.to_string();
            }
            ed.script = text_editor::Content::with_text(t.script);
            ed.interval = t.interval_mins.to_string();
            ed.flash = None;
        }
        Msg::Script(action) => {
            let edit = action.is_edit();
            ed.script.perform(action);
            if edit {
                ed.draft.script = ed.script.text();
                ed.flash = None;
            }
        }
        Msg::Save => {
            let mut i = ed.draft.clone();
            i.name = i.name.trim().to_string();
            i.image_url = i.image_url.trim().to_string();
            i.interval_mins = i.interval_mins.max(1);
            ed.interval = i.interval_mins.to_string();
            ed.draft = i.clone();
            ed.saved = i.clone();
            ed.is_new = false;
            ed.flash = Some("Saved");
            app.upsert_interest(i);
        }
        Msg::Test => {
            let i = ed.draft.clone();
            ed.testing = true;
            let prev = app.states.get(&i.id).and_then(|s| s.last_output.clone());
            let input = RunInput { script: i.script, name: i.name, prev, cors_proxy: app.settings.cors_proxy.clone() };
            let id = i.id;
            return Task::perform(script::run(input), move |r| Message::Editor(Msg::TestFinished(id, Box::new(r))));
        }
        Msg::TestFinished(id, report) => {
            if ed.draft.id == id {
                ed.test = Some(*report);
                ed.testing = false;
            }
        }
        Msg::RunHeadless => {
            // Like `run_script`: no previous output, so nothing is compared.
            let i = ed.draft.clone();
            ed.headless_running = true;
            let input = RunInput { script: i.script, name: i.name, prev: None, cors_proxy: app.settings.cors_proxy.clone() };
            let id = i.id;
            return Task::perform(script::run(input), move |r| Message::Editor(Msg::HeadlessFinished(id, app::headless_dump(&r))));
        }
        Msg::HeadlessFinished(id, dump) => {
            if ed.draft.id == id {
                ed.headless = Some(text_editor::Content::with_text(&dump));
                ed.headless_running = false;
            }
        }
        Msg::HeadlessAction(action) => {
            // Read-only: allow selecting and scrolling, not editing.
            if let Some(content) = &mut ed.headless {
                if !action.is_edit() {
                    content.perform(action);
                }
            }
        }
        Msg::CloseHeadless => ed.headless = None,
        Msg::RunNow => {
            let id = ed.draft.id.clone();
            return app.start_run(id);
        }
        Msg::ConfirmDelete(on) => ed.confirm_delete = on,
        Msg::Delete => {
            let id = ed.draft.id.clone();
            app.delete_interest(&id);
            app.page = Page::open(app, Route::Interests);
        }
        Msg::ToggleLog => ed.show_log = !ed.show_log,
        Msg::ToggleOutput => ed.show_output = !ed.show_output,
        Msg::ToggleReference => ed.show_reference = !ed.show_reference,
    }
    Task::none()
}

fn msg(m: Msg) -> Message {
    Message::Editor(m)
}

pub fn view<'a>(app: &'a App, ed: &'a Editor) -> Element<'a> {
    let d = &ed.draft;
    let dirty = ed.dirty();
    let running = app.running.contains(&d.id);
    let valid = !d.name.trim().is_empty() && !d.script.trim().is_empty();

    let mut actions: Vec<Element> = Vec::new();
    if let Some(flash) = ed.flash {
        actions.push(text(format!("✓ {flash}")).size(SMALL).style(style::good).into());
    }
    if !ed.is_new {
        actions.push(status_pill(app, &ed.saved));
        // Runs the saved script, so it's only offered without unsaved changes.
        actions.push(btn("Run now", (!running && !dirty).then_some(msg(Msg::RunNow))).into());
    }
    actions.push(
        btn(if ed.testing { "Testing…" } else { "Test run" }, (!ed.testing && !d.script.trim().is_empty()).then_some(msg(Msg::Test)))
            .into(),
    );
    actions.push(
        btn(
            if ed.headless_running { "Running…" } else { "Run headless" },
            (!ed.headless_running && !d.script.trim().is_empty()).then_some(msg(Msg::RunHeadless)),
        )
        .into(),
    );
    actions.push(btn_primary("Save", (valid && (dirty || ed.is_new)).then_some(msg(Msg::Save))).into());
    let title = if ed.is_new { "New interest" } else { ed.saved.name.as_str() };

    column![
        page_head(title, actions),
        row![form(app, ed).width(FillPortion(5)), side(app, ed).width(FillPortion(4))].spacing(16).align_y(Top),
    ]
    .spacing(18)
    .into()
}

fn labeled<'a>(label: &'a str, field: impl Into<Element<'a>>) -> Column<'a, Message> {
    column![text(label).size(SMALL).style(style::ink_2), field.into()].spacing(4)
}

fn form<'a>(app: &'a App, ed: &'a Editor) -> Column<'a, Message> {
    let d = &ed.draft;
    let highlight_theme =
        if app.theme_is_dark() { highlighter::Theme::Base16Ocean } else { highlighter::Theme::InspiredGitHub };
    let templates: Vec<TemplateChoice> = (0..TEMPLATES.len()).map(TemplateChoice).collect();

    let mut fields = column![
        row![
            labeled(
                "Name",
                input("e.g. New screenings at Bio Rio", &d.name, |v| msg(Msg::Name(v)))
            )
            .width(Fill),
            labeled("Every (minutes)", input("60", &ed.interval, |v| msg(Msg::Interval(v)))).width(140),
        ]
        .spacing(12),
        row![
            labeled(
                "Image URL",
                input("Optional; defaults to the image the script returns", &d.image_url, |v| msg(Msg::ImageUrl(v)))
            )
            .width(Fill),
            avatar(app, &d.image_url, &d.name, 64.0),
        ]
        .spacing(12)
        .align_y(Bottom),
        row![
            checkbox(d.enabled).label("Refresh automatically").on_toggle(|on| msg(Msg::Enabled(on))),
            checkbox(d.notify).label("Notify on updates").on_toggle(|on| msg(Msg::Notify(on))),
            checkbox(d.reverse_order).label("Reverse item order").on_toggle(|on| msg(Msg::Reverse(on))),
        ]
        .spacing(16),
        column![
            row![
                text("Script").size(SMALL).style(style::ink_2),
                space::horizontal(),
                pick_list(templates, None::<TemplateChoice>, |t| msg(Msg::Template(t)))
                    .placeholder("Start from a template…")
                    .text_size(SMALL)
                    .padding([4, 8])
                    .style(style::pick_list)
                    .menu_style(style::menu),
            ]
            .align_y(Bottom),
            text_editor(&ed.script)
                .on_action(|a| msg(Msg::Script(a)))
                .font(mono())
                .size(13)
                .padding(10)
                .height(480)
                .style(style::text_editor)
                .highlight("rs", highlight_theme),
        ]
        .spacing(4),
    ]
    .spacing(14);
    if !ed.is_new && d.script != ed.saved.script {
        fields = fields.push(muted(
            "Saving a changed script resets this interest's baseline: the next run records the current state without reporting updates.",
        ));
    }
    if !ed.is_new {
        let danger: Element = if ed.confirm_delete {
            row![
                space::horizontal(),
                small("Delete this interest and its history?"),
                btn_small("Delete", Some(msg(Msg::Delete))).style(style::btn_danger),
                btn_small("Cancel", Some(msg(Msg::ConfirmDelete(false)))),
            ]
            .spacing(8)
            .align_y(Center)
            .into()
        } else {
            row![space::horizontal(), btn_small("Delete interest", Some(msg(Msg::ConfirmDelete(true))))].into()
        };
        fields = fields.push(hr()).push(danger);
    }
    column![panel(fields)]
}

fn side<'a>(app: &'a App, ed: &'a Editor) -> Column<'a, Message> {
    let mut side = column![].spacing(16);
    if let Some(report) = &ed.test {
        side = side.push(test_result(app, report, &ed.draft.id));
    }
    if let Some(st) = app.states.get(&ed.draft.id) {
        let mut p = column![
            h2("Last run"),
            kv(vec![
                ("Checked", format!("{} ({} ms)", fmt_ago(st.last_run_at, app.now), st.last_duration_ms)),
                ("Last success", fmt_ago(st.last_success_at, app.now)),
                ("Last update", fmt_ago(st.last_update_at, app.now)),
                ("Refreshes", format!("{} ({} failed)", fmt_count(st.stats.refreshes), fmt_count(st.stats.failures))),
                ("Updates caught", fmt_count(st.stats.updates)),
                ("Fetched", format!("{} requests, {}", fmt_count(st.stats.fetches), fmt_bytes(st.stats.bytes))),
            ]),
        ]
        .spacing(10);
        if let Some(err) = &st.last_error {
            p = p.push(error_box(err));
        }
        if !st.last_logs.is_empty() {
            let summary = format!("Log ({} lines)", st.last_logs.len());
            p = p.push(details(summary, ed.show_log, msg(Msg::ToggleLog), || log_box(&st.last_logs)));
        }
        if let Some(out) = &st.last_output {
            let summary = format!("Current output ({} items)", out.items.len());
            p = p.push(details(summary, ed.show_output, msg(Msg::ToggleOutput), || output_view(app, out, 20, true, ed.draft.reverse_order, "editor:output".into(), None)));
        }
        side = side.push(panel(p));
    }
    side.push(panel(details("Script reference".into(), ed.show_reference, msg(Msg::ToggleReference), reference)))
}

/// The "Run headless" dialog, drawn over the whole window. Clicking outside it closes it.
pub fn modal(ed: &Editor) -> Option<Element<'_>> {
    let content = ed.headless.as_ref()?;
    let dialog = panel(
        column![
            row![h2("Headless run"), space::horizontal(), btn_small("Close", Some(msg(Msg::CloseHeadless)))].align_y(Center),
            text_editor(content)
                .on_action(|a| msg(Msg::HeadlessAction(a)))
                .font(mono())
                .size(SMALL)
                .height(Fill)
                .style(style::text_editor),
        ]
        .spacing(10),
    )
    .max_width(960)
    .height(Fill);
    Some(opaque(mouse_area(center(opaque(dialog)).padding(24).style(style::backdrop)).on_press(msg(Msg::CloseHeadless))))
}

fn reference<'a>() -> Element<'a> {
    let mut col = column![muted(
        "Scripts are written in Rhai (rhai.rs/book), a small Rust-like language. The value of the last expression is the output."
    )]
    .spacing(8);
    for (sig, desc) in API_HELP {
        col = col.push(column![text(*sig).font(mono()).size(SMALL), text(*desc).size(SMALL).style(style::ink_2)].spacing(2));
    }
    col.into()
}

fn test_result<'a>(app: &'a App, report: &'a RunReport, interest_id: &str) -> Element<'a> {
    let st = app.state(interest_id);
    let verdict = report.result.as_ref().ok().map(|out| match engine::detect(st.last_output.as_ref(), &st.seen_ids, out) {
        _ if st.last_output.is_none() => "First run: this output would become the baseline.".to_string(),
        Some(c) => format!("Would report: {}", c.summary),
        None => "No change compared to the last saved run.".to_string(),
    });
    let meta = format!(
        "{} ms · {} · {}",
        report.duration_ms,
        counted(report.fetches.len(), "fetch", "fetches"),
        counted(report.rounds as usize, "pass", "passes")
    );

    let mut p = column![row![h2("Test run"), space::horizontal(), muted(meta)].align_y(Center)].spacing(8);
    p = match &report.result {
        Ok(_) => p.push(text("✓ Success").size(SMALL).style(style::good)),
        Err(e) => p.push(error_box(e)),
    };
    if let Some(v) = verdict {
        p = p.push(small(v));
    }
    if !report.fetches.is_empty() {
        let fetches = report.fetches.iter().map(|f| {
            let (mark, style, rest): (String, fn(&iced::Theme) -> text::Style, String) = match (&f.error, f.status) {
                (Some(e), _) => ("✕".into(), style::critical, format!("{} — {e}", f.label())),
                (None, Some(s)) if (200..300).contains(&s) => {
                    (format!("✓ {s}"), style::good, format!("{} · {} · {} ms", f.label(), fmt_bytes(f.bytes as u64), f.ms))
                }
                (None, s) => (format!("! {}", s.unwrap_or(0)), style::warning, format!("{} · {} ms", f.label(), f.ms)),
            };
            row![text(mark).size(SMALL).style(style), small(rest).wrapping(text::Wrapping::WordOrGlyph)].spacing(6).into()
        });
        p = p.push(Column::with_children(fetches).spacing(3));
    }
    if !report.logs.is_empty() {
        p = p.push(log_box(&report.logs));
    }
    if let Ok(out) = &report.result {
        p = p.push(output_view(app, out, 30, true, false, "editor:test".into(), None));
    }
    panel(p).into()
}

fn counted(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}
