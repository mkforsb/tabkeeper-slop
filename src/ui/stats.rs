use dioxus::prelude::*;

use super::state::*;
use super::{fmt_ago, fmt_bytes, fmt_count, fmt_duration, fmt_time, Route};

#[component]
pub fn Stats() -> Element {
    let sys = SYSTEM();
    let now = NOW();
    let uptime = now - SESSION_STARTED();
    let interests = INTERESTS();
    let states = STATES();
    let fail_rate = if sys.refreshes > 0 { sys.failures as f64 * 100.0 / sys.refreshes as f64 } else { 0.0 };
    let storage = storage_size();
    let running = RUNNING.read().len();

    rsx! {
        div { class: "page-head", h1 { "Stats" } }

        div { class: "tiles",
            StatTile { label: "Refreshes", value: fmt_count(sys.refreshes), sub: format!("{} fetches", fmt_count(sys.fetches)) }
            StatTile { label: "Failures", value: fmt_count(sys.failures), sub: format!("{fail_rate:.1}% of refreshes") }
            StatTile { label: "Updates caught", value: fmt_count(sys.updates), sub: format!("{} in history", EVENTS.read().len()) }
            StatTile { label: "Data fetched", value: fmt_bytes(sys.bytes), sub: format!("{} stored locally", fmt_bytes(storage as u64)) }
        }

        div { class: "panel",
            h2 { "System" }
            dl { class: "kv",
                dt { "Session uptime" } dd { "{fmt_duration(uptime)}" }
                dt { "Worker" } dd { if SETTINGS.read().paused { "paused" } else { "running" } ", {running} refresh(es) in flight, concurrency {SETTINGS.read().concurrency}" }
                dt { "First started" } dd { "{fmt_time(sys.first_started_at)}" }
                dt { "Launches" } dd { "{fmt_count(sys.launches)}" }
                dt { "Interests" } dd { "{interests.len()} ({interests.iter().filter(|i| i.enabled).count()} enabled)" }
                dt { "Platform" } dd { if cfg!(target_arch = "wasm32") { "web (localStorage)" } else { "desktop" } }
            }
        }

        div { class: "panel table-wrap",
            h2 { "Per interest" }
            table { class: "table",
                thead {
                    tr {
                        th { "Name" }
                        th { class: "num", "Refreshes" }
                        th { class: "num", "Failures" }
                        th { class: "num", "Fail rate" }
                        th { class: "num", "Updates" }
                        th { class: "num", "Avg time" }
                        th { class: "num", "Fetched" }
                        th { "Last success" }
                        th { "Last update" }
                    }
                }
                tbody {
                    for i in interests {
                        {
                            let st = states.get(&i.id).cloned().unwrap_or_default();
                            let s = &st.stats;
                            let rate = if s.refreshes > 0 { s.failures as f64 * 100.0 / s.refreshes as f64 } else { 0.0 };
                            let avg = s.total_duration_ms.checked_div(s.refreshes).unwrap_or(0);
                            rsx! {
                                tr { key: "{i.id}",
                                    td { Link { to: Route::Editor { id: i.id.clone() }, "{i.name}" } }
                                    td { class: "num", "{fmt_count(s.refreshes)}" }
                                    td { class: "num", "{fmt_count(s.failures)}" }
                                    td { class: "num",
                                        div { class: "meter", title: "{rate:.1}%",
                                            div { class: if rate >= 50.0 { "meter-fill critical" } else if rate > 0.0 { "meter-fill warning" } else { "meter-fill" }, style: "width: {rate.max(0.0).min(100.0)}%" }
                                        }
                                        "{rate:.0}%"
                                    }
                                    td { class: "num", "{fmt_count(s.updates)}" }
                                    td { class: "num", "{avg} ms" }
                                    td { class: "num", "{fmt_bytes(s.bytes)}" }
                                    td { class: "muted", "{fmt_ago(st.last_success_at, now)}" }
                                    td { class: "muted", "{fmt_ago(st.last_update_at, now)}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn StatTile(label: String, value: String, sub: String) -> Element {
    rsx! {
        div { class: "tile",
            div { class: "tile-label", "{label}" }
            div { class: "tile-value", "{value}" }
            div { class: "tile-sub", "{sub}" }
        }
    }
}
