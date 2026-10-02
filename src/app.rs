//! App logic shared by the front ends (Dioxus in `ui/`, Iced in
//! `tabkeeper-iced/`): editing rules, scheduling, backups and display formatting.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::engine;
use crate::model::*;
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

/// Moves interest `id` to position `to` (clamped to the list). The list's
/// order is also the dashboard's.
pub fn move_interest(list: &mut Vec<Interest>, id: &str, to: usize) {
    let Some(from) = list.iter().position(|i| i.id == id) else { return };
    let interest = list.remove(from);
    list.insert(to.min(list.len()), interest);
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
}
