//! View pieces shared by the pages (the counterpart of the Dioxus `components.rs`).

use iced::widget::{button, column, container, row, rule, space, text, text_input, Column};
use iced::{padding, Center, Fill, Shrink};
use tabkeeper::app::{fmt_ago, fmt_in, fmt_item_date, SHOW_MORE_STEP};
use tabkeeper::engine;
use tabkeeper::model::{Interest, Item, Output};

use crate::style::{self, bold, H1, SMALL};
use crate::{App, Message, Route};

pub type Element<'a> = iced::Element<'a, Message>;

pub fn page_head<'a>(title: impl text::IntoFragment<'a>, actions: Vec<Element<'a>>) -> Element<'a> {
    row![text(title).size(H1).font(bold()), space::horizontal(), row(actions).spacing(8).align_y(Center)]
        .align_y(Center)
        .spacing(12)
        .into()
}

pub fn panel<'a>(content: impl Into<Element<'a>>) -> container::Container<'a, Message> {
    container(content).padding(16).width(Fill).style(style::panel)
}

pub fn h2<'a>(s: impl text::IntoFragment<'a>) -> text::Text<'a> {
    text(s).size(style::H2).font(bold())
}

pub fn small<'a>(s: impl text::IntoFragment<'a>) -> text::Text<'a> {
    text(s).size(SMALL)
}

pub fn muted<'a>(s: impl text::IntoFragment<'a>) -> text::Text<'a> {
    text(s).size(SMALL).style(style::muted)
}

pub fn btn<'a>(label: impl text::IntoFragment<'a>, on_press: Option<Message>) -> button::Button<'a, Message> {
    button(text(label)).padding([6, 12]).style(style::btn).on_press_maybe(on_press)
}

pub fn btn_small<'a>(label: impl text::IntoFragment<'a>, on_press: Option<Message>) -> button::Button<'a, Message> {
    button(text(label).size(SMALL)).padding([3, 9]).style(style::btn).on_press_maybe(on_press)
}

pub fn btn_primary<'a>(label: impl text::IntoFragment<'a>, on_press: Option<Message>) -> button::Button<'a, Message> {
    btn(label, on_press).style(style::btn_primary)
}

/// An in-app link.
pub fn link<'a>(content: impl Into<Element<'a>>, to: Route) -> button::Button<'a, Message> {
    button(content).padding(0).style(style::link).on_press(Message::Navigate(to))
}

/// Marks `content` (on a panel) as unread with an accent left edge.
pub fn unread_marker<'a>(content: Element<'a>) -> Element<'a> {
    let inner = container(content).padding(padding::left(9)).width(Fill).style(style::surface);
    container(inner).padding(padding::left(3)).width(Fill).style(style::unread_edge).into()
}

pub fn hr<'a>() -> Element<'a> {
    rule::horizontal(1).style(style::rule).into()
}

pub fn input<'a>(placeholder: &str, value: &str, on_input: impl Fn(String) -> Message + 'a) -> text_input::TextInput<'a, Message> {
    text_input(placeholder, value).on_input(on_input).padding([7, 9]).style(style::text_input)
}

/// Opens `url` in the system browser; plain text if there's no URL.
pub fn ext_link<'a>(label: impl text::IntoFragment<'a>, url: &str, font: iced::Font) -> Element<'a> {
    let t = text(label).font(font);
    if url.is_empty() {
        t.into()
    } else {
        button(t).padding(0).style(style::link).on_press(Message::OpenUrl(url.to_string())).into()
    }
}

pub fn error_box<'a>(msg: impl std::fmt::Display) -> Element<'a> {
    container(text(format!("✕ {msg}")).size(SMALL)).padding([8, 10]).width(Fill).style(style::error_box).into()
}

pub fn tile<'a>(label: &'a str, value: String, sub: String) -> Element<'a> {
    tile_with(label, value, sub, None)
}

/// A [`tile`] with a small link after its sub text, e.g. "Reset".
pub fn tile_with<'a>(label: &'a str, value: String, sub: String, action: Option<Element<'a>>) -> Element<'a> {
    let sub = row![muted(sub)].push(action).spacing(8).align_y(Center);
    container(column![text(label).size(SMALL).style(style::ink_2), text(value).size(28).font(bold()), sub].spacing(2))
        .padding([14, 16])
        .width(Fill)
        .style(style::panel)
        .into()
}

/// A small text button in the link color.
pub fn link_btn<'a>(label: impl text::IntoFragment<'a>, on_press: Message) -> button::Button<'a, Message> {
    button(text(label).size(SMALL)).padding(0).style(style::link).on_press(on_press)
}

/// The "Reset" link on failure count tiles, if there's anything to reset.
pub fn reset_failures<'a>(app: &App) -> Option<Element<'a>> {
    (app.system.failures > 0).then(|| link_btn("Reset", Message::ResetFailures).into())
}

/// Two-column key/value list.
pub fn kv<'a>(rows: Vec<(&'a str, String)>) -> Element<'a> {
    Column::with_children(
        rows.into_iter().map(|(k, v)| row![text(k).style(style::muted).width(130), text(v).width(Fill)].spacing(16).into()),
    )
    .spacing(4)
    .into()
}

/// A section that can be expanded, like `<details>`.
pub fn details<'a>(summary: String, open: bool, toggle: Message, body: impl FnOnce() -> Element<'a>) -> Element<'a> {
    let head = button(text(format!("{} {summary}", if open { "▾" } else { "▸" })))
        .padding([4, 0])
        .style(style::plain_link)
        .on_press(toggle);
    if open {
        column![head, body()].spacing(6).into()
    } else {
        head.into()
    }
}

pub fn log_box<'a>(lines: &[String]) -> Element<'a> {
    container(text(lines.join("\n")).font(style::mono()).size(SMALL))
        .padding([8, 10])
        .width(Fill)
        .style(style::log)
        .into()
}

/// Status shown with an icon and label, never color alone.
pub fn status_pill<'a>(app: &App, interest: &Interest) -> Element<'a> {
    let running = app.running.contains(&interest.id);
    let st = app.state(&interest.id);
    let (style, label): (fn(&iced::Theme) -> text::Style, String) = if running {
        (style::accent, "↻ Refreshing".into())
    } else if !interest.enabled {
        (style::muted, "○ Disabled".into())
    } else if st.last_error.is_some() {
        let streak = if st.fail_streak > 1 { format!(" ×{}", st.fail_streak) } else { String::new() };
        (style::critical, format!("✕ Failing{streak}"))
    } else if st.last_success_at.is_some() {
        (style::good, "✓ OK".into())
    } else {
        (style::muted, "… Pending".into())
    };
    text(label).size(SMALL).style(style).into()
}

/// "checked 5m ago · next in 25m"
pub fn timing<'a>(app: &App, interest: &Interest) -> Element<'a> {
    let st = app.state(&interest.id);
    let next = if interest.enabled && !app.settings.paused {
        format!(" · next {}", fmt_in(engine::next_due(interest, st, app.settings.random_delay_mins), app.now))
    } else {
        String::new()
    };
    muted(format!("checked {}{next}", fmt_ago(st.last_run_at, app.now))).into()
}

pub fn display_image(app: &App, interest: &Interest) -> String {
    if !interest.image_url.trim().is_empty() {
        return interest.image_url.trim().to_string();
    }
    app.states.get(&interest.id).and_then(|s| s.last_output.as_ref()).map(|o| o.image.clone()).unwrap_or_default()
}

pub fn avatar<'a>(app: &App, src: &str, name: &str, size: f32) -> Element<'a> {
    let initial = name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "?".into());
    let radius = if size < 32.0 { 6.0 } else { 8.0 };
    let fallback = container(text(initial).font(bold())).center(size).style(style::avatar_fallback);
    if src.trim().is_empty() || app.images.failed(src.trim()) {
        return fallback.into();
    }
    app.images.view(src.trim(), size, size, radius, fallback)
}

fn item_title(item: &Item) -> &str {
    [&item.title, &item.text, &item.url, &item.id].into_iter().find(|s| !s.is_empty()).map(String::as_str).unwrap_or("")
}

/// Shortens long text to roughly `n` characters, like the CSS line clamps.
pub fn clamp(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(n).collect::<String>().trim_end())
    }
}

/// Stars or unstars `item` of interest `interest_id`.
pub fn star<'a>(app: &App, interest_id: &str, item: &Item) -> Element<'a> {
    let on = app.is_starred(interest_id, &item.id);
    button(text("★").size(15))
        .padding([0, 2])
        .style(style::star(on))
        .on_press(Message::ToggleStar(interest_id.to_string(), item.clone()))
        .into()
}

/// `star`: the interest the item belongs to, to show a [`star`].
pub fn item_row<'a>(app: &App, item: &Item, fresh: bool, star: Option<&str>) -> Element<'a> {
    let title = item_title(item);
    let mut body = column![ext_link(clamp(title, 140), &item.url, style::regular())].spacing(1).width(Fill);
    if !item.date.is_empty() {
        body = body.push(muted(fmt_item_date(&item.date)));
    }
    if !item.text.is_empty() && item.text != title {
        body = body.push(text(clamp(&item.text, 220)).size(SMALL).style(style::ink_2));
    }
    let mut r = row![].spacing(10);
    if let Some(interest_id) = star {
        r = r.push(self::star(app, interest_id, item));
    }
    if !item.image.is_empty() && !app.images.failed(&item.image) {
        let placeholder = container(space::horizontal()).style(style::log);
        r = r.push(app.images.view(&item.image, 64.0, 40.0, 5.0, placeholder));
    }
    let r = r.push(body);
    if fresh {
        container(r).padding(6).width(Fill).style(style::fresh).into()
    } else {
        container(r).padding([4, 0]).into()
    }
}

/// Renders a script output. `limit` caps the number of items shown at first;
/// "+ N more" shows more, remembered in [`App::more`] under `key`. `reverse`
/// lists them last to first. With `star`, the id of the interest that
/// produced it, the items can be starred.
pub fn output_view<'a>(
    app: &App,
    output: &'a Output,
    limit: usize,
    show_image: bool,
    reverse: bool,
    key: String,
    star: Option<&str>,
) -> Element<'a> {
    let extra = app.more.get(&key).copied().unwrap_or(0);
    let limit = limit.saturating_add(extra);
    let mut col = column![].spacing(6).width(Fill);
    if show_image && !output.image.is_empty() && !app.images.failed(&output.image) {
        let placeholder = container(space::horizontal()).style(style::log);
        col = col.push(app.images.view(&output.image, 280.0, 160.0, 8.0, placeholder));
    }
    if !output.title.is_empty() {
        col = col.push(ext_link(output.title.as_str(), &output.url, bold()));
    }
    if !output.text.is_empty() {
        col = col.push(text(clamp(&output.text, 600)).style(style::ink_2));
    }
    if let Some(key) = &output.key {
        col = col.push(muted(format!("key: {key}")));
    }
    let shown = output.items.len().min(limit);
    if shown > 0 {
        let items: Box<dyn Iterator<Item = &Item>> = if reverse { Box::new(output.items.iter().rev()) } else { Box::new(output.items.iter()) };
        col = col.push(Column::with_children(items.take(limit).map(|i| item_row(app, i, false, star))).spacing(4));
    }
    let hidden = output.items.len() - shown;
    if hidden > 0 || extra > 0 {
        let mut more = row![].spacing(12);
        if hidden > 0 {
            more = more.push(link_btn(format!("+ {hidden} more"), Message::ShowMore(key.clone(), extra + SHOW_MORE_STEP)));
        }
        if extra > 0 {
            more = more.push(link_btn("Show fewer", Message::ShowMore(key, 0)));
        }
        col = col.push(more);
    }
    col.into()
}

/// A horizontal row of equal tiles.
pub fn tiles<'a>(tiles: Vec<Element<'a>>) -> Element<'a> {
    row(tiles).spacing(12).height(Shrink).into()
}
