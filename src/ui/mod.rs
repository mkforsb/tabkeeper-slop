mod components;
mod dashboard;
mod editor;
mod interests;
mod settings;
pub mod state;
mod stats;
mod updates;
mod worker;

use dioxus::prelude::*;

use dashboard::Dashboard;
use editor::Editor;
use interests::Interests;
use settings::SettingsPage;
use state::*;
use stats::Stats;
use updates::Updates;

const CSS: &str = include_str!("../../assets/main.css");

#[derive(Routable, Clone, PartialEq, Debug)]
#[rustfmt::skip]
pub enum Route {
    #[layout(Shell)]
        #[route("/")]
        Dashboard {},
        #[route("/interests")]
        Interests {},
        #[route("/interest/:id")]
        Editor { id: String },
        #[route("/updates")]
        Updates {},
        #[route("/stats")]
        Stats {},
        #[route("/settings")]
        SettingsPage {},
}

#[component]
pub fn App() -> Element {
    use_persistence();
    worker::use_worker();
    rsx! {
        style { {CSS} }
        Router::<Route> {}
    }
}

#[component]
fn Shell() -> Element {
    let paused = SETTINGS.read().paused;
    let running = RUNNING.read().len();
    let unread = unread_count(None);
    let total = INTERESTS.read().len();
    let enabled = INTERESTS.read().iter().filter(|i| i.enabled).count();

    rsx! {
        div { class: "shell",
            nav { class: "sidebar",
                div { class: "brand", span { class: "brand-mark", "◉" } "Tabkeeper" }
                Link { class: "nav", active_class: "active", to: Route::Dashboard {}, "Dashboard" }
                Link { class: "nav", active_class: "active", to: Route::Interests {}, "Interests"
                    span { class: "nav-count", "{total}" }
                }
                Link { class: "nav", active_class: "active", to: Route::Updates {}, "Updates"
                    if unread > 0 { span { class: "nav-badge", "{unread}" } }
                }
                Link { class: "nav", active_class: "active", to: Route::Stats {}, "Stats" }
                Link { class: "nav", active_class: "active", to: Route::SettingsPage {}, "Settings" }
                div { class: "sidebar-foot",
                    div { class: "worker-status",
                        if paused {
                            span { class: "status status-warning", "⏸ Paused" }
                        } else if running > 0 {
                            span { class: "status status-running", span { class: "spinner" } "Refreshing {running}" }
                        } else {
                            span { class: "status status-good", "✓ Watching {enabled}" }
                        }
                    }
                    button {
                        class: "btn btn-small",
                        onclick: move |_| { let mut s = SETTINGS.write(); s.paused = !s.paused; },
                        if paused { "Resume" } else { "Pause" }
                    }
                }
            }
            main { class: "content", Outlet::<Route> {} }
        }
    }
}

// ---- formatting helpers shared by the pages ----

pub fn fmt_ago(at: Option<i64>, now: i64) -> String {
    let Some(at) = at else { return "never".into() };
    let d = (now - at) / 1000;
    match d {
        i64::MIN..=9 => "just now".into(),
        10..=59 => format!("{d}s ago"),
        60..=3599 => format!("{}m ago", d / 60),
        3600..=86_399 => format!("{}h ago", d / 3600),
        _ => format!("{}d ago", d / 86_400),
    }
}

pub fn fmt_in(at: i64, now: i64) -> String {
    let d = (at - now) / 1000;
    match d {
        i64::MIN..=0 => "now".into(),
        1..=59 => format!("in {d}s"),
        60..=3599 => format!("in {}m", d / 60),
        3600..=86_399 => format!("in {}h {}m", d / 3600, d % 3600 / 60),
        _ => format!("in {}d", d / 86_400),
    }
}

pub fn fmt_time(at: i64) -> String {
    use chrono::{Local, TimeZone};
    Local.timestamp_millis_opt(at).single().map(|t| t.format("%Y-%m-%d %H:%M").to_string()).unwrap_or_default()
}

pub fn fmt_duration(ms: i64) -> String {
    let s = ms / 1000;
    if s < 60 {
        format!("{s}s")
    } else if s < 3600 {
        format!("{}m", s / 60)
    } else if s < 86_400 {
        format!("{}h {}m", s / 3600, s % 3600 / 60)
    } else {
        format!("{}d {}h", s / 86_400, s % 86_400 / 3600)
    }
}

pub fn fmt_bytes(b: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = b as f64;
    let mut u = 0;
    while v >= 1024.0 && u < UNITS.len() - 1 {
        v /= 1024.0;
        u += 1;
    }
    if u == 0 {
        format!("{b} B")
    } else {
        format!("{v:.1} {}", UNITS[u])
    }
}

/// 1,284 / 12.9K / 4.2M
pub fn fmt_count(n: u64) -> String {
    match n {
        0..=9_999 => {
            let s = n.to_string();
            let mut out = String::new();
            for (i, c) in s.chars().enumerate() {
                if i > 0 && (s.len() - i).is_multiple_of(3) {
                    out.push(',');
                }
                out.push(c);
            }
            out
        }
        10_000..=999_999 => format!("{:.1}K", n as f64 / 1000.0),
        _ => format!("{:.1}M", n as f64 / 1_000_000.0),
    }
}
