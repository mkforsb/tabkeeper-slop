//! Tabkeeper desktop app built with Iced: an alternative front end to the
//! Dioxus app in the root crate, over the same `tabkeeper` core and data files.

mod appearance;
mod images;
mod pages;
mod style;
mod widgets;

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;
use std::time::Duration;

use iced::theme::Mode;
use iced::widget::operation::{snap_to, RelativeOffset};
use iced::widget::{bottom_right, button, column, container, row, rule, scrollable, space, stack, text, Id};
use iced::{window, Center, Fill, Padding, Size, Subscription, Task, Theme};
use tabkeeper::app;
use tabkeeper::engine;
use tabkeeper::model::*;
use tabkeeper::platform::sleep_ms;
use tabkeeper::script::{self, RunInput, RunReport};
use tabkeeper::storage;

use appearance::Appearance;
use images::Images;
use pages::{editor, settings, Page};
use widgets::{muted, Element};

const TICK_MS: u64 = 5_000;
const SIDEBAR_WIDTH: f32 = 200.0;
const WINDOW_SIZE: Size = Size::new(1280.0, 860.0);
static CONTENT: LazyLock<Id> = LazyLock::new(|| Id::new("content"));

pub fn main() -> iced::Result {
    // iced multiplies the window size by the UI scale; divide so the window
    // opens at the same size on screen whatever the scale.
    let scale = appearance::init().scale_factor();
    iced::application(App::new, App::update, App::view)
        .title("Tabkeeper")
        .theme(App::theme)
        .subscription(App::subscription)
        .scale_factor(|app: &App| app.appearance.scale_factor())
        .window(window::Settings {
            size: Size::new(WINDOW_SIZE.width / scale, WINDOW_SIZE.height / scale),
            min_size: Some(Size::new(900.0, 560.0)),
            ..Default::default()
        })
        .settings(iced::Settings { default_font: style::regular(), default_text_size: 14.into(), ..Default::default() })
        .run()
}

#[derive(Debug, Clone, PartialEq)]
pub enum Route {
    Dashboard,
    Interests,
    /// An interest's id, or "new".
    Editor(String),
    Updates,
    Stats,
    Settings,
}

#[derive(Debug, Clone)]
pub enum Message {
    Navigate(Route),
    Tick,
    ThemeChanged(Mode),
    Resized(Size),
    Run(String),
    RefreshAll,
    ShuffleCards,
    ResetFailures,
    /// Interest id, the script that ran, and its report.
    RunFinished(String, String, Box<RunReport>),
    Notified(Result<(), String>),
    TogglePaused,
    MarkRead(Option<String>),
    MarkEventRead(String),
    AddExamples,
    SetEnabled(String, bool),
    SetReversed(String, bool),
    ConfirmDelete(Option<String>),
    /// Reordering on the Interests page and the Dashboard.
    DragStart(String),
    DragOver(Option<usize>),
    DragEnd,
    Delete(String),
    ConfirmClear(bool),
    ClearEvents,
    ShowMoreEvents,
    /// An output list's key and how many extra items it shows.
    ShowMore(String, usize),
    OpenUrl(String),
    LoadImage(String),
    ImageLoaded(String, Result<Vec<u8>, String>),
    DismissToast,
    Editor(editor::Msg),
    Settings(settings::Msg),
}

/// Which persisted values changed and need writing.
#[derive(Default)]
pub struct Dirty {
    pub interests: bool,
    pub states: bool,
    pub events: bool,
    pub settings: bool,
    pub system: bool,
}

impl Dirty {
    pub fn all() -> Self {
        Self { interests: true, states: true, events: true, settings: true, system: true }
    }
}

pub struct App {
    pub interests: Vec<Interest>,
    pub states: HashMap<String, InterestState>,
    /// Newest first.
    pub events: Vec<UpdateEvent>,
    pub settings: Settings,
    pub system: SystemStats,
    /// Ids of interests whose script is running right now.
    pub running: HashSet<String>,
    pub session_started: Millis,
    /// A coarse clock so relative times ("5m ago") refresh.
    pub now: Millis,
    /// Last notification error, shown in Settings.
    pub notify_error: Option<String>,
    pub images: Images,
    pub page: Page,
    pub dirty: Dirty,
    pub toast: Option<String>,
    /// Extra items shown in an output's list after clicking "+ N more", by
    /// list. Reset when changing pages.
    pub more: HashMap<String, usize>,
    /// As edited in Settings; the fonts in use are fixed at launch.
    pub appearance: Appearance,
    mode: Mode,
    /// In screen pixels, unaffected by the UI scale.
    window_width: f32,
}

fn drag_release(event: iced::Event, _: iced::event::Status, _: window::Id) -> Option<Message> {
    matches!(event, iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left))).then_some(Message::DragEnd)
}

static NO_STATE: LazyLock<InterestState> = LazyLock::new(InterestState::default);

impl App {
    fn new() -> (Self, Task<Message>) {
        let mut system: SystemStats = storage::load("system");
        if system.first_started_at == 0 {
            system.first_started_at = now_ms();
        }
        system.launches += 1;
        storage::save("system", &system);

        let app = Self {
            interests: storage::load("interests"),
            states: storage::load("states"),
            events: storage::load("events"),
            settings: storage::load("settings"),
            system,
            running: HashSet::new(),
            session_started: now_ms(),
            now: now_ms(),
            notify_error: None,
            images: Images::default(),
            page: Page::Dashboard { drag: None },
            dirty: Dirty::default(),
            toast: None,
            more: HashMap::new(),
            appearance: appearance::init().clone(),
            mode: Mode::None,
            window_width: WINDOW_SIZE.width,
        };
        let boot = Task::batch([
            iced::system::theme().map(Message::ThemeChanged),
            Task::perform(sleep_ms(1_500), |_| Message::Tick),
        ]);
        (app, boot)
    }

    fn theme(&self) -> Theme {
        style::theme(self.mode)
    }

    fn subscription(&self) -> Subscription<Message> {
        let dragging = matches!(self.page, Page::Interests { drag: Some(_), .. } | Page::Dashboard { drag: Some(_) });
        Subscription::batch([
            iced::time::every(Duration::from_millis(TICK_MS)).map(|_| Message::Tick),
            iced::system::theme_changes().map(Message::ThemeChanged),
            window::resize_events().map(|(_, size)| Message::Resized(size)),
            // The drop happens wherever the button is released.
            if dragging { iced::event::listen_with(drag_release) } else { Subscription::none() },
        ])
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        let task = self.handle(message);
        self.flush();
        task
    }

    fn handle(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Navigate(route) => {
                self.more.clear();
                self.page = Page::open(self, route);
                return snap_to(CONTENT.clone(), RelativeOffset::START);
            }
            Message::Tick => {
                self.now = now_ms();
                if self.settings.paused {
                    return Task::none();
                }
                let due = app::due_interests(&self.interests, &self.states, &self.running, self.settings.concurrency, self.now);
                return Task::batch(due.into_iter().map(|id| self.start_run(id)).collect::<Vec<_>>());
            }
            Message::ThemeChanged(mode) => self.mode = mode,
            // Reported in layout units, i.e. divided by the UI scale.
            Message::Resized(size) => self.window_width = size.width * self.appearance.scale_factor(),
            Message::Run(id) => return self.start_run(id),
            Message::RefreshAll => {
                let ids: Vec<String> = self.interests.iter().filter(|i| i.enabled).map(|i| i.id.clone()).collect();
                return Task::batch(ids.into_iter().map(|id| self.start_run(id)).collect::<Vec<_>>());
            }
            Message::RunFinished(id, script, report) => return self.finish_run(id, script, *report),
            Message::Notified(result) => self.notify_error = result.err(),
            Message::TogglePaused => {
                self.settings.paused = !self.settings.paused;
                self.dirty.settings = true;
            }
            Message::MarkRead(interest_id) => self.mark_read(interest_id.as_deref()),
            Message::MarkEventRead(id) => {
                if let Some(e) = self.events.iter_mut().find(|e| e.id == id) {
                    e.read = true;
                    self.dirty.events = true;
                }
            }
            Message::AddExamples => {
                for i in app::example_interests() {
                    self.upsert_interest(i);
                }
            }
            Message::SetEnabled(id, enabled) => {
                if let Some(i) = self.interests.iter_mut().find(|i| i.id == id) {
                    i.enabled = enabled;
                    self.dirty.interests = true;
                }
            }
            Message::SetReversed(id, on) => {
                if let Some(i) = self.interests.iter_mut().find(|i| i.id == id) {
                    i.reverse_order = on;
                    self.dirty.interests = true;
                }
            }
            Message::ConfirmDelete(id) => {
                if let Page::Interests { confirm_delete, .. } = &mut self.page {
                    *confirm_delete = id;
                }
            }
            Message::DragStart(id) => {
                if let Page::Interests { drag, .. } | Page::Dashboard { drag } = &mut self.page {
                    *drag = Some(pages::Drag { id, over: None });
                }
            }
            Message::DragOver(over) => {
                if let Page::Interests { drag: Some(d), .. } | Page::Dashboard { drag: Some(d) } = &mut self.page {
                    d.over = over;
                }
            }
            Message::DragEnd => match &mut self.page {
                Page::Interests { drag, .. } => {
                    if let Some(pages::Drag { id, over: Some(to) }) = drag.take() {
                        app::move_interest(&mut self.interests, &id, to);
                        self.dirty.interests = true;
                    }
                }
                Page::Dashboard { drag } => {
                    if let Some(pages::Drag { id, over: Some(to) }) = drag.take() {
                        let cards = app::dashboard_cards(&self.interests, &self.settings.dashboard_order);
                        if let Some(target) = cards.get(to).map(|i| i.id.clone()) {
                            app::move_card(&mut self.settings.dashboard_order, &self.interests, &id, &target);
                            self.dirty.settings = true;
                        }
                    }
                }
                _ => {}
            },
            Message::ResetFailures => {
                app::reset_failures(&mut self.states, &mut self.system);
                self.dirty.states = true;
                self.dirty.system = true;
            }
            Message::ShuffleCards => {
                app::shuffle_cards(&mut self.settings.dashboard_order, &self.interests);
                self.dirty.settings = true;
            }
            Message::Delete(id) => self.delete_interest(&id),
            Message::ConfirmClear(on) => {
                if let Page::Updates { confirm_clear, .. } = &mut self.page {
                    *confirm_clear = on;
                }
            }
            Message::ClearEvents => {
                self.events.clear();
                self.dirty.events = true;
                if let Page::Updates { confirm_clear, .. } = &mut self.page {
                    *confirm_clear = false;
                }
            }
            Message::ShowMoreEvents => {
                if let Page::Updates { shown, .. } = &mut self.page {
                    *shown += pages::updates::PAGE_SIZE;
                }
            }
            Message::OpenUrl(url) => {
                if let Err(e) = open::that_detached(&url) {
                    self.toast = Some(format!("Couldn't open {url}: {e}"));
                }
            }
            Message::LoadImage(url) => return self.images.load(url),
            Message::ImageLoaded(url, result) => self.images.loaded(url, result),
            Message::DismissToast => self.toast = None,
            Message::ShowMore(key, 0) => _ = self.more.remove(&key),
            Message::ShowMore(key, extra) => _ = self.more.insert(key, extra),
            Message::Editor(msg) => return editor::update(self, msg),
            Message::Settings(msg) => return settings::update(self, msg),
        }
        Task::none()
    }

    /// Writes changed values to storage, like the Dioxus app's persistence effects.
    fn flush(&mut self) {
        let d = std::mem::take(&mut self.dirty);
        if d.interests {
            storage::save("interests", &self.interests);
        }
        if d.states {
            storage::save("states", &self.states);
        }
        if d.events {
            storage::save("events", &self.events);
        }
        if d.settings {
            storage::save("settings", &self.settings);
        }
        if d.system {
            storage::save("system", &self.system);
        }
    }

    // ---- background refreshing ----

    pub fn start_run(&mut self, id: String) -> Task<Message> {
        let Some(interest) = self.interest(&id).cloned() else { return Task::none() };
        if !self.running.insert(id.clone()) {
            return Task::none();
        }
        let prev = self.states.get(&id).and_then(|s| s.last_output.clone());
        let input = RunInput {
            script: interest.script.clone(),
            name: interest.name,
            prev,
            cors_proxy: self.settings.cors_proxy.clone(),
        };
        let ran = interest.script;
        Task::perform(script::run(input), move |report| Message::RunFinished(id, ran, Box::new(report)))
    }

    fn finish_run(&mut self, id: String, ran: String, report: RunReport) -> Task<Message> {
        self.running.remove(&id);
        // The interest may have been deleted or re-scripted while running.
        let Some(current) = self.interest(&id).cloned() else { return Task::none() };
        if current.script != ran {
            return Task::none();
        }
        let st = self.states.entry(id).or_default();
        let event = engine::apply_run(&current, st, &mut self.system, &report, now_ms());
        self.dirty.states = true;
        self.dirty.system = true;
        let Some(event) = event else { return Task::none() };

        let summary = event.summary.clone();
        self.events.insert(0, event);
        self.events.truncate(self.settings.max_events.max(10) as usize);
        self.dirty.events = true;
        if self.settings.notifications && current.notify {
            Task::perform(tabkeeper::notify::send(current.name, summary), Message::Notified)
        } else {
            Task::none()
        }
    }

    // ---- data helpers ----

    pub fn interest(&self, id: &str) -> Option<&Interest> {
        self.interests.iter().find(|i| i.id == id)
    }

    pub fn state(&self, id: &str) -> &InterestState {
        self.states.get(id).unwrap_or(&NO_STATE)
    }

    /// Inserts or replaces an interest; a changed script re-baselines its state.
    pub fn upsert_interest(&mut self, interest: Interest) {
        app::upsert_interest(&mut self.interests, &mut self.states, interest);
        self.dirty.interests = true;
        self.dirty.states = true;
    }

    pub fn delete_interest(&mut self, id: &str) {
        self.interests.retain(|i| i.id != id);
        self.states.remove(id);
        self.events.retain(|e| e.interest_id != id);
        self.dirty.interests = true;
        self.dirty.states = true;
        self.dirty.events = true;
    }

    pub fn unread_count(&self, interest_id: Option<&str>) -> usize {
        self.events.iter().filter(|e| !e.read && interest_id.is_none_or(|id| e.interest_id == id)).count()
    }

    fn mark_read(&mut self, interest_id: Option<&str>) {
        for e in self.events.iter_mut().filter(|e| interest_id.is_none_or(|id| e.interest_id == id)) {
            e.read = true;
        }
        self.dirty.events = true;
    }

    pub fn theme_is_dark(&self) -> bool {
        self.mode == Mode::Dark
    }

    /// Width available to page content (window minus sidebar and padding),
    /// in layout units, which the UI scale enlarges.
    pub fn content_width(&self) -> f32 {
        (self.window_width / self.appearance.scale_factor() - SIDEBAR_WIDTH - 1.0).min(1500.0) - 56.0
    }

    // ---- view ----

    fn view(&self) -> Element<'_> {
        let content = pages::view(self);
        let main = row![
            self.sidebar(),
            rule::vertical(1).style(style::rule),
            scrollable(container(content).padding([22, 28]).width(Fill).max_width(1500))
                .id(CONTENT.clone())
                .width(Fill)
                .height(Fill),
        ];
        let mut layers = stack![main];
        if let Some(modal) = pages::modal(self) {
            layers = layers.push(modal);
        }
        if let Some(t) = &self.toast {
            layers = layers.push(
                bottom_right(button(text(t.as_str())).padding([10, 14]).style(style::toast_button).on_press(Message::DismissToast))
                    .padding(20),
            );
        }
        layers.into()
    }

    fn sidebar(&self) -> Element<'_> {
        let unread = self.unread_count(None);
        let total = self.interests.len();
        let enabled = self.interests.iter().filter(|i| i.enabled).count();
        let running = self.running.len();

        let nav = |label: &'static str, route: Route, extra: Option<Element<'static>>| {
            let active = self.page.is(&route);
            let mut r = row![text(label)].align_y(Center);
            if let Some(extra) = extra {
                r = r.push(space::horizontal()).push(extra);
            }
            button(r).width(Fill).padding([7, 10]).style(style::nav(active)).on_press(Message::Navigate(route))
        };
        let badge = (unread > 0).then(|| {
            container(text(unread.to_string()).size(11.5).font(style::bold()))
                .padding([1, 7])
                .style(style::nav_badge)
                .into()
        });

        let status: Element = if self.settings.paused {
            text("⏸ Paused").size(style::SMALL).style(style::warning).into()
        } else if running > 0 {
            text(format!("↻ Refreshing {running}")).size(style::SMALL).style(style::accent).into()
        } else {
            text(format!("✓ Watching {enabled}")).size(style::SMALL).style(style::good).into()
        };

        container(
            column![
                row![text("◉").size(16).style(style::accent), text("Tabkeeper").size(16).font(style::bold())]
                    .spacing(8)
                    .padding(Padding { top: 4.0, right: 10.0, bottom: 16.0, left: 10.0 }),
                nav("Dashboard", Route::Dashboard, None),
                nav("Interests", Route::Interests, Some(muted(total.to_string()).into())),
                nav("Updates", Route::Updates, badge),
                nav("Stats", Route::Stats, None),
                nav("Settings", Route::Settings, None),
                space::vertical(),
                column![
                    status,
                    widgets::btn_small(if self.settings.paused { "Resume" } else { "Pause" }, Some(Message::TogglePaused)),
                ]
                .spacing(8)
                .padding([0, 6]),
            ]
            .spacing(2),
        )
        .padding([18, 12])
        .width(SIDEBAR_WIDTH)
        .height(Fill)
        .style(style::sidebar)
        .into()
    }
}
