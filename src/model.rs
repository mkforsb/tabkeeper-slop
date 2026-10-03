//! Persistent data model shared by the engine and the UI.

use serde::{Deserialize, Serialize};

/// Milliseconds since the Unix epoch.
pub type Millis = i64;

pub fn now_ms() -> Millis {
    chrono::Utc::now().timestamp_millis()
}

/// A thing on the internet we keep tabs on.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Interest {
    pub id: String,
    pub name: String,
    pub image_url: String,
    pub script: String,
    /// Minutes between refreshes.
    pub interval_mins: u32,
    pub enabled: bool,
    /// Show a desktop notification when an update is caught.
    pub notify: bool,
    /// Show the output's items last to first. Display only; detection is unaffected.
    pub reverse_order: bool,
    pub created_at: Millis,
}

impl Default for Interest {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: String::new(),
            image_url: String::new(),
            script: String::new(),
            interval_mins: 60,
            enabled: true,
            notify: true,
            reverse_order: false,
            created_at: now_ms(),
        }
    }
}

/// One entry in a script's output list (a video, a track, a screening...).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Item {
    /// Stable identity used to detect new items. Defaults to `url`, then `title`.
    pub id: String,
    pub title: String,
    pub url: String,
    pub image: String,
    pub text: String,
    pub date: String,
}

/// What a script produces. Rendered on the dashboard and diffed between runs.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Output {
    pub title: String,
    pub text: String,
    pub image: String,
    pub url: String,
    pub items: Vec<Item>,
    /// If set, an update is reported exactly when `key` changes; items and text are not diffed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct InterestStats {
    pub refreshes: u64,
    pub failures: u64,
    pub updates: u64,
    pub fetches: u64,
    pub bytes: u64,
    pub total_duration_ms: u64,
}

/// Runtime state for an interest, kept apart from the user-edited [`Interest`].
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct InterestState {
    pub last_output: Option<Output>,
    /// Item ids seen so far, oldest first (capped).
    pub seen_ids: Vec<String>,
    pub last_run_at: Option<Millis>,
    pub last_success_at: Option<Millis>,
    pub last_update_at: Option<Millis>,
    pub last_error: Option<String>,
    pub last_duration_ms: u64,
    pub last_logs: Vec<String>,
    /// Consecutive failures, used for backoff.
    pub fail_streak: u32,
    pub stats: InterestStats,
}

/// A caught change, shown in the updates feed and sent as a notification.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UpdateEvent {
    pub id: String,
    pub interest_id: String,
    pub interest_name: String,
    pub at: Millis,
    pub summary: String,
    pub items: Vec<Item>,
    pub read: bool,
}

/// An item the user starred, kept as it was when starred: it stays listed
/// after it drops out of its interest's output, or the interest is deleted.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StarredItem {
    /// Identifies this entry in the Starred list.
    pub id: String,
    pub interest_id: String,
    /// The interest's name when starred, shown if the interest is deleted.
    pub interest_name: String,
    pub item: Item,
    pub starred_at: Millis,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Web only: CORS proxy template, e.g. `https://corsproxy.io/?url={url}`.
    pub cors_proxy: String,
    pub concurrency: u32,
    pub notifications: bool,
    pub paused: bool,
    pub max_events: u32,
    /// Interest ids in Dashboard order. Interests not listed follow in their
    /// Interests page order; see `app::dashboard_cards`.
    pub dashboard_order: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            cors_proxy: String::new(),
            concurrency: 3,
            notifications: true,
            paused: false,
            max_events: 300,
            dashboard_order: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SystemStats {
    pub first_started_at: Millis,
    pub launches: u64,
    pub refreshes: u64,
    pub failures: u64,
    pub updates: u64,
    pub fetches: u64,
    pub bytes: u64,
}
