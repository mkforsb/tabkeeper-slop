use dioxus::prelude::*;
use tabkeeper::app::{fmt_item_date, SHOW_MORE_STEP};
use tabkeeper::engine;
use tabkeeper::model::{Interest, Item, Output};

use super::state::*;
use super::{fmt_ago, fmt_in};

/// An interest being dragged to a new position, on the Interests page or the
/// Dashboard. Built on mouse events rather than HTML drag and drop, which the
/// webview's file-drop handling can block.
#[derive(Clone, PartialEq)]
pub struct Drag {
    pub id: String,
    /// The row or card under the cursor, where the interest will be moved to.
    pub over: Option<usize>,
}

/// Status shown with an icon and label, never color alone.
#[component]
pub fn StatusPill(interest: Interest) -> Element {
    let running = RUNNING.read().contains(&interest.id);
    let st = STATES.read().get(&interest.id).cloned().unwrap_or_default();
    let (class, label) = if running {
        ("status-running", "Refreshing".to_string())
    } else if !interest.enabled {
        ("status-muted", "○ Disabled".into())
    } else if st.last_error.is_some() {
        ("status-critical", format!("✕ Failing{}", if st.fail_streak > 1 { format!(" ×{}", st.fail_streak) } else { String::new() }))
    } else if st.last_success_at.is_some() {
        ("status-good", "✓ OK".into())
    } else {
        ("status-muted", "… Pending".into())
    };
    rsx! {
        span { class: "status {class}",
            if running { span { class: "spinner" } }
            "{label}"
        }
    }
}

/// "checked 5m ago · next in 25m"
#[component]
pub fn Timing(interest: Interest) -> Element {
    let now = NOW();
    let st = STATES.read().get(&interest.id).cloned().unwrap_or_default();
    let checked = fmt_ago(st.last_run_at, now);
    let next = if interest.enabled && !SETTINGS.read().paused {
        format!(" · next {}", fmt_in(engine::next_due(&interest, &st), now))
    } else {
        String::new()
    };
    rsx! { span { class: "muted small", "checked {checked}{next}" } }
}

pub fn display_image(interest: &Interest) -> String {
    if !interest.image_url.trim().is_empty() {
        return interest.image_url.clone();
    }
    STATES.read().get(&interest.id).and_then(|s| s.last_output.as_ref()).map(|o| o.image.clone()).unwrap_or_default()
}

#[component]
pub fn Avatar(src: String, name: String, #[props(default = "avatar".to_string())] class: String) -> Element {
    let initial = name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "?".into());
    rsx! {
        if src.trim().is_empty() {
            div { class: "{class} avatar-fallback", "{initial}" }
        } else {
            img { class: "{class}", src: "{src}", alt: "", referrerpolicy: "no-referrer", loading: "lazy" }
        }
    }
}

#[component]
pub fn ItemRow(item: Item, #[props(default = false)] fresh: bool) -> Element {
    let title = [&item.title, &item.text, &item.url, &item.id].into_iter().find(|s| !s.is_empty()).cloned().unwrap_or_default();
    let show_text = !item.text.is_empty() && item.text != title;
    rsx! {
        div { class: if fresh { "item fresh" } else { "item" },
            if !item.image.is_empty() {
                img { class: "item-thumb", src: "{item.image}", alt: "", referrerpolicy: "no-referrer", loading: "lazy" }
            }
            div { class: "item-body",
                if item.url.is_empty() {
                    div { class: "item-title", "{title}" }
                } else {
                    a { class: "item-title", href: "{item.url}", target: "_blank", rel: "noopener", "{title}" }
                }
                if !item.date.is_empty() { div { class: "muted small", "{fmt_item_date(&item.date)}" } }
                if show_text { div { class: "item-text", "{item.text}" } }
            }
        }
    }
}

/// Renders a script output. `limit` caps the number of items shown at first;
/// "+ N more" shows more. `reverse` lists them last to first.
#[component]
pub fn OutputView(
    output: Output,
    #[props(default = usize::MAX)] limit: usize,
    #[props(default = true)] show_image: bool,
    #[props(default = false)] reverse: bool,
) -> Element {
    let mut extra = use_signal(|| 0usize);
    let limit = limit.saturating_add(extra());
    let shown: Vec<Item> = if reverse {
        output.items.iter().rev().take(limit).cloned().collect()
    } else {
        output.items.iter().take(limit).cloned().collect()
    };
    let hidden = output.items.len().saturating_sub(shown.len());
    rsx! {
        div { class: "output",
            if show_image && !output.image.is_empty() {
                img { class: "output-image", src: "{output.image}", alt: "", referrerpolicy: "no-referrer" }
            }
            if !output.title.is_empty() {
                div { class: "output-title",
                    if output.url.is_empty() {
                        "{output.title}"
                    } else {
                        a { href: "{output.url}", target: "_blank", rel: "noopener", "{output.title}" }
                    }
                }
            }
            if !output.text.is_empty() { p { class: "output-text", "{output.text}" } }
            if let Some(key) = &output.key { div { class: "muted small", "key: {key}" } }
            if !shown.is_empty() {
                div { class: "items",
                    for item in shown { ItemRow { key: "{item.id}", item: item.clone() } }
                }
            }
            if hidden > 0 || extra() > 0 {
                div { class: "small more",
                    if hidden > 0 { button { class: "link-btn", onclick: move |_| extra += SHOW_MORE_STEP, "+ {hidden} more" } }
                    if extra() > 0 { button { class: "link-btn", onclick: move |_| extra.set(0), "Show fewer" } }
                }
            }
        }
    }
}
