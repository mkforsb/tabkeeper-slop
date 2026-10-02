//! HTML parsing and CSS selection exposed to scripts.

use std::rc::Rc;

use html5ever::driver::ParseOpts;
use html5ever::tendril::TendrilSink;
use html5ever::tree_builder::TreeBuilderOpts;
use rhai::{Array, Dynamic, Engine, EvalAltResult};
use scraper::{ElementRef, Html, HtmlTreeSink, Selector};

/// A handle to an element inside a parsed document.
#[derive(Clone)]
pub struct HtmlNode {
    doc: Rc<Html>,
    id: ego_tree::NodeId,
}


pub fn parse(source: &str) -> HtmlNode {
    // Parse with scripting disabled so <noscript> content becomes real
    // elements: that is what a JS-less fetcher is served, and some sites
    // (e.g. SoundCloud) put their only server-rendered content there.
    let opts = ParseOpts {
        tree_builder: TreeBuilderOpts { scripting_enabled: false, ..Default::default() },
        ..Default::default()
    };
    let doc = html5ever::driver::parse_document(HtmlTreeSink::new(Html::new_document()), opts).one(source);
    let id = doc.root_element().id();
    HtmlNode { doc: Rc::new(doc), id }
}

impl HtmlNode {
    fn element(&self) -> ElementRef<'_> {
        ElementRef::wrap(self.doc.tree.get(self.id).expect("node id from same tree")).expect("node is an element")
    }

    fn wrap(&self, el: ElementRef<'_>) -> HtmlNode {
        HtmlNode { doc: self.doc.clone(), id: el.id() }
    }

    pub fn select(&self, css: &str) -> Result<Vec<HtmlNode>, Box<EvalAltResult>> {
        let sel = selector(css)?;
        let el = self.element();
        // Include the element itself, so `doc.select("html")` and similar work.
        let mut out = Vec::new();
        if sel.matches(&el) {
            out.push(self.clone());
        }
        out.extend(el.select(&sel).map(|e| self.wrap(e)));
        Ok(out)
    }

    pub fn text(&self) -> String {
        squish(&self.element().text().collect::<String>())
    }

    pub fn attr(&self, name: &str) -> Option<String> {
        self.element().attr(name).map(str::to_string)
    }
}

fn selector(css: &str) -> Result<Selector, Box<EvalAltResult>> {
    Selector::parse(css).map_err(|e| format!("invalid CSS selector '{css}': {e}").into())
}

/// Collapses runs of whitespace into single spaces and trims.
pub fn squish(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn strip_tags(s: &str) -> String {
    let frag = Html::parse_fragment(s);
    squish(&frag.root_element().text().collect::<String>())
}

fn to_array(nodes: Vec<HtmlNode>) -> Array {
    nodes.into_iter().map(Dynamic::from).collect()
}

pub fn register(engine: &mut Engine) {
    engine.register_type_with_name::<HtmlNode>("HtmlNode");
    engine.register_fn("html", |s: &str| parse(s));
    engine.register_fn("select", |n: &mut HtmlNode, css: &str| n.select(css).map(to_array));
    engine.register_fn("select_one", |n: &mut HtmlNode, css: &str| -> Result<Dynamic, Box<EvalAltResult>> {
        Ok(n.select(css)?.into_iter().next().map(Dynamic::from).unwrap_or(Dynamic::UNIT))
    });
    engine.register_fn("text", |n: &mut HtmlNode| n.text());
    engine.register_fn("attr", |n: &mut HtmlNode, name: &str| -> Dynamic {
        n.attr(name).map(Dynamic::from).unwrap_or(Dynamic::UNIT)
    });
    engine.register_fn("html", |n: &mut HtmlNode| n.element().html());
    engine.register_fn("inner_html", |n: &mut HtmlNode| n.element().inner_html());
    engine.register_fn("tag", |n: &mut HtmlNode| n.element().value().name().to_string());
    engine.register_fn("children", |n: &mut HtmlNode| -> Array {
        let el = n.element();
        el.child_elements().map(|c| Dynamic::from(n.wrap(c))).collect()
    });
    engine.register_fn("parent", |n: &mut HtmlNode| -> Dynamic {
        let el = n.element();
        el.parent().and_then(ElementRef::wrap).map(|p| Dynamic::from(n.wrap(p))).unwrap_or(Dynamic::UNIT)
    });
    engine.register_fn("to_string", |n: &mut HtmlNode| format!("<{}>", n.element().value().name()));

    // Make chains like `doc.select_one(".missing").text()` yield empty values
    // instead of "function not found" errors.
    engine.register_fn("text", |_: ()| String::new());
    engine.register_fn("attr", |_: (), _: &str| Dynamic::UNIT);
    engine.register_fn("html", |_: ()| String::new());
    engine.register_fn("inner_html", |_: ()| String::new());
    engine.register_fn("select", |_: (), _: &str| Array::new());
    engine.register_fn("select_one", |_: (), _: &str| Dynamic::UNIT);
    engine.register_fn("children", |_: ()| Array::new());
    engine.register_fn("parent", |_: ()| Dynamic::UNIT);

    engine.register_fn("strip_tags", |s: &str| strip_tags(s));
    engine.register_fn("squish", |s: &str| squish(s));
}
