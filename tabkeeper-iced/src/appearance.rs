//! Appearance settings for the Iced app: UI scale and fonts. Stored in their
//! own file, apart from the settings shared with the Dioxus app.
//!
//! The scale applies immediately. Fonts are fixed at launch: iced sets the
//! default font once, and most text doesn't name a font of its own.

use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::sync::{Mutex, OnceLock};

use iced::advanced::graphics::text::cosmic_text::fontdb;
use iced::advanced::graphics::text::font_system;
use iced::font::{Family, Weight};
use iced::Font;
use serde::{Deserialize, Serialize};
use tabkeeper::storage;

const KEY: &str = "appearance";

/// UI scale choices, in percent.
pub const SCALES: &[u32] = &[80, 90, 100, 110, 125, 150, 175, 200];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Appearance {
    /// UI scale in percent.
    pub scale: u32,
    /// Interface font family; `None` for iced's default sans-serif.
    pub ui_font: Option<String>,
    /// Script editor font family; `None` for iced's default monospace.
    pub mono_font: Option<String>,
}

impl Default for Appearance {
    fn default() -> Self {
        Self { scale: 100, ui_font: None, mono_font: None }
    }
}

impl Appearance {
    pub fn save(&self) {
        storage::save(KEY, self);
    }

    pub fn scale_factor(&self) -> f32 {
        self.scale.clamp(50, 300) as f32 / 100.0
    }

    /// Whether the fonts differ from the ones the app launched with.
    pub fn needs_restart(&self) -> bool {
        let launched = launched();
        self.ui_font != launched.ui_font || self.mono_font != launched.mono_font
    }
}

/// The fonts in use, resolved at launch.
pub struct Fonts {
    pub regular: Font,
    pub bold: Font,
    pub mono: Font,
}

static LAUNCHED: OnceLock<Appearance> = OnceLock::new();
static FONTS: OnceLock<Fonts> = OnceLock::new();

/// Loads the stored appearance and fixes the fonts for this session.
/// Families that are no longer installed fall back to the defaults.
pub fn init() -> &'static Appearance {
    let mut a: Appearance = storage::load(KEY);
    let installed = families();
    a.ui_font = a.ui_font.filter(|f| installed.ui.contains(f));
    a.mono_font = a.mono_font.filter(|f| installed.mono.contains(f));
    let _ = FONTS.set(Fonts {
        regular: ui_font(a.ui_font.as_deref()),
        bold: Font { weight: Weight::Bold, ..ui_font(a.ui_font.as_deref()) },
        mono: mono_font(a.mono_font.as_deref()),
    });
    LAUNCHED.get_or_init(|| a)
}

fn launched() -> &'static Appearance {
    LAUNCHED.get_or_init(Appearance::default)
}

pub fn fonts() -> &'static Fonts {
    FONTS.get_or_init(|| Fonts {
        regular: Font::DEFAULT,
        bold: Font { weight: Weight::Bold, ..Font::DEFAULT },
        mono: Font::MONOSPACE,
    })
}

pub fn ui_font(family: Option<&str>) -> Font {
    Font { family: family.map_or(Family::SansSerif, |f| Family::Name(intern(f))), ..Font::DEFAULT }
}

pub fn mono_font(family: Option<&str>) -> Font {
    Font { family: family.map_or(Family::Monospace, |f| Family::Name(intern(f))), ..Font::MONOSPACE }
}

/// iced names families with `&'static str`; each distinct name is leaked once.
fn intern(name: &str) -> &'static str {
    static NAMES: Mutex<Option<HashSet<&'static str>>> = Mutex::new(None);
    let mut names = NAMES.lock().unwrap_or_else(|e| e.into_inner());
    let names = names.get_or_insert_with(HashSet::new);
    if let Some(n) = names.get(name) {
        return n;
    }
    let leaked: &'static str = Box::leak(name.to_string().into_boxed_str());
    names.insert(leaked);
    leaked
}

/// Installed families that can be offered, sorted by name.
pub struct Families {
    pub ui: Vec<String>,
    pub mono: Vec<String>,
}

/// Only families with an upright Regular and Bold face are offered: for a
/// missing weight, the text engine falls back to any installed font that has
/// it (serif or small-caps faces, say), not to a nearby weight.
pub fn families() -> &'static Families {
    static FAMILIES: OnceLock<Families> = OnceLock::new();
    FAMILIES.get_or_init(|| {
        // (regular, bold, monospaced)
        let mut found: BTreeMap<String, (bool, bool, bool)> = BTreeMap::new();
        let mut system = font_system().write().unwrap_or_else(|e| e.into_inner());
        for face in system.raw().db().faces() {
            let Some((name, _)) = face.families.first() else { continue };
            if face.style != fontdb::Style::Normal || name.starts_with('.') {
                continue;
            }
            let entry = found.entry(name.clone()).or_default();
            entry.0 |= face.weight == fontdb::Weight::NORMAL;
            entry.1 |= face.weight == fontdb::Weight::BOLD;
            entry.2 |= face.monospaced;
        }
        let usable = found.into_iter().filter(|(_, (regular, bold, _))| *regular && *bold);
        let (mono, ui): (Vec<_>, Vec<_>) = usable.partition(|(_, (_, _, mono))| *mono);
        Families { ui: ui.into_iter().map(|(n, _)| n).collect(), mono: mono.into_iter().map(|(n, _)| n).collect() }
    })
}

/// A font family in a picker; `None` is the default.
#[derive(Debug, Clone, PartialEq)]
pub struct FontChoice(pub Option<String>);

impl fmt::Display for FontChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_deref().unwrap_or("Default"))
    }
}

impl FontChoice {
    /// "Default" followed by `families`.
    pub fn list(families: &[String]) -> Vec<Self> {
        std::iter::once(Self(None)).chain(families.iter().map(|f| Self(Some(f.clone())))).collect()
    }
}

/// A UI scale in a picker, in percent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scale(pub u32);

impl fmt::Display for Scale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}%", self.0)
    }
}
