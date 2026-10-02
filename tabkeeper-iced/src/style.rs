//! Colors and widget styles, mirroring `assets/main.css` of the Dioxus app.

use std::sync::LazyLock;

use iced::theme::{Mode, Palette};
use iced::widget::{self, button, container, rule, text};
use iced::{border, color, Background, Border, Color, Font, Shadow, Theme, Vector};

pub struct Colors {
    pub bg: Color,
    pub surface: Color,
    pub surface_2: Color,
    pub border: Color,
    pub ink: Color,
    pub ink_2: Color,
    pub muted: Color,
    pub accent: Color,
    pub accent_ink: Color,
    pub accent_soft: Color,
    pub good: Color,
    pub good_text: Color,
    pub warning_text: Color,
    pub critical: Color,
    pub critical_soft: Color,
}

const LIGHT: Colors = Colors {
    bg: color!(0xf4f4f2),
    surface: color!(0xfcfcfb),
    surface_2: color!(0xf0efec),
    border: color!(0xe2e1dc),
    ink: color!(0x0b0b0b),
    ink_2: color!(0x52514e),
    muted: color!(0x898781),
    accent: color!(0x2f5bd3),
    accent_ink: color!(0xffffff),
    accent_soft: color!(0xe6ecfb),
    good: color!(0x0ca30c),
    good_text: color!(0x006300),
    warning_text: color!(0x8a5a00),
    critical: color!(0xd03b3b),
    critical_soft: color!(0xfbeaea),
};

const DARK: Colors = Colors {
    bg: color!(0x121211),
    surface: color!(0x1a1a19),
    surface_2: color!(0x232321),
    border: color!(0x2e2e2c),
    ink: color!(0xffffff),
    ink_2: color!(0xc3c2b7),
    muted: color!(0x898781),
    accent: color!(0x7c9cf0),
    accent_ink: color!(0x0b0b0b),
    accent_soft: color!(0x1f2a45),
    good: color!(0x0ca30c),
    good_text: color!(0x0ca30c),
    warning_text: color!(0xfab219),
    critical: color!(0xd03b3b),
    critical_soft: color!(0x3a1d1d),
};

const WARNING: Color = color!(0xfab219);

fn palette(c: &Colors) -> Palette {
    Palette { background: c.bg, text: c.ink, primary: c.accent, success: c.good, warning: WARNING, danger: c.critical }
}

static LIGHT_THEME: LazyLock<Theme> = LazyLock::new(|| Theme::custom("Tabkeeper Light", palette(&LIGHT)));
static DARK_THEME: LazyLock<Theme> = LazyLock::new(|| Theme::custom("Tabkeeper Dark", palette(&DARK)));

/// Follows the system's light/dark preference, like the CSS does.
pub fn theme(mode: Mode) -> Theme {
    match mode {
        Mode::Dark => DARK_THEME.clone(),
        Mode::Light | Mode::None => LIGHT_THEME.clone(),
    }
}

pub fn colors(theme: &Theme) -> &'static Colors {
    if theme.extended_palette().is_dark {
        &DARK
    } else {
        &LIGHT
    }
}

// ---- type ----

pub const SMALL: f32 = 12.5;
pub const H1: f32 = 22.0;
pub const H2: f32 = 15.0;
// Fonts are chosen under Settings → Appearance and fixed at launch. Only
// Regular and Bold are used; see `appearance::families` for why.

pub fn regular() -> Font {
    crate::appearance::fonts().regular
}

pub fn bold() -> Font {
    crate::appearance::fonts().bold
}

pub fn mono() -> Font {
    crate::appearance::fonts().mono
}

pub fn muted(theme: &Theme) -> text::Style {
    text::Style { color: Some(colors(theme).muted) }
}

pub fn ink_2(theme: &Theme) -> text::Style {
    text::Style { color: Some(colors(theme).ink_2) }
}

pub fn good(theme: &Theme) -> text::Style {
    text::Style { color: Some(colors(theme).good_text) }
}

pub fn warning(theme: &Theme) -> text::Style {
    text::Style { color: Some(colors(theme).warning_text) }
}

pub fn critical(theme: &Theme) -> text::Style {
    text::Style { color: Some(colors(theme).critical) }
}

pub fn accent(theme: &Theme) -> text::Style {
    text::Style { color: Some(colors(theme).accent) }
}

// ---- containers ----

pub fn panel(theme: &Theme) -> container::Style {
    let c = colors(theme);
    container::Style {
        background: Some(c.surface.into()),
        border: Border { color: c.border, width: 1.0, radius: 10.0.into() },
        ..Default::default()
    }
}

pub fn sidebar(theme: &Theme) -> container::Style {
    container::Style { background: Some(colors(theme).surface.into()), ..Default::default() }
}

pub fn error_box(theme: &Theme) -> container::Style {
    let c = colors(theme);
    container::Style {
        background: Some(c.critical_soft.into()),
        text_color: Some(c.critical),
        border: border::rounded(7),
        ..Default::default()
    }
}

pub fn log(theme: &Theme) -> container::Style {
    container::Style { background: Some(colors(theme).surface_2.into()), border: border::rounded(7), ..Default::default() }
}

pub fn avatar_fallback(theme: &Theme) -> container::Style {
    let c = colors(theme);
    container::Style { background: Some(c.surface_2.into()), text_color: Some(c.ink_2), border: border::rounded(8), ..Default::default() }
}

pub fn toast(theme: &Theme) -> container::Style {
    let c = colors(theme);
    container::Style { background: Some(c.ink.into()), text_color: Some(c.surface), border: border::rounded(8), ..Default::default() }
}

pub fn nav_badge(theme: &Theme) -> container::Style {
    let c = colors(theme);
    container::Style { background: Some(c.accent.into()), text_color: Some(c.accent_ink), border: border::rounded(10), ..Default::default() }
}

pub fn meter_track(theme: &Theme) -> container::Style {
    container::Style { background: Some(colors(theme).surface_2.into()), border: border::rounded(3), ..Default::default() }
}

pub fn meter_fill(rate: f64) -> impl Fn(&Theme) -> container::Style {
    move |theme| {
        let c = colors(theme);
        let fill = if rate >= 50.0 {
            c.critical
        } else if rate > 0.0 {
            WARNING
        } else {
            c.good
        };
        container::Style { background: Some(fill.into()), border: border::rounded(3), ..Default::default() }
    }
}

/// The accent edge marking unread entries.
pub fn unread_edge(theme: &Theme) -> container::Style {
    container::Style { background: Some(colors(theme).accent.into()), ..Default::default() }
}

pub fn surface(theme: &Theme) -> container::Style {
    container::Style { background: Some(colors(theme).surface.into()), ..Default::default() }
}

/// A newly caught item.
pub fn fresh(theme: &Theme) -> container::Style {
    container::Style { background: Some(colors(theme).accent_soft.into()), border: border::rounded(7), ..Default::default() }
}

pub fn rule(theme: &Theme) -> rule::Style {
    rule::Style { color: colors(theme).border, radius: 0.0.into(), fill_mode: rule::FillMode::Full, snap: true }
}

// ---- inputs ----

fn input_border(c: &Colors, focused: bool) -> Border {
    Border { color: if focused { c.accent } else { c.border }, width: if focused { 2.0 } else { 1.0 }, radius: 7.0.into() }
}

pub fn text_input(theme: &Theme, status: widget::text_input::Status) -> widget::text_input::Style {
    let c = colors(theme);
    widget::text_input::Style {
        background: c.surface.into(),
        border: input_border(c, matches!(status, widget::text_input::Status::Focused { .. })),
        icon: c.muted,
        placeholder: c.muted,
        value: c.ink,
        selection: c.accent.scale_alpha(0.3),
    }
}

pub fn text_editor(theme: &Theme, status: widget::text_editor::Status) -> widget::text_editor::Style {
    let c = colors(theme);
    widget::text_editor::Style {
        background: c.surface.into(),
        border: input_border(c, matches!(status, widget::text_editor::Status::Focused { .. })),
        placeholder: c.muted,
        value: c.ink,
        selection: c.accent.scale_alpha(0.3),
    }
}

pub fn pick_list(theme: &Theme, status: widget::pick_list::Status) -> widget::pick_list::Style {
    let c = colors(theme);
    widget::pick_list::Style {
        text_color: c.ink,
        placeholder_color: c.ink_2,
        handle_color: c.ink_2,
        background: match status {
            widget::pick_list::Status::Hovered => c.surface_2.into(),
            _ => c.surface.into(),
        },
        border: input_border(c, matches!(status, widget::pick_list::Status::Opened { .. })),
    }
}

// ---- buttons ----

fn base_button(bg: Color, fg: Color, border_color: Color) -> button::Style {
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: fg,
        border: Border { color: border_color, width: 1.0, radius: 7.0.into() },
        ..Default::default()
    }
}

fn disabled(mut s: button::Style) -> button::Style {
    s.background = s.background.map(|b| b.scale_alpha(0.5));
    s.text_color = s.text_color.scale_alpha(0.5);
    s.border.color = s.border.color.scale_alpha(0.5);
    s
}

pub fn btn(theme: &Theme, status: button::Status) -> button::Style {
    let c = colors(theme);
    let s = base_button(c.surface, c.ink, c.border);
    match status {
        button::Status::Hovered | button::Status::Pressed => base_button(c.surface_2, c.ink, c.border),
        button::Status::Disabled => disabled(s),
        button::Status::Active => s,
    }
}

pub fn btn_primary(theme: &Theme, status: button::Status) -> button::Style {
    let c = colors(theme);
    let s = base_button(c.accent, c.accent_ink, c.accent);
    match status {
        button::Status::Hovered | button::Status::Pressed => {
            let hover = c.accent.scale_alpha(0.88);
            base_button(hover, c.accent_ink, hover)
        }
        button::Status::Disabled => disabled(s),
        button::Status::Active => s,
    }
}

pub fn btn_danger(theme: &Theme, status: button::Status) -> button::Style {
    let c = colors(theme);
    let s = base_button(c.critical, Color::WHITE, c.critical);
    match status {
        button::Status::Disabled => disabled(s),
        _ => s,
    }
}

/// A sidebar entry.
pub fn nav(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let c = colors(theme);
        let bg = match (active, status) {
            (true, _) => Some(c.accent_soft.into()),
            (false, button::Status::Hovered | button::Status::Pressed) => Some(c.surface_2.into()),
            _ => None,
        };
        button::Style {
            background: bg,
            text_color: if active { c.ink } else { c.ink_2 },
            border: border::rounded(7),
            ..Default::default()
        }
    }
}

/// Text that behaves like a hyperlink.
pub fn link(theme: &Theme, status: button::Status) -> button::Style {
    let c = colors(theme);
    let color = match status {
        button::Status::Hovered | button::Status::Pressed => c.accent.scale_alpha(0.8),
        _ => c.accent,
    };
    button::Style { background: None, text_color: color, ..Default::default() }
}

/// Like [`link`] but in the body text color, for headings and toggles.
pub fn plain_link(theme: &Theme, status: button::Status) -> button::Style {
    let c = colors(theme);
    let color = match status {
        button::Status::Hovered | button::Status::Pressed => c.accent,
        _ => c.ink,
    };
    button::Style { background: None, text_color: color, ..Default::default() }
}

/// The "3 new" pill on cards.
pub fn badge(theme: &Theme, status: button::Status) -> button::Style {
    let c = colors(theme);
    let bg = match status {
        button::Status::Hovered | button::Status::Pressed => c.accent.scale_alpha(0.85),
        _ => c.accent,
    };
    button::Style { background: Some(bg.into()), text_color: c.accent_ink, border: border::rounded(10), ..Default::default() }
}

pub fn toast_button(theme: &Theme, _status: button::Status) -> button::Style {
    let s = toast(theme);
    button::Style {
        background: s.background,
        text_color: s.text_color.unwrap_or(Color::WHITE),
        border: s.border,
        ..Default::default()
    }
}

/// The drop-down of a pick list.
pub fn menu(theme: &Theme) -> widget::overlay::menu::Style {
    let c = colors(theme);
    widget::overlay::menu::Style {
        background: c.surface.into(),
        border: Border { color: c.border, width: 1.0, radius: 7.0.into() },
        text_color: c.ink,
        selected_text_color: c.ink,
        selected_background: c.accent_soft.into(),
        shadow: Shadow { color: Color::BLACK.scale_alpha(0.15), offset: Vector::new(0.0, 2.0), blur_radius: 8.0 },
    }
}
