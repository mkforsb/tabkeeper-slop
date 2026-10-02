//! Minimal RSS 2.0 / Atom parser producing script-friendly maps.

use rhai::{Array, Dynamic, Map};
use roxmltree::{Document, Node, ParsingOptions};

use super::html::strip_tags;

const MAX_TEXT: usize = 500;

fn child<'a, 'i>(n: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    n.children().find(|c| c.is_element() && c.tag_name().name() == name)
}

fn child_text(n: Node, name: &str) -> String {
    child(n, name).and_then(|c| c.text()).map(|t| t.trim().to_string()).unwrap_or_default()
}

fn descendant_attr(n: Node, name: &str, attr: &str) -> Option<String> {
    n.descendants()
        .find(|c| c.is_element() && c.tag_name().name() == name && c.attribute(attr).is_some())
        .and_then(|c| c.attribute(attr))
        .map(str::to_string)
}

fn truncate(s: String) -> String {
    if s.chars().count() <= MAX_TEXT {
        return s;
    }
    let mut t: String = s.chars().take(MAX_TEXT).collect();
    t.push('…');
    t
}

fn atom_link(n: Node) -> String {
    n.children()
        .filter(|c| c.is_element() && c.tag_name().name() == "link")
        .find(|c| matches!(c.attribute("rel"), None | Some("alternate")))
        .and_then(|c| c.attribute("href"))
        .unwrap_or_default()
        .to_string()
}

fn image_of(n: Node) -> String {
    descendant_attr(n, "thumbnail", "url")
        .or_else(|| {
            n.descendants()
                .find(|c| {
                    c.is_element()
                        && matches!(c.tag_name().name(), "enclosure" | "content")
                        && (c.attribute("type").is_some_and(|t| t.starts_with("image/"))
                            || c.attribute("medium") == Some("image"))
                })
                .and_then(|c| c.attribute("url"))
                .map(str::to_string)
        })
        .or_else(|| descendant_attr(n, "image", "href"))
        .unwrap_or_default()
}

fn item_map(n: Node, atom: bool) -> Map {
    let mut m = Map::new();
    let (id, url, date, text) = if atom {
        let text = [child_text(n, "summary"), child_text(n, "content")]
            .into_iter()
            .chain(n.descendants().find(|c| c.tag_name().name() == "description").and_then(|c| c.text()).map(str::to_string))
            .find(|s| !s.trim().is_empty())
            .unwrap_or_default();
        let date = Some(child_text(n, "published")).filter(|s| !s.is_empty()).unwrap_or_else(|| child_text(n, "updated"));
        (child_text(n, "id"), atom_link(n), date, text)
    } else {
        (child_text(n, "guid"), child_text(n, "link"), child_text(n, "pubDate"), child_text(n, "description"))
    };
    let url = if url.is_empty() { child(n, "enclosure").and_then(|e| e.attribute("url")).unwrap_or_default().to_string() } else { url };
    let id = if id.is_empty() { url.clone() } else { id };
    m.insert("id".into(), id.into());
    m.insert("title".into(), child_text(n, "title").into());
    m.insert("url".into(), url.into());
    m.insert("date".into(), date.into());
    m.insert("text".into(), truncate(strip_tags(&text)).into());
    m.insert("image".into(), image_of(n).into());
    m
}

/// Parses an RSS or Atom document into
/// `#{ title, url, text, image, items: [#{ id, title, url, date, text, image }] }`.
pub fn parse_feed(xml: &str) -> Result<Map, String> {
    let opts = ParsingOptions { allow_dtd: true, ..Default::default() };
    let doc = Document::parse_with_options(xml.trim_start_matches('\u{feff}'), opts).map_err(|e| format!("invalid feed XML: {e}"))?;
    let root = doc.root_element();
    let (atom, channel) = match root.tag_name().name() {
        "feed" => (true, root),
        "rss" => (false, child(root, "channel").ok_or("RSS without <channel>")?),
        "RDF" => (false, root),
        other => return Err(format!("not a feed (root element <{other}>)")),
    };

    let mut m = Map::new();
    m.insert("title".into(), child_text(channel, "title").into());
    let url = if atom { atom_link(channel) } else { child_text(channel, "link") };
    m.insert("url".into(), url.into());
    let text = if atom { child_text(channel, "subtitle") } else { child_text(channel, "description") };
    m.insert("text".into(), truncate(strip_tags(&text)).into());
    let image = child(channel, "image")
        .map(|i| child_text(i, "url"))
        .filter(|s| !s.is_empty())
        .or_else(|| child(channel, "image").and_then(|i| i.attribute("href")).map(str::to_string))
        .or_else(|| Some(child_text(channel, "logo")).filter(|s| !s.is_empty()))
        .or_else(|| Some(child_text(channel, "icon")).filter(|s| !s.is_empty()))
        .unwrap_or_default();
    m.insert("image".into(), image.into());

    let item_tag = if atom { "entry" } else { "item" };
    // RSS 1.0 (RDF) puts items next to the channel, so search the whole document.
    let scope = if root.tag_name().name() == "RDF" { root } else { channel };
    let items: Array = scope
        .children()
        .filter(|c| c.is_element() && c.tag_name().name() == item_tag)
        .map(|n| Dynamic::from_map(item_map(n, atom)))
        .collect();
    m.insert("items".into(), items.into());
    Ok(m)
}
