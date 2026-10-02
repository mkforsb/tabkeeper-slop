//! App logic shared by the front ends (Dioxus in `ui/`, Iced in
//! `tabkeeper-iced/`): editing rules, scheduling, backups and display formatting.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::engine;
use crate::model::*;
use crate::script::RunReport;
use crate::templates::TEMPLATES;

/// Everything persisted, for export/import.
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Backup {
    pub interests: Vec<Interest>,
    pub states: HashMap<String, InterestState>,
    pub events: Vec<UpdateEvent>,
    pub settings: Settings,
    pub system: SystemStats,
}

pub fn upsert_interest(list: &mut Vec<Interest>, states: &mut HashMap<String, InterestState>, updated: Interest) {
    match list.iter_mut().find(|i| i.id == updated.id) {
        Some(existing) => {
            if existing.script != updated.script {
                // A different script may produce different item ids; re-baseline
                // instead of reporting everything as new, and run it right away.
                if let Some(st) = states.get_mut(&updated.id) {
                    st.last_output = None;
                    st.seen_ids.clear();
                    st.last_run_at = None;
                    st.fail_streak = 0;
                }
            }
            *existing = updated;
        }
        None => list.push(updated),
    }
}

/// Moves interest `id` to position `to` (clamped to the list).
pub fn move_interest(list: &mut Vec<Interest>, id: &str, to: usize) {
    let Some(from) = list.iter().position(|i| i.id == id) else { return };
    let interest = list.remove(from);
    list.insert(to.min(list.len()), interest);
}

/// Zeroes the failure counts, overall and per interest. Whether an interest
/// is failing right now (its last error and backoff) is left as is.
pub fn reset_failures(states: &mut HashMap<String, InterestState>, sys: &mut SystemStats) {
    sys.failures = 0;
    for st in states.values_mut() {
        st.stats.failures = 0;
    }
}

/// All interests in Dashboard order: those in `order` (as saved in
/// `Settings::dashboard_order`) first, then the rest in their list order.
fn dashboard_order<'a>(interests: &'a [Interest], order: &[String]) -> Vec<&'a Interest> {
    let pos: HashMap<&str, usize> = order.iter().enumerate().map(|(k, id)| (id.as_str(), k)).collect();
    let mut all: Vec<&Interest> = interests.iter().collect();
    all.sort_by_key(|i| pos.get(i.id.as_str()).copied().unwrap_or(usize::MAX));
    all
}

/// The interests shown on the Dashboard, in its order. Disabled ones are left
/// out, but keep their place in `order` for when they're enabled again.
pub fn dashboard_cards<'a>(interests: &'a [Interest], order: &[String]) -> Vec<&'a Interest> {
    dashboard_order(interests, order).into_iter().filter(|i| i.enabled).collect()
}

/// Moves card `id` to where card `target` is; the cards in between shift over.
pub fn move_card(order: &mut Vec<String>, interests: &[Interest], id: &str, target: &str) {
    let mut ids: Vec<String> = dashboard_order(interests, order).into_iter().map(|i| i.id.clone()).collect();
    let (Some(from), Some(to)) = (ids.iter().position(|i| i == id), ids.iter().position(|i| i == target)) else { return };
    let moved = ids.remove(from);
    ids.insert(to, moved);
    *order = ids;
}

/// Puts the Dashboard cards in random order.
pub fn shuffle_cards(order: &mut Vec<String>, interests: &[Interest]) {
    let mut ids: Vec<String> = interests.iter().map(|i| i.id.clone()).collect();
    // Sorting by fresh random keys is a uniform shuffle without a rand dependency.
    ids.sort_by_cached_key(|_| uuid::Uuid::new_v4());
    *order = ids;
}

/// How many more items each click on an output's "+ N more" shows.
pub const SHOW_MORE_STEP: usize = 20;

/// What `run_script` prints to stderr: fetches, the script's log lines and timing.
pub fn run_log(report: &RunReport) -> Vec<String> {
    let mut lines: Vec<String> = report
        .fetches
        .iter()
        .map(|f| format!("{} {} -> {:?} {} bytes {} ms {}", f.method, f.url, f.status, f.bytes, f.ms, f.error.clone().unwrap_or_default()))
        .collect();
    lines.extend(report.logs.iter().map(|l| format!("log: {l}")));
    lines.push(format!("{} rounds, {} ms", report.rounds, report.duration_ms));
    lines
}

/// A run as `run_script` prints it: [`run_log`], then the output as JSON or the error.
pub fn headless_dump(report: &RunReport) -> String {
    let mut lines = run_log(report);
    lines.push(match &report.result {
        Ok(out) => serde_json::to_string_pretty(out).unwrap_or_default(),
        Err(e) => format!("error: {e}"),
    });
    lines.join("\n")
}

/// The example interests offered on an empty dashboard.
pub fn example_interests() -> Vec<Interest> {
    let examples = ["youtube", "soundcloud-tracks", "soundcloud-bio", "instagram", "biorio", "bioaspen", "slakthuset"];
    examples
        .into_iter()
        .filter_map(|key| TEMPLATES.iter().find(|t| t.key == key))
        .map(|t| Interest {
            name: t.name.into(),
            script: t.script.into(),
            interval_mins: t.interval_mins,
            // Instagram needs a session cookie pasted into its script first.
            enabled: t.key != "instagram",
            created_at: now_ms(),
            ..Default::default()
        })
        .collect()
}

/// Ids of enabled interests that are due for a refresh, most overdue first,
/// limited to the concurrency slots not taken by `running`.
pub fn due_interests(
    interests: &[Interest],
    states: &HashMap<String, InterestState>,
    running: &HashSet<String>,
    concurrency: u32,
    now: Millis,
) -> Vec<String> {
    let slots = (concurrency.max(1) as usize).saturating_sub(running.len());
    let mut due: Vec<(i64, String)> = interests
        .iter()
        .filter(|i| i.enabled && !running.contains(&i.id))
        .map(|i| (engine::next_due(i, &states.get(&i.id).cloned().unwrap_or_default()), i.id.clone()))
        .filter(|(at, _)| *at <= now)
        .collect();
    due.sort();
    due.into_iter().take(slots).map(|(_, id)| id).collect()
}

// ---- display formatting ----

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

/// Machine-readable dates (RFC 3339 / RFC 2822, as feeds use) in local time;
/// anything else is shown as the script wrote it.
pub fn fmt_item_date(s: &str) -> String {
    use chrono::{DateTime, Local};
    DateTime::parse_from_rfc3339(s.trim())
        .or_else(|_| DateTime::parse_from_rfc2822(s.trim()))
        .map(|d| d.with_timezone(&Local).format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|_| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_interest_reorders() {
        let ids = |list: &[Interest]| list.iter().map(|i| i.id.clone()).collect::<Vec<_>>().join("");
        let mut list: Vec<Interest> = "abcd".chars().map(|c| Interest { id: c.into(), ..Default::default() }).collect();
        move_interest(&mut list, "a", 2);
        assert_eq!(ids(&list), "bcad");
        move_interest(&mut list, "d", 0);
        assert_eq!(ids(&list), "dbca");
        move_interest(&mut list, "b", 99);
        assert_eq!(ids(&list), "dcab");
        move_interest(&mut list, "missing", 0);
        assert_eq!(ids(&list), "dcab");
    }

    #[test]
    fn dashboard_order_and_moves() {
        let interests: Vec<Interest> =
            "abcde".chars().map(|c| Interest { id: c.into(), enabled: c != 'b', ..Default::default() }).collect();
        let cards = |order: &[String]| dashboard_cards(&interests, order).iter().map(|i| i.id.clone()).collect::<String>();
        // Unlisted interests follow in list order; unknown ids are ignored.
        let mut order: Vec<String> = ["d", "gone", "b", "a"].map(String::from).to_vec();
        assert_eq!(cards(&order), "dace");
        move_card(&mut order, &interests, "d", "c");
        assert_eq!(cards(&order), "acde");
        // Hidden, disabled "b" keeps its place for when it's enabled again.
        assert_eq!(order.join(""), "bacde");
        move_card(&mut order, &interests, "e", "a");
        assert_eq!(cards(&order), "eacd");
        shuffle_cards(&mut order, &interests);
        let mut sorted = order.clone();
        sorted.sort();
        assert_eq!(sorted.join(""), "abcde");
    }

    #[test]
    fn reset_failures_keeps_failing_state() {
        let mut sys = SystemStats { failures: 5, refreshes: 9, ..Default::default() };
        let mut st = InterestState { fail_streak: 2, last_error: Some("boom".into()), ..Default::default() };
        st.stats.failures = 3;
        let mut states = HashMap::from([("a".to_string(), st)]);
        reset_failures(&mut states, &mut sys);
        assert_eq!((sys.failures, sys.refreshes), (0, 9));
        let st = &states["a"];
        assert_eq!((st.stats.failures, st.fail_streak, st.last_error.as_deref()), (0, 2, Some("boom")));
    }

    #[test]
    fn headless_dump_matches_run_script() {
        let report = |result| RunReport { result, logs: vec!["hi".into()], fetches: vec![], duration_ms: 7, rounds: 1 };
        assert_eq!(headless_dump(&report(Err("boom".into()))), "log: hi\n1 rounds, 7 ms\nerror: boom");
        let out = Output { title: "T".into(), ..Default::default() };
        assert!(headless_dump(&report(Ok(out))).starts_with("log: hi\n1 rounds, 7 ms\n{\n  \"title\": \"T\","));
    }
}
