use iced::widget::{checkbox, column, pick_list, row, text, text_editor};
use iced::{Fill, Font, Task};
use tabkeeper::app::Backup;

use super::Page;
use crate::appearance::{self, FontChoice, Scale, SCALES};
use crate::style::{self, mono, SMALL};
use crate::widgets::*;
use crate::{App, Dirty, Message};

pub struct SettingsPage {
    /// Field texts; the settings hold their last valid (clamped) values.
    concurrency: String,
    max_events: String,
    export: Option<text_editor::Content>,
    import: text_editor::Content,
    confirm_reset: bool,
    ui_fonts: Vec<FontChoice>,
    mono_fonts: Vec<FontChoice>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Concurrency(String),
    MaxEvents(String),
    /// Shows the clamped values once editing is done.
    Normalize,
    Paused(bool),
    Notifications(bool),
    Scale(Scale),
    UiFont(FontChoice),
    MonoFont(FontChoice),
    TestNotification,
    TestNotificationDone(Result<(), String>),
    Export,
    ExportAction(text_editor::Action),
    CopyExport,
    ImportAction(text_editor::Action),
    Import,
    ConfirmReset(bool),
    Reset,
}

impl SettingsPage {
    pub fn new(app: &App) -> Self {
        Self {
            concurrency: app.settings.concurrency.to_string(),
            max_events: app.settings.max_events.to_string(),
            export: None,
            import: text_editor::Content::new(),
            confirm_reset: false,
            ui_fonts: FontChoice::list(&appearance::families().ui),
            mono_fonts: FontChoice::list(&appearance::families().mono),
        }
    }
}

fn msg(m: Msg) -> Message {
    Message::Settings(m)
}

fn digits(v: &str) -> bool {
    v.chars().all(|c| c.is_ascii_digit())
}

pub fn update(app: &mut App, msg: Msg) -> Task<Message> {
    let Page::Settings(page) = &mut app.page else { return Task::none() };
    match msg {
        Msg::Concurrency(v) if digits(&v) => {
            if let Ok(n) = v.parse::<u32>() {
                app.settings.concurrency = n.clamp(1, 16);
                app.dirty.settings = true;
            }
            page.concurrency = v;
        }
        Msg::MaxEvents(v) if digits(&v) => {
            if let Ok(n) = v.parse::<u32>() {
                app.settings.max_events = n.max(10);
                app.dirty.settings = true;
            }
            page.max_events = v;
        }
        Msg::Concurrency(_) | Msg::MaxEvents(_) => {}
        Msg::Normalize => {
            page.concurrency = app.settings.concurrency.to_string();
            page.max_events = app.settings.max_events.to_string();
        }
        Msg::Paused(on) => {
            app.settings.paused = on;
            app.dirty.settings = true;
        }
        Msg::Notifications(on) => {
            app.settings.notifications = on;
            app.dirty.settings = true;
        }
        Msg::Scale(Scale(percent)) => {
            app.appearance.scale = percent;
            app.appearance.save();
        }
        Msg::UiFont(FontChoice(family)) => {
            app.appearance.ui_font = family;
            app.appearance.save();
        }
        Msg::MonoFont(FontChoice(family)) => {
            app.appearance.mono_font = family;
            app.appearance.save();
        }
        Msg::TestNotification => {
            return Task::perform(
                tabkeeper::notify::send("Tabkeeper".into(), "Notifications are working.".into()),
                |r| Message::Settings(Msg::TestNotificationDone(r)),
            );
        }
        Msg::TestNotificationDone(r) => {
            app.toast = Some(match r {
                Ok(()) => "Test notification sent.".into(),
                Err(e) => format!("Notification failed: {e}"),
            });
        }
        Msg::Export => {
            let backup = Backup {
                interests: app.interests.clone(),
                states: app.states.clone(),
                events: app.events.clone(),
                settings: app.settings.clone(),
                system: app.system.clone(),
                starred: app.starred.clone(),
            };
            let json = serde_json::to_string_pretty(&backup).unwrap_or_default();
            page.export = Some(text_editor::Content::with_text(&json));
        }
        Msg::ExportAction(action) => {
            // Read-only: allow selecting and scrolling, not editing.
            if let Some(content) = &mut page.export {
                if !action.is_edit() {
                    content.perform(action);
                }
            }
        }
        Msg::CopyExport => {
            if let Some(content) = &page.export {
                app.toast = Some("Export copied to the clipboard.".into());
                return iced::clipboard::write(content.text());
            }
        }
        Msg::ImportAction(action) => page.import.perform(action),
        Msg::Import => {
            let json = page.import.text();
            match serde_json::from_str::<Backup>(&json) {
                Ok(b) => {
                    let n = b.interests.len();
                    restore(app, b);
                    app.page = Page::Settings(Box::new(SettingsPage::new(app)));
                    app.toast = Some(format!("Imported {n} interests."));
                }
                Err(e) => app.toast = Some(format!("Import failed: {e}")),
            }
        }
        Msg::ConfirmReset(on) => page.confirm_reset = on,
        Msg::Reset => {
            restore(app, Backup::default());
            app.page = Page::Settings(Box::new(SettingsPage::new(app)));
            app.toast = Some("All data erased.".into());
        }
    }
    Task::none()
}

/// Replaces all data with a backup.
fn restore(app: &mut App, b: Backup) {
    app.interests = b.interests;
    app.states = b.states;
    app.events = b.events;
    app.settings = b.settings;
    app.system = b.system;
    app.starred = b.starred;
    app.dirty = Dirty::all();
}

pub fn view<'a>(app: &'a App, page: &'a SettingsPage) -> Element<'a> {
    let s = &app.settings;

    let refreshing = panel(
        column![
            h2("Refreshing"),
            row![
                column![
                    small("Concurrent refreshes").style(style::ink_2),
                    input("3", &page.concurrency, |v| msg(Msg::Concurrency(v)))
                        .on_submit(msg(Msg::Normalize))
                        .width(180),
                ]
                .spacing(4),
                column![
                    small("Updates kept in history").style(style::ink_2),
                    input("300", &page.max_events, |v| msg(Msg::MaxEvents(v)))
                        .on_submit(msg(Msg::Normalize))
                        .width(180),
                ]
                .spacing(4),
            ]
            .spacing(12),
            checkbox(s.paused).label("Pause all background refreshing").on_toggle(|on| msg(Msg::Paused(on))),
        ]
        .spacing(12),
    );

    let mut notifications = column![
        h2("Notifications"),
        checkbox(s.notifications)
            .label("Show desktop notifications for updates (per-interest toggle in the editor)")
            .on_toggle(|on| msg(Msg::Notifications(on))),
        row![btn_small("Send test notification", Some(msg(Msg::TestNotification)))],
    ]
    .spacing(12);
    if let Some(e) = &app.notify_error {
        notifications = notifications.push(error_box(format!("Last notification failed: {e}")));
    }

    let a = &app.appearance;
    let ui_font = appearance::ui_font(a.ui_font.as_deref());
    let mut looks = column![
        h2("Appearance"),
        column![
            small("UI scale").style(style::ink_2),
            pick_list(SCALES.iter().copied().map(Scale).collect::<Vec<_>>(), Some(Scale(a.scale)), |s| msg(Msg::Scale(s)))
                .padding([7, 9])
                .width(120)
                .style(style::pick_list)
                    .menu_style(style::menu),
        ]
        .spacing(4),
        row![
            column![
                small("Interface font").style(style::ink_2),
                pick_list(&page.ui_fonts[..], Some(FontChoice(a.ui_font.clone())), |f| msg(Msg::UiFont(f)))
                    .padding([7, 9])
                    .width(Fill)
                    .style(style::pick_list)
                    .menu_style(style::menu),
                text("Bio Rio reRUN · Lördag 3 oktober 10:30").font(ui_font),
                text("The Breakfast Club").font(Font { weight: iced::font::Weight::Bold, ..ui_font }),
            ]
            .spacing(4)
            .width(Fill),
            column![
                small("Editor font").style(style::ink_2),
                pick_list(&page.mono_fonts[..], Some(FontChoice(a.mono_font.clone())), |f| msg(Msg::MonoFont(f)))
                    .padding([7, 9])
                    .width(Fill)
                    .style(style::pick_list)
                    .menu_style(style::menu),
                text("let doc = html(fetch(url)); // 0O 1lI {}").font(appearance::mono_font(a.mono_font.as_deref())).size(13),
            ]
            .spacing(4)
            .width(Fill),
        ]
        .spacing(16),
        muted("Only fonts with Regular and Bold styles are listed."),
    ]
    .spacing(12);
    if a.needs_restart() {
        looks = looks.push(text("Restart Tabkeeper to use the new fonts everywhere.").size(SMALL).style(style::warning));
    }

    let mut data = column![
        h2("Data"),
        muted(format!("Stored as JSON files in {}.", tabkeeper::storage::data_dir().display())),
        row![
            btn_small("Export", Some(msg(Msg::Export))),
            btn_small("Copy to clipboard", page.export.is_some().then_some(msg(Msg::CopyExport))),
        ]
        .spacing(8),
    ]
    .spacing(12);
    if let Some(export) = &page.export {
        data = data.push(
            text_editor(export).on_action(|a| msg(Msg::ExportAction(a))).font(mono()).size(SMALL).height(180).style(style::text_editor),
        );
    }
    let reset: Element = if page.confirm_reset {
        row![
            btn_small("Erase everything", Some(msg(Msg::Reset))).style(style::btn_danger),
            btn_small("Cancel", Some(msg(Msg::ConfirmReset(false)))),
        ]
        .spacing(8)
        .into()
    } else {
        btn_small("Reset all data…", Some(msg(Msg::ConfirmReset(true)))).into()
    };
    data = data
        .push(
            column![
                small("Import (replaces all current data)").style(style::ink_2),
                text_editor(&page.import)
                    .placeholder("Paste an export here")
                    .on_action(|a| msg(Msg::ImportAction(a)))
                    .font(mono())
                    .size(SMALL)
                    .height(100)
                    .style(style::text_editor),
            ]
            .spacing(4),
        )
        .push(
            row![btn_small("Import", (!page.import.text().trim().is_empty()).then_some(msg(Msg::Import))), reset].spacing(8),
        );

    column![page_head("Settings", vec![]), refreshing, panel(notifications), panel(looks), panel(data)].spacing(18).into()
}
