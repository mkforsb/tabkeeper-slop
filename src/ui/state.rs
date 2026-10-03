//! Global app state. Persisted signals are loaded from storage on first read
//! and written back by effects in `App` whenever they change.

use std::collections::{HashMap, HashSet};

use dioxus::prelude::*;
use tabkeeper::app::Backup;
use tabkeeper::app;
use tabkeeper::model::*;
use tabkeeper::storage;

pub static INTERESTS: GlobalSignal<Vec<Interest>> = Signal::global(|| storage::load("interests"));
pub static STATES: GlobalSignal<HashMap<String, InterestState>> = Signal::global(|| storage::load("states"));
/// Newest first.
pub static EVENTS: GlobalSignal<Vec<UpdateEvent>> = Signal::global(|| storage::load("events"));
pub static SETTINGS: GlobalSignal<Settings> = Signal::global(|| storage::load("settings"));
pub static SYSTEM: GlobalSignal<SystemStats> = Signal::global(|| storage::load("system"));
/// In the Starred page's order.
pub static STARRED: GlobalSignal<Vec<StarredItem>> = Signal::global(|| storage::load("starred"));

/// Ids of interests whose script is running right now.
pub static RUNNING: GlobalSignal<HashSet<String>> = Signal::global(HashSet::new);
pub static SESSION_STARTED: GlobalSignal<Millis> = Signal::global(now_ms);
/// A coarse clock so relative times ("5m ago") refresh.
pub static NOW: GlobalSignal<Millis> = Signal::global(now_ms);
/// The update to scroll to when the Updates page opens next, set by clicking
/// it in the Dashboard's Recent updates.
pub static SHOW_EVENT: GlobalSignal<Option<String>> = Signal::global(|| None);
/// Last notification error, shown in Settings.
pub static NOTIFY_ERROR: GlobalSignal<Option<String>> = Signal::global(|| None);

/// Wires persistence: each persisted signal is saved whenever it changes.
pub fn use_persistence() {
    use_hook(|| {
        // Global signals initialise lazily; pin the session start to app launch.
        let _ = SESSION_STARTED();
        let mut sys = SYSTEM.write();
        if sys.first_started_at == 0 {
            sys.first_started_at = now_ms();
        }
        sys.launches += 1;
    });
    use_effect(|| storage::save("interests", &*INTERESTS.read()));
    use_effect(|| storage::save("states", &*STATES.read()));
    use_effect(|| storage::save("events", &*EVENTS.read()));
    use_effect(|| storage::save("settings", &*SETTINGS.read()));
    use_effect(|| storage::save("system", &*SYSTEM.read()));
    use_effect(|| storage::save("starred", &*STARRED.read()));
}

pub fn interest(id: &str) -> Option<Interest> {
    INTERESTS.read().iter().find(|i| i.id == id).cloned()
}

/// Inserts or replaces an interest; a changed script re-baselines its state.
pub fn upsert_interest(updated: Interest) {
    app::upsert_interest(&mut INTERESTS.write(), &mut STATES.write(), updated);
}

pub fn set_reversed(id: &str, on: bool) {
    if let Some(i) = INTERESTS.write().iter_mut().find(|i| i.id == id) {
        i.reverse_order = on;
    }
}

pub fn is_starred(interest_id: &str, item_id: &str) -> bool {
    app::is_starred(&STARRED.read(), interest_id, item_id)
}

pub fn toggle_star(interest_id: &str, item: &Item) {
    app::toggle_star(&mut STARRED.write(), &INTERESTS.read(), interest_id, item, now_ms());
}

pub fn reset_failures() {
    app::reset_failures(&mut STATES.write(), &mut SYSTEM.write());
}

pub fn delete_interest(id: &str) {
    INTERESTS.write().retain(|i| i.id != id);
    STATES.write().remove(id);
    EVENTS.write().retain(|e| e.interest_id != id);
}

pub fn unread_count(interest_id: Option<&str>) -> usize {
    EVENTS.read().iter().filter(|e| !e.read && interest_id.is_none_or(|id| e.interest_id == id)).count()
}

pub fn mark_read(interest_id: Option<&str>) {
    let mut events = EVENTS.write();
    for e in events.iter_mut().filter(|e| interest_id.is_none_or(|id| e.interest_id == id)) {
        e.read = true;
    }
}

pub fn export_json() -> String {
    let backup = Backup {
        interests: INTERESTS(),
        states: STATES(),
        events: EVENTS(),
        settings: SETTINGS(),
        system: SYSTEM(),
        starred: STARRED(),
    };
    serde_json::to_string_pretty(&backup).unwrap_or_default()
}

pub fn import_json(s: &str) -> Result<usize, String> {
    let b: Backup = serde_json::from_str(s).map_err(|e| e.to_string())?;
    let n = b.interests.len();
    *INTERESTS.write() = b.interests;
    *STATES.write() = b.states;
    *EVENTS.write() = b.events;
    *SETTINGS.write() = b.settings;
    *SYSTEM.write() = b.system;
    *STARRED.write() = b.starred;
    Ok(n)
}

/// Approximate stored size in bytes.
pub fn storage_size() -> usize {
    fn len<T: serde::Serialize>(v: &T) -> usize {
        serde_json::to_string(v).map(|s| s.len()).unwrap_or(0)
    }
    len(&*INTERESTS.read()) + len(&*STATES.read()) + len(&*EVENTS.read()) + len(&*SETTINGS.read()) + len(&*SYSTEM.read())
        + len(&*STARRED.read())
}
