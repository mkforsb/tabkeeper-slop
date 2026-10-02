use dioxus::prelude::*;
use tabkeeper::engine;
use tabkeeper::model::Interest;
use tabkeeper::script::{self, RunInput, RunReport, API_HELP};
use tabkeeper::templates::{self, TEMPLATES};

use super::components::*;
use super::state::*;
use super::worker::start_run;
use super::{fmt_ago, fmt_bytes, fmt_count, Route};

#[component]
pub fn Editor(id: String) -> Element {
    // Keyed so navigating between interests resets the form state.
    rsx! { EditorForm { key: "{id}", id: id.clone() } }
}

#[component]
fn EditorForm(id: String) -> Element {
    let is_new = id == "new";
    let initial = if is_new {
        Some(Interest { script: templates::find("page-section").map(|t| t.script.to_string()).unwrap_or_default(), ..Default::default() })
    } else {
        interest(&id)
    };
    let Some(initial) = initial else {
        return rsx! {
            div { class: "panel empty",
                h2 { "Interest not found" }
                Link { class: "btn", to: Route::Interests {}, "Back to interests" }
            }
        };
    };

    let mut draft = use_signal(|| initial.clone());
    let mut saved = use_signal(|| initial.clone());
    // Text fields are uncontrolled: writing `value` on every keystroke races the
    // webview (stale values overwrite newer input and the caret jumps to the end).
    // `form` holds what the fields were last filled with; bumping its revision
    // remounts them, which is how programmatic edits reach the DOM.
    let mut form = use_signal(|| (0u32, initial.clone()));
    let mut test = use_signal(|| None::<RunReport>);
    let mut testing = use_signal(|| false);
    let mut confirm_delete = use_signal(|| false);
    let mut flash = use_signal(|| None::<String>);
    let nav = navigator();

    let d = draft();
    let dirty = d != saved();
    let state = STATES.read().get(&d.id).cloned();
    let running = RUNNING.read().contains(&d.id);
    let valid = !d.name.trim().is_empty() && !d.script.trim().is_empty();

    let save = move |_| {
        let mut i = draft();
        i.name = i.name.trim().to_string();
        i.image_url = i.image_url.trim().to_string();
        i.interval_mins = i.interval_mins.max(1);
        upsert_interest(i.clone());
        draft.set(i.clone());
        saved.set(i.clone());
        let rev = form.peek().0 + 1;
        form.set((rev, i.clone()));
        flash.set(Some("Saved".into()));
        if is_new {
            nav.replace(Route::Editor { id: i.id });
        }
    };

    let run_test = move |_| {
        let i = draft();
        let prev = STATES.read().get(&i.id).and_then(|s| s.last_output.clone());
        let cors_proxy = SETTINGS.read().cors_proxy.clone();
        testing.set(true);
        spawn(async move {
            let r = script::run(RunInput { script: i.script, name: i.name, prev, cors_proxy }).await;
            test.set(Some(r));
            testing.set(false);
        });
    };

    rsx! {
        div { class: "page-head",
            h1 { if is_new { "New interest" } else { "{saved().name}" } }
            div { class: "actions",
                if let Some(msg) = flash() { span { class: "status status-good", "✓ {msg}" } }
                if !is_new {
                    StatusPill { interest: saved() }
                    button { class: "btn", disabled: running || dirty, title: "Save first to run the saved script", onclick: move |_| start_run(draft().id), "Run now" }
                }
                button { class: "btn", disabled: testing() || d.script.trim().is_empty(), onclick: run_test,
                    if testing() { span { class: "spinner" } "Testing…" } else { "Test run" }
                }
                button { class: "btn btn-primary", disabled: !valid || (!dirty && !is_new), onclick: save, "Save" }
            }
        }

        div { class: "editor",
            div { class: "editor-main",
                for (rev, f) in [form()] {
                    div { key: "{rev}", class: "panel form",
                        div { class: "form-row",
                            label { "Name"
                                input { initial_value: "{f.name}", placeholder: "e.g. New screenings at Bio Rio",
                                    oninput: move |e| { draft.write().name = e.value(); flash.set(None); } }
                            }
                            label { class: "narrow", "Every (minutes)"
                                input { r#type: "number", min: "1", initial_value: "{f.interval_mins}",
                                    oninput: move |e| { if let Ok(v) = e.value().parse() { draft.write().interval_mins = v; } flash.set(None); } }
                            }
                        }
                        div { class: "form-row",
                            label { "Image URL"
                                input { initial_value: "{f.image_url}", placeholder: "Optional; defaults to the image the script returns",
                                    oninput: move |e| { draft.write().image_url = e.value(); flash.set(None); } }
                            }
                            Avatar { src: d.image_url.clone(), name: d.name.clone(), class: "avatar avatar-large" }
                        }
                        div { class: "form-row checks",
                            label { class: "check",
                                input { r#type: "checkbox", checked: d.enabled, onchange: move |e| draft.write().enabled = e.checked() }
                                "Refresh automatically"
                            }
                            label { class: "check",
                                input { r#type: "checkbox", checked: d.notify, onchange: move |e| draft.write().notify = e.checked() }
                                "Notify on updates"
                            }
                            label { class: "template",
                                select {
                                    onchange: move |e| {
                                        if let Some(t) = templates::find(&e.value()) {
                                            let mut dr = draft.write();
                                            dr.script = t.script.to_string();
                                            dr.interval_mins = t.interval_mins;
                                            if dr.name.trim().is_empty() { dr.name = t.name.to_string(); }
                                            let filled = dr.clone();
                                            drop(dr);
                                            let rev = form.peek().0 + 1;
                                            form.set((rev, filled));
                                        }
                                    },
                                    option { value: "", selected: true, "Start from a template…" }
                                    for t in TEMPLATES { option { value: t.key, "{t.label}" } }
                                }
                            }
                        }
                        label { "Script"
                            textarea {
                                class: "code",
                                spellcheck: false,
                                rows: 24,
                                initial_value: "{f.script}",
                                oninput: move |e| { draft.write().script = e.value(); flash.set(None); },
                            }
                        }
                        if !is_new && d.script != saved().script {
                            p { class: "muted small", "Saving a changed script resets this interest's baseline: the next run records the current state without reporting updates." }
                        }
                        if !is_new {
                            div { class: "danger-zone",
                                if confirm_delete() {
                                    span { class: "small", "Delete this interest and its history?" }
                                    button { class: "btn btn-small btn-danger",
                                        onclick: move |_| { delete_interest(&draft().id); nav.push(Route::Interests {}); },
                                        "Delete" }
                                    button { class: "btn btn-small", onclick: move |_| confirm_delete.set(false), "Cancel" }
                                } else {
                                    button { class: "btn btn-small", onclick: move |_| confirm_delete.set(true), "Delete interest" }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "editor-side",
                if let Some(r) = test() {
                    TestResult { report: r, interest_id: d.id.clone() }
                }
                if let Some(st) = state {
                    div { class: "panel",
                        h2 { "Last run" }
                        dl { class: "kv",
                            dt { "Checked" } dd { "{fmt_ago(st.last_run_at, NOW())} ({st.last_duration_ms} ms)" }
                            dt { "Last success" } dd { "{fmt_ago(st.last_success_at, NOW())}" }
                            dt { "Last update" } dd { "{fmt_ago(st.last_update_at, NOW())}" }
                            dt { "Refreshes" } dd { "{fmt_count(st.stats.refreshes)} ({fmt_count(st.stats.failures)} failed)" }
                            dt { "Updates caught" } dd { "{fmt_count(st.stats.updates)}" }
                            dt { "Fetched" } dd { "{fmt_count(st.stats.fetches)} requests, {fmt_bytes(st.stats.bytes)}" }
                        }
                        if let Some(err) = &st.last_error { div { class: "error-box small", "✕ {err}" } }
                        if !st.last_logs.is_empty() {
                            details {
                                summary { "Log ({st.last_logs.len()} lines)" }
                                pre { class: "log", {st.last_logs.join("\n")} }
                            }
                        }
                        if let Some(out) = &st.last_output {
                            details {
                                summary { "Current output ({out.items.len()} items)" }
                                OutputView { output: out.clone(), limit: 20 }
                            }
                        }
                    }
                }
                div { class: "panel",
                    details { open: is_new,
                        summary { "Script reference" }
                        p { class: "small muted", "Scripts are written in Rhai (rhai.rs/book), a small Rust-like language. The value of the last expression is the output." }
                        dl { class: "api",
                            for (sig, desc) in API_HELP {
                                dt { code { "{sig}" } }
                                dd { "{desc}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn TestResult(report: RunReport, interest_id: String) -> Element {
    let st = STATES.read().get(&interest_id).cloned().unwrap_or_default();
    let verdict = report.result.as_ref().ok().map(|out| match engine::detect(st.last_output.as_ref(), &st.seen_ids, out) {
        _ if st.last_output.is_none() => "First run: this output would become the baseline.".to_string(),
        Some(c) => format!("Would report: {}", c.summary),
        None => "No change compared to the last saved run.".to_string(),
    });
    let meta = format!(
        "{} ms · {} · {}",
        report.duration_ms,
        counted(report.fetches.len(), "fetch", "fetches"),
        counted(report.rounds as usize, "pass", "passes")
    );
    rsx! {
        div { class: "panel test-result",
            div { class: "panel-head",
                h2 { "Test run" }
                span { class: "muted small", "{meta}" }
            }
            match &report.result {
                Ok(_) => rsx! { div { class: "status status-good", "✓ Success" } },
                Err(e) => rsx! { div { class: "error-box", "✕ {e}" } },
            }
            if let Some(v) = verdict { p { class: "small", "{v}" } }
            if !report.fetches.is_empty() {
                ul { class: "fetches small",
                    for f in report.fetches.iter() {
                        li {
                            match (&f.error, f.status) {
                                (Some(e), _) => rsx! { span { class: "status status-critical", "✕" } " {f.url} — {e}" },
                                (None, Some(s)) if (200..300).contains(&s) => rsx! { span { class: "status status-good", "✓ {s}" } " {f.url} · {fmt_bytes(f.bytes as u64)} · {f.ms} ms" },
                                (None, s) => rsx! { span { class: "status status-warning", "! {s.unwrap_or(0)}" } " {f.url} · {f.ms} ms" },
                            }
                        }
                    }
                }
            }
            if !report.logs.is_empty() {
                pre { class: "log", {report.logs.join("\n")} }
            }
            if let Ok(out) = &report.result {
                OutputView { output: out.clone(), limit: 30 }
            }
        }
    }
}

fn counted(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}
