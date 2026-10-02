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

pub use tabkeeper::app::{fmt_ago, fmt_bytes, fmt_count, fmt_duration, fmt_in, fmt_time};

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
