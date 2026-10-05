//! Change detection and bookkeeping after a script run. Pure functions, so the
//! UI's worker loop stays thin and this logic is testable.

use std::collections::HashSet;

use crate::model::{Interest, InterestState, Item, Millis, Output, SystemStats, UpdateEvent};
use crate::script::RunReport;

const MAX_SEEN: usize = 2000;
const MAX_EVENT_ITEMS: usize = 10;
const MAX_LOGS_KEPT: usize = 50;

#[derive(Clone, Debug, PartialEq)]
pub struct Change {
    pub summary: String,
    pub items: Vec<Item>,
}

fn clip(s: &str, n: usize) -> String {
    let s = s.trim();
    if s.chars().count() <= n {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(n).collect::<String>().trim_end())
    }
}

fn item_label(i: &Item) -> &str {
    [&i.title, &i.text, &i.url, &i.id].into_iter().find(|s| !s.is_empty()).map(|s| s.as_str()).unwrap_or("item")
}

/// Compares a new output against the previous one. The first successful run
/// (no `prev`) only establishes a baseline and never reports a change.
pub fn detect(prev: Option<&Output>, seen: &[String], new: &Output) -> Option<Change> {
    let prev = prev?;

    if let Some(key) = &new.key {
        return (prev.key.as_ref() != Some(key)).then(|| Change {
            summary: format!("Changed: {}", clip(if new.title.is_empty() { &new.text } else { &new.title }, 120)),
            items: Vec::new(),
        });
    }

    let seen: HashSet<&str> = seen.iter().map(String::as_str).collect();
    let mut fresh: Vec<Item> = Vec::new();
    for item in &new.items {
        if !seen.contains(item.id.as_str()) && !fresh.iter().any(|f| f.id == item.id) {
            fresh.push(item.clone());
        }
    }
    let text_changed = prev.text.trim() != new.text.trim() && !(prev.text.trim().is_empty() && new.text.trim().is_empty());

    let mut parts = Vec::new();
    if !fresh.is_empty() {
        let names: Vec<String> = fresh.iter().take(3).map(|i| clip(item_label(i), 60)).collect();
        let more = if fresh.len() > 3 { format!(" +{} more", fresh.len() - 3) } else { String::new() };
        parts.push(format!("{} new: {}{more}", fresh.len(), names.join(", ")));
    }
    if text_changed {
        parts.push(format!("Text changed: {}", clip(&new.text, 160)));
    }
    if parts.is_empty() {
        return None;
    }
    fresh.truncate(MAX_EVENT_ITEMS);
    Some(Change { summary: parts.join(" · "), items: fresh })
}

/// Records the output's item ids in `seen`, ordered by when they were last
/// seen: ids in this output move to the end, so the cap drops ids that have
/// left the output, never current ones (unless the output alone exceeds it).
pub fn merge_seen(seen: &mut Vec<String>, out: &Output) {
    let mut current = HashSet::new();
    let ids: Vec<&String> = out.items.iter().map(|i| &i.id).filter(|id| !id.is_empty() && current.insert(id.as_str())).collect();
    seen.retain(|id| !current.contains(id.as_str()));
    seen.extend(ids.into_iter().cloned());
    if seen.len() > MAX_SEEN {
        let excess = seen.len() - MAX_SEEN;
        seen.drain(..excess);
    }
}

/// Folds a run report into an interest's state and the system stats.
/// `delay_ms` is added to the next scheduled refresh (see [`next_due`]).
/// Returns the update event if a change was caught.
pub fn apply_run(
    interest: &Interest,
    state: &mut InterestState,
    sys: &mut SystemStats,
    report: &RunReport,
    now: Millis,
    delay_ms: Millis,
) -> Option<UpdateEvent> {
    let fetched_bytes: u64 = report.fetches.iter().map(|f| f.bytes as u64).sum();
    state.last_run_at = Some(now);
    state.delay_ms = delay_ms;
    state.last_duration_ms = report.duration_ms;
    state.stats.refreshes += 1;
    state.stats.fetches += report.fetches.len() as u64;
    state.stats.bytes += fetched_bytes;
    state.stats.total_duration_ms += report.duration_ms;
    sys.refreshes += 1;
    sys.fetches += report.fetches.len() as u64;
    sys.bytes += fetched_bytes;

    let mut logs: Vec<String> = report
        .fetches
        .iter()
        .map(|f| match (&f.error, f.status) {
            (Some(e), _) => format!("{} {} → error: {e} ({} ms)", f.method, f.url, f.ms),
            (None, s) => format!("{} {} → {} ({} bytes, {} ms)", f.method, f.url, s.unwrap_or(0), f.bytes, f.ms),
        })
        .collect();
    logs.extend(report.logs.iter().cloned());
    logs.truncate(MAX_LOGS_KEPT);
    state.last_logs = logs;

    match &report.result {
        Err(e) => {
            state.last_error = Some(e.clone());
            state.fail_streak += 1;
            state.stats.failures += 1;
            sys.failures += 1;
            None
        }
        Ok(out) => {
            state.last_error = None;
            state.fail_streak = 0;
            state.last_success_at = Some(now);
            let change = detect(state.last_output.as_ref(), &state.seen_ids, out);
            merge_seen(&mut state.seen_ids, out);
            state.last_output = Some(out.clone());
            change.map(|c| {
                state.last_update_at = Some(now);
                state.stats.updates += 1;
                sys.updates += 1;
                UpdateEvent {
                    id: uuid::Uuid::new_v4().to_string(),
                    interest_id: interest.id.clone(),
                    interest_name: interest.name.clone(),
                    at: now,
                    summary: c.summary,
                    items: c.items,
                    read: false,
                }
            })
        }
    }
}

/// When the interest should next run: its interval after the last run, with
/// exponential backoff (capped at 8x, max 24h) while it keeps failing, plus
/// the random delay rolled after the last run. That delay is capped at
/// `max_delay_mins`, so lowering the setting applies right away.
pub fn next_due(interest: &Interest, state: &InterestState, max_delay_mins: u32) -> Millis {
    let Some(last) = state.last_run_at else { return 0 };
    let base = interest.interval_mins.max(1) as i64 * 60_000;
    let factor = 1i64 << state.fail_streak.min(3);
    let delay = state.delay_ms.clamp(0, max_delay_mins as i64 * 60_000);
    last + (base * factor).min(24 * 3_600_000).max(base) + delay
}

/// A random delay of up to `max_mins` minutes, for [`apply_run`].
pub fn random_delay_ms(max_mins: u32) -> Millis {
    let max = max_mins as u128 * 60_000;
    // uuid's random bits stand in for a rand dependency, as in `app::shuffle_cards`.
    (uuid::Uuid::new_v4().as_u128() % (max + 1)) as Millis
}

#[cfg(test)]
mod tests {
    use super::*;

    fn out(ids: &[&str], text: &str) -> Output {
        Output {
            text: text.into(),
            items: ids.iter().map(|i| Item { id: i.to_string(), title: i.to_uppercase(), ..Default::default() }).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn first_run_is_baseline() {
        assert_eq!(detect(None, &[], &out(&["a"], "x")), None);
    }

    #[test]
    fn new_items_and_text_changes() {
        let prev = out(&["a", "b"], "hello");
        let seen = vec!["a".to_string(), "b".to_string()];
        assert_eq!(detect(Some(&prev), &seen, &out(&["b", "a"], "hello")), None);
        let c = detect(Some(&prev), &seen, &out(&["c", "a", "b"], "hello")).unwrap();
        assert_eq!(c.items.len(), 1);
        assert!(c.summary.starts_with("1 new: C"));
        let c = detect(Some(&prev), &seen, &out(&["a", "b"], "hello there")).unwrap();
        assert!(c.summary.starts_with("Text changed"));
    }

    #[test]
    fn items_that_reappear_are_not_new() {
        let seen = vec!["a".to_string(), "b".to_string()];
        assert_eq!(detect(Some(&out(&["b"], "")), &seen, &out(&["a", "b"], "")), None);
    }

    #[test]
    fn seen_cap_drops_ids_that_left_the_output() {
        let mut prev = out(&["keep", "old"], "");
        let mut seen = Vec::new();
        merge_seen(&mut seen, &prev);
        // "keep" stays in the output while more than MAX_SEEN other ids come and go.
        for n in 0..MAX_SEEN + 10 {
            let id = n.to_string();
            let new = out(&["keep", &id, &id], "");
            let change = detect(Some(&prev), &seen, &new).unwrap();
            assert_eq!(change.items.iter().map(|i| i.id.as_str()).collect::<Vec<_>>(), [id.as_str()]);
            merge_seen(&mut seen, &new);
            prev = new;
        }
        assert_eq!(seen.len(), MAX_SEEN);
        assert!(!seen.contains(&"old".to_string()));
        assert_eq!(seen.iter().filter(|id| *id == "keep").count(), 1);
    }

    #[test]
    fn key_mode() {
        let mut p = out(&[], "");
        p.key = Some("1".into());
        let mut n = out(&["z"], "different");
        n.key = Some("1".into());
        assert_eq!(detect(Some(&p), &[], &n), None);
        n.key = Some("2".into());
        assert!(detect(Some(&p), &[], &n).is_some());
    }

    #[test]
    fn apply_run_tracks_stats_and_events() {
        let interest = Interest { name: "I".into(), ..Default::default() };
        let mut st = InterestState::default();
        let mut sys = SystemStats::default();
        let report = |r: Result<Output, String>| RunReport { result: r, logs: vec![], fetches: vec![], duration_ms: 5, rounds: 1 };
        assert!(apply_run(&interest, &mut st, &mut sys, &report(Ok(out(&["a"], ""))), 1, 0).is_none());
        assert!(apply_run(&interest, &mut st, &mut sys, &report(Err("boom".into())), 2, 0).is_none());
        assert_eq!(st.fail_streak, 1);
        let ev = apply_run(&interest, &mut st, &mut sys, &report(Ok(out(&["a", "b"], ""))), 3, 0).unwrap();
        assert_eq!(ev.items[0].id, "b");
        assert_eq!((st.stats.refreshes, st.stats.failures, st.stats.updates), (3, 1, 1));
        assert_eq!((sys.refreshes, sys.failures, sys.updates), (3, 1, 1));
        assert_eq!(st.fail_streak, 0);
    }

    #[test]
    fn random_delay_is_added_and_capped() {
        let interest = Interest { interval_mins: 10, ..Default::default() };
        let mut st = InterestState::default();
        assert_eq!(next_due(&interest, &st, 5), 0, "never run: due right away");
        let mut sys = SystemStats::default();
        let report = RunReport { result: Ok(out(&[], "")), logs: vec![], fetches: vec![], duration_ms: 1, rounds: 1 };
        apply_run(&interest, &mut st, &mut sys, &report, 1_000, 180_000);
        assert_eq!(next_due(&interest, &st, 5), 1_000 + 600_000 + 180_000);
        // Lowering the setting caps a delay that was rolled under the old one.
        assert_eq!(next_due(&interest, &st, 1), 1_000 + 600_000 + 60_000);
        assert_eq!(next_due(&interest, &st, 0), 1_000 + 600_000);
        // Added on top of failure backoff too.
        st.fail_streak = 1;
        assert_eq!(next_due(&interest, &st, 5), 1_000 + 1_200_000 + 180_000);

        assert_eq!(random_delay_ms(0), 0);
        let rolls: Vec<Millis> = (0..200).map(|_| random_delay_ms(2)).collect();
        assert!(rolls.iter().all(|d| (0..=120_000).contains(d)));
        assert!(rolls.iter().any(|d| *d != rolls[0]), "rolls vary");
    }
}
