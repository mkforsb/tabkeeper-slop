//! Background refresh loop. Runs inside the app: as async tasks on desktop
//! (script evaluation goes to a blocking thread pool), and in the page on web,
//! where it only runs while the tab is open.

use dioxus::prelude::*;
use tabkeeper::engine;
use tabkeeper::model::now_ms;
use tabkeeper::platform::sleep_ms;
use tabkeeper::script::{self, RunInput};

use super::state::*;

const TICK_MS: u64 = 5_000;

pub fn use_worker() {
    use_future(|| async move {
        sleep_ms(1_500).await;
        loop {
            *NOW.write() = now_ms();
            tick();
            sleep_ms(TICK_MS).await;
        }
    });
}

fn tick() {
    let settings = SETTINGS.read().clone();
    if settings.paused {
        return;
    }
    let now = now_ms();
    let running = RUNNING.read().clone();
    let slots = (settings.concurrency.max(1) as usize).saturating_sub(running.len());
    if slots == 0 {
        return;
    }
    let mut due: Vec<(i64, String)> = {
        let states = STATES.read();
        INTERESTS
            .read()
            .iter()
            .filter(|i| i.enabled && !running.contains(&i.id))
            .map(|i| (engine::next_due(i, &states.get(&i.id).cloned().unwrap_or_default()), i.id.clone()))
            .filter(|(at, _)| *at <= now)
            .collect()
    };
    due.sort();
    for (_, id) in due.into_iter().take(slots) {
        start_run(id);
    }
}

/// Starts a refresh in a task that outlives the calling component.
pub fn start_run(id: String) {
    dioxus::core::spawn_forever(run_interest(id));
}

async fn run_interest(id: String) {
    let Some(interest) = interest(&id) else { return };
    if !RUNNING.write().insert(id.clone()) {
        return;
    }
    let prev = STATES.read().get(&id).and_then(|s| s.last_output.clone());
    let cors_proxy = SETTINGS.read().cors_proxy.clone();
    let report = script::run(RunInput { script: interest.script.clone(), name: interest.name.clone(), prev, cors_proxy }).await;

    RUNNING.write().remove(&id);
    // The interest may have been deleted or re-scripted while running.
    let Some(current) = super::state::interest(&id) else { return };
    if current.script != interest.script {
        return;
    }

    let event = {
        let mut states = STATES.write();
        let st = states.entry(id.clone()).or_default();
        engine::apply_run(&current, st, &mut SYSTEM.write(), &report, now_ms())
    };
    let Some(event) = event else { return };

    let settings = SETTINGS.read().clone();
    {
        let mut events = EVENTS.write();
        events.insert(0, event.clone());
        events.truncate(settings.max_events.max(10) as usize);
    }
    if settings.notifications && current.notify {
        let result = crate::notify::send(current.name.clone(), event.summary.clone()).await;
        *NOTIFY_ERROR.write() = result.err();
    }
}
