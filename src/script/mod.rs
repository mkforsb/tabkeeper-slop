//! Script engine for Interests.
//!
//! Scripts are [Rhai](https://rhai.rs) programs. `fetch()` looks synchronous to the
//! script, but network I/O is async (and must be, in the browser). We reconcile the
//! two by *replaying*: a fetch whose response isn't cached yet records the request
//! and aborts the run; the runner performs all recorded fetches concurrently and
//! re-runs the script with the responses cached. Scripts are deterministic given
//! their fetch responses, so the final run sees everything it asked for.

pub mod feed;
pub mod html;

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::rc::Rc;
use std::sync::Arc;

use rhai::{Array, Dynamic, Engine, EvalAltResult, Map, Scope};

use crate::fetch::{self, FetchLog, FetchRequest, FetchResponse};
use crate::model::{Item, Output};

const MAX_ROUNDS: u32 = 8;
const MAX_FETCHES: usize = 40;
const MAX_OPERATIONS: u64 = 50_000_000;
const MAX_ITEMS: usize = 200;
const MAX_FIELD: usize = 2000;
const MAX_LOGS: usize = 200;
const PENDING_FETCH: &str = "__tabkeeper_pending_fetch__";

type FetchCache = HashMap<FetchRequest, Result<FetchResponse, String>>;

pub struct RunInput {
    pub script: String,
    pub name: String,
    pub prev: Option<Output>,
    pub cors_proxy: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RunReport {
    pub result: Result<Output, String>,
    pub logs: Vec<String>,
    pub fetches: Vec<FetchLog>,
    pub duration_ms: u64,
    pub rounds: u32,
}

pub async fn run(input: RunInput) -> RunReport {
    let start = crate::platform::Instant::now();
    let mut cache = FetchCache::new();
    let mut fetches = Vec::new();
    let mut rounds = 0;
    // Arc, not Rc: the Iced app needs this future to be Send.
    let script: Arc<str> = input.script.into();
    let prev_json = input.prev.as_ref().map(|p| serde_json::to_value(p).unwrap_or_default());

    let (result, logs) = loop {
        rounds += 1;
        let (result, pending, logs) = {
            let (script, name, prev, cache) = (script.to_string(), input.name.clone(), prev_json.clone(), cache.clone());
            crate::platform::run_blocking(move || eval_once(&script, &name, prev, cache)).await
        };
        if pending.is_empty() {
            break (result, logs);
        }
        if rounds >= MAX_ROUNDS {
            break (Err(format!("gave up after {MAX_ROUNDS} fetch rounds")), logs);
        }
        if cache.len() + pending.len() > MAX_FETCHES {
            break (Err(format!("too many fetches (limit {MAX_FETCHES})")), logs);
        }
        let proxy = input.cors_proxy.as_str();
        let done = futures::future::join_all(pending.into_iter().map(|req| async move {
            let (res, log) = fetch::fetch(&req, proxy).await;
            (req, res, log)
        }))
        .await;
        for (req, res, log) in done {
            cache.insert(req, res);
            fetches.push(log);
        }
    };

    RunReport { result, logs, fetches, duration_ms: start.elapsed_ms(), rounds }
}

/// One evaluation pass. Returns the result, fetches the script wanted but
/// weren't cached, and captured log lines.
fn eval_once(
    script: &str,
    name: &str,
    prev: Option<serde_json::Value>,
    cache: FetchCache,
) -> (Result<Output, String>, BTreeSet<FetchRequest>, Vec<String>) {
    let pending = Rc::new(RefCell::new(BTreeSet::new()));
    let logs = Rc::new(RefCell::new(Vec::new()));
    let engine = build_engine(Rc::new(cache), pending.clone(), logs.clone());

    let mut scope = Scope::new();
    let prev = prev.and_then(|p| rhai::serde::to_dynamic(p).ok()).unwrap_or(Dynamic::UNIT);
    scope.push_constant("prev", prev);
    scope.push_constant("NAME", name.to_string());

    let result = engine.eval_with_scope::<Dynamic>(&mut scope, script);
    let pending = pending.take();
    let result = match result {
        Ok(v) => to_output(v),
        Err(e) => Err(describe_error(&e)),
    };
    (result, pending, logs.take())
}

fn describe_error(e: &EvalAltResult) -> String {
    match e {
        EvalAltResult::ErrorInFunctionCall(f, _, inner, pos) if !f.starts_with("anon$") => {
            format!("in {f}() {pos}: {}", describe_error(inner))
        }
        _ => e.to_string(),
    }
}

fn build_engine(cache: Rc<FetchCache>, pending: Rc<RefCell<BTreeSet<FetchRequest>>>, logs: Rc<RefCell<Vec<String>>>) -> Engine {
    let mut engine = Engine::new();
    engine.set_max_operations(MAX_OPERATIONS);
    engine.set_max_call_levels(64);
    engine.set_max_expr_depths(128, 64);

    let push_log = {
        let logs = logs.clone();
        move |s: String| {
            let mut l = logs.borrow_mut();
            if l.len() < MAX_LOGS {
                l.push(s);
            }
        }
    };
    {
        let push_log = push_log.clone();
        engine.on_print(move |s| push_log(s.to_string()));
    }
    {
        let push_log = push_log.clone();
        engine.on_debug(move |s, _, pos| push_log(format!("{pos}: {s}")));
    }
    engine.register_fn("log", move |v: Dynamic| push_log(display(&v)));

    // fetch(url), fetch(url, #{ method: "POST", body: ..., headers: #{...}, allow_error: true })
    let do_fetch = move |url: &str, opts: Map| -> Result<String, Box<EvalAltResult>> {
        let mut headers = BTreeMap::new();
        if let Some(h) = opts.get("headers").and_then(|h| h.read_lock::<Map>().map(|m| m.clone())) {
            for (k, v) in h {
                headers.insert(k.to_string(), display(&v));
            }
        }
        let allow_error = opts.get("allow_error").and_then(|v| v.as_bool().ok()).unwrap_or(false);
        let method = opts.get("method").map(display).filter(|m| !m.is_empty()).unwrap_or_else(|| "GET".into()).to_ascii_uppercase();
        if !method.bytes().all(|b| b.is_ascii_alphabetic()) {
            return Err(format!("invalid HTTP method '{method}'").into());
        }
        // A string body is sent as-is; a map or array is sent as JSON.
        let body = match opts.get("body").filter(|b| !b.is_unit()) {
            None => None,
            Some(b) if b.is_map() || b.is_array() => {
                if !headers.keys().any(|k| k.eq_ignore_ascii_case("content-type")) {
                    headers.insert("Content-Type".into(), "application/json".into());
                }
                Some(serde_json::to_string(b).map_err(|e| format!("body: {e}"))?)
            }
            Some(b) => Some(display(b)),
        };
        let req = FetchRequest { method, url: url.to_string(), headers, body };
        let what = req.method.clone() + " " + url;
        match cache.get(&req) {
            Some(Ok(resp)) if allow_error || (200..300).contains(&resp.status) => Ok(resp.body.to_string()),
            Some(Ok(resp)) => Err(format!("HTTP {} fetching {what}", resp.status).into()),
            Some(Err(e)) => Err(format!("fetching {what}: {e}").into()),
            None => {
                url::Url::parse(url).map_err(|e| format!("invalid URL '{url}': {e}"))?;
                pending.borrow_mut().insert(req);
                Err(PENDING_FETCH.into())
            }
        }
    };
    let do_fetch = Rc::new(do_fetch);
    {
        let f = do_fetch.clone();
        engine.register_fn("fetch", move |url: &str| f(url, Map::new()));
    }
    {
        let f = do_fetch.clone();
        engine.register_fn("fetch", move |url: &str, opts: Map| f(url, opts));
    }
    {
        let f = do_fetch.clone();
        engine.register_fn("fetch_json", move |url: &str| parse_json(&f(url, Map::new())?));
    }
    {
        let f = do_fetch.clone();
        engine.register_fn("fetch_json", move |url: &str, opts: Map| parse_json(&f(url, opts)?));
    }

    html::register(&mut engine);
    engine.register_fn("parse_feed", |xml: &str| feed::parse_feed(xml).map_err(Box::<EvalAltResult>::from));
    engine.register_fn("parse_json", |s: &str| parse_json(s));
    engine.register_fn("to_json", |v: Dynamic| serde_json::to_string(&v).unwrap_or_default());

    let regexes: Rc<RefCell<HashMap<String, regex::Regex>>> = Default::default();
    let get_re = move |pat: &str| -> Result<regex::Regex, Box<EvalAltResult>> {
        let mut cache = regexes.borrow_mut();
        if let Some(r) = cache.get(pat) {
            return Ok(r.clone());
        }
        let r = regex::Regex::new(pat).map_err(|e| format!("invalid regex: {e}"))?;
        cache.insert(pat.to_string(), r.clone());
        Ok(r)
    };
    let get_re = Rc::new(get_re);
    // Returns capture group 1 if the pattern has one, else the whole match.
    fn pick(c: &regex::Captures) -> String {
        c.get(1).or_else(|| c.get(0)).map(|m| m.as_str().to_string()).unwrap_or_default()
    }
    {
        let re = get_re.clone();
        engine.register_fn("regex_find", move |s: &str, pat: &str| -> Result<Dynamic, Box<EvalAltResult>> {
            Ok(re(pat)?.captures(s).map(|c| Dynamic::from(pick(&c))).unwrap_or(Dynamic::UNIT))
        });
    }
    {
        let re = get_re.clone();
        engine.register_fn("regex_find_all", move |s: &str, pat: &str| -> Result<Array, Box<EvalAltResult>> {
            Ok(re(pat)?.captures_iter(s).map(|c| Dynamic::from(pick(&c))).collect())
        });
    }
    {
        let re = get_re.clone();
        engine.register_fn("regex_test", move |s: &str, pat: &str| -> Result<bool, Box<EvalAltResult>> { Ok(re(pat)?.is_match(s)) });
    }
    {
        let re = get_re.clone();
        engine.register_fn("regex_replace", move |s: &str, pat: &str, rep: &str| -> Result<String, Box<EvalAltResult>> {
            Ok(re(pat)?.replace_all(s, rep).into_owned())
        });
    }

    engine.register_fn("between", |s: &str, start: &str, end: &str| -> Dynamic {
        s.find(start)
            .map(|i| &s[i + start.len()..])
            .and_then(|rest| rest.find(end).map(|j| rest[..j].to_string()))
            .map(Dynamic::from)
            .unwrap_or(Dynamic::UNIT)
    });
    engine.register_fn("url_join", |base: &str, rel: &str| -> String {
        url::Url::parse(base).and_then(|b| b.join(rel)).map(|u| u.to_string()).unwrap_or_else(|_| rel.to_string())
    });
    engine.register_fn("url_join", |base: &str, _: ()| -> String { let _ = base; String::new() });
    engine.register_fn("url_encode", |s: &str| -> String { url::form_urlencoded::byte_serialize(s.as_bytes()).collect() });
    engine.register_fn("url_decode", |s: &str| -> String {
        percent_encoding::percent_decode_str(s).decode_utf8_lossy().into_owned()
    });

    engine
}

fn parse_json(s: &str) -> Result<Dynamic, Box<EvalAltResult>> {
    let v: serde_json::Value = serde_json::from_str(s).map_err(|e| format!("invalid JSON: {e}"))?;
    rhai::serde::to_dynamic(v)
}

/// Renders a value for logs and output fields: strings as-is, unit as empty.
fn display(v: &Dynamic) -> String {
    if v.is_unit() {
        String::new()
    } else if let Ok(s) = v.clone().into_string() {
        s
    } else if v.is_map() || v.is_array() {
        serde_json::to_string(v).unwrap_or_else(|_| v.to_string())
    } else {
        v.to_string()
    }
}

fn field(m: &Map, key: &str) -> String {
    let s = m.get(key).map(display).unwrap_or_default();
    if s.chars().count() > MAX_FIELD {
        let mut t: String = s.chars().take(MAX_FIELD).collect();
        t.push('…');
        t
    } else {
        s
    }
}

fn to_item(v: Dynamic) -> Item {
    let mut item = if let Some(m) = v.read_lock::<Map>() {
        Item {
            id: field(&m, "id"),
            title: field(&m, "title"),
            url: field(&m, "url"),
            image: field(&m, "image"),
            text: field(&m, "text"),
            date: field(&m, "date"),
        }
    } else {
        Item { title: display(&v), ..Default::default() }
    };
    if item.id.is_empty() {
        item.id = [&item.url, &item.title, &item.text].into_iter().find(|s| !s.is_empty()).cloned().unwrap_or_default();
    }
    item
}

fn to_items(v: Option<&Dynamic>) -> Vec<Item> {
    v.and_then(|v| v.read_lock::<Array>().map(|a| a.clone()))
        .map(|a| a.into_iter().filter(|v| !v.is_unit()).take(MAX_ITEMS).map(to_item).collect())
        .unwrap_or_default()
}

/// Converts a script's final value into an [`Output`].
/// Accepts a map (`#{ title, text, image, url, items, key }`), an array of items, or a string.
pub fn to_output(v: Dynamic) -> Result<Output, String> {
    if v.is_unit() {
        return Err("script returned nothing; end it with a map like #{ text: \"...\", items: [...] }".into());
    }
    if let Some(m) = v.read_lock::<Map>() {
        return Ok(Output {
            title: field(&m, "title"),
            text: field(&m, "text"),
            image: field(&m, "image"),
            url: field(&m, "url"),
            items: to_items(m.get("items")),
            key: m.get("key").filter(|k| !k.is_unit()).map(display),
        });
    }
    if v.is_array() {
        return Ok(Output { items: to_items(Some(&v)), ..Default::default() });
    }
    Ok(Output { text: display(&v), ..Default::default() })
}

/// Reference for the editor's help panel.
pub const API_HELP: &[(&str, &str)] = &[
    ("fetch(url) / fetch(url, #{ headers: #{..}, allow_error: true })", "GET a URL, returns the body as a string. Throws on non-2xx unless allow_error."),
    ("fetch(url, #{ method: \"POST\", body: .. })", "Other methods. A string body is sent as-is; a map or array is sent as JSON (with Content-Type: application/json unless set)."),
    ("fetch_json(url[, opts])", "fetch() + parse_json()."),
    ("html(str)", "Parse HTML into a node. <noscript> content is parsed as markup."),
    ("node.select(css) / node.select_one(css)", "CSS selection. select_one returns () if nothing matches."),
    ("node.text() / node.attr(name) / node.html() / node.inner_html()", "Read a node. Calling these on () yields \"\" / ()."),
    ("node.children() / node.parent() / node.tag()", "Navigate the tree."),
    ("parse_feed(xml)", "RSS/Atom → #{ title, url, text, image, items: [#{ id, title, url, date, text, image }] }."),
    ("parse_json(str) / to_json(value)", "JSON conversion."),
    ("regex_find(s, re) / regex_find_all(s, re)", "First / all matches; capture group 1 if present."),
    ("regex_test(s, re) / regex_replace(s, re, rep)", "Regex test and replace-all."),
    ("between(s, start, end)", "Substring between two markers, or ()."),
    ("url_join(base, rel) / url_encode(s) / url_decode(s)", "Resolve relative links, percent-encode/decode."),
    ("strip_tags(html) / squish(s)", "HTML to text, collapse whitespace."),
    ("log(x) / print(x)", "Write to the run log."),
    ("prev, NAME", "The previous successful output (or ()) and this Interest's name."),
    ("Return value", "#{ title, text, image, url, items: [#{ id, title, url, image, text, date }], key }. New item ids and changed text count as updates; if `key` is set, only a change of key counts."),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(script: &str, cache: FetchCache) -> (Result<Output, String>, BTreeSet<FetchRequest>, Vec<String>) {
        eval_once(script, "test", None, cache)
    }

    fn cached(url: &str, status: u16, body: &str) -> FetchCache {
        let mut c = FetchCache::new();
        c.insert(FetchRequest::get(url), Ok(FetchResponse { status, body: body.into() }));
        c
    }

    #[test]
    fn string_and_map_outputs() {
        assert_eq!(eval(r#""hi""#, Default::default()).0.unwrap().text, "hi");
        let out = eval(r#"#{ title: "T", items: ["a", #{ url: "u", title: "b" }] }"#, Default::default()).0.unwrap();
        assert_eq!(out.title, "T");
        assert_eq!(out.items[0].id, "a");
        assert_eq!(out.items[1].id, "u");
        assert!(eval("()", Default::default()).0.is_err());
    }

    #[test]
    fn uncached_fetch_is_recorded_as_pending() {
        let (res, pending, _) = eval(r#"let a = fetch("https://a.test/"); a"#, Default::default());
        assert!(res.is_err());
        assert_eq!(pending.len(), 1);
        // A script catching the pending error still gets its fetch recorded.
        let (_, pending, _) = eval(r#"try { fetch("https://b.test/") } catch { "fallback" }"#, Default::default());
        assert_eq!(pending.len(), 1);
    }

    #[test]
    fn cached_fetch_and_html_selection() {
        let page = r#"<html><body><ul><li><a href="/x">One</a></li><li><a href="/y"> Two  </a></li></ul>
            <noscript><article class="t"><a href="/z">Hidden</a></article></noscript></body></html>"#;
        let script = r#"
            let doc = html(fetch("https://s.test/list"));
            let items = doc.select("li a").map(|a| #{ title: a.text(), url: url_join("https://s.test/", a.attr("href")) });
            items += doc.select("noscript article a").map(|a| #{ title: a.text() });
            log(doc.select_one(".missing").text());
            #{ items: items, text: doc.select_one(".missing").attr("x") ?? "none" }
        "#;
        let (res, pending, logs) = eval(script, cached("https://s.test/list", 200, page));
        assert!(pending.is_empty());
        let out = res.unwrap();
        assert_eq!(out.items.len(), 3);
        assert_eq!(out.items[1].title, "Two");
        assert_eq!(out.items[1].url, "https://s.test/y");
        assert_eq!(out.items[2].title, "Hidden");
        assert_eq!(out.text, "none");
        assert_eq!(logs, vec![String::new()]);
    }

    #[test]
    fn http_errors_throw_unless_allowed() {
        let c = cached("https://e.test/", 404, "nope");
        assert!(eval(r#"fetch("https://e.test/")"#, c.clone()).0.unwrap_err().contains("HTTP 404"));
        assert_eq!(eval(r#"fetch("https://e.test/", #{ allow_error: true })"#, c).0.unwrap().text, "nope");
    }

    #[test]
    fn post_requests() {
        let (_, pending, _) = eval(r#"fetch("https://p.test/", #{ method: "post", body: #{ q: "x" } })"#, Default::default());
        let req = pending.into_iter().next().unwrap();
        assert_eq!(req.method, "POST");
        assert_eq!(req.body.as_deref(), Some(r#"{"q":"x"}"#));
        assert_eq!(req.headers["Content-Type"], "application/json");

        // A string body is sent verbatim and an explicit content type is kept.
        let script = r#"fetch("https://p.test/", #{ method: "POST", body: "a=1", headers: #{ "content-type": "application/x-www-form-urlencoded" } })"#;
        let req = eval(script, Default::default()).1.into_iter().next().unwrap();
        assert_eq!(req.body.as_deref(), Some("a=1"));
        assert_eq!(req.headers.len(), 1);

        // A POST's response is cached separately from a GET to the same URL.
        let mut c = cached("https://p.test/", 200, "got");
        c.insert(req, Ok(FetchResponse { status: 200, body: "posted".into() }));
        assert_eq!(eval(script, c.clone()).0.unwrap().text, "posted");
        assert_eq!(eval(r#"fetch("https://p.test/")"#, c).0.unwrap().text, "got");
    }

    #[test]
    fn helpers() {
        let out = eval(
            r#"#{ title: regex_find("id=42;", "id=(\\d+)"), text: between("a[b]c", "[", "]"), url: parse_json("{\"x\":[1,2]}").x[1].to_string() }"#,
            Default::default(),
        )
        .0
        .unwrap();
        assert_eq!((out.title.as_str(), out.text.as_str(), out.url.as_str()), ("42", "b", "2"));
    }

    #[test]
    fn feeds() {
        let atom = r#"<?xml version="1.0"?><feed xmlns="http://www.w3.org/2005/Atom" xmlns:media="http://search.yahoo.com/mrss/">
            <title>Chan</title><link rel="alternate" href="https://c.test/"/>
            <entry><id>yt:video:1</id><title>V1</title><link rel="alternate" href="https://c.test/v1"/><published>2026-01-01</published>
            <media:group><media:thumbnail url="https://i.test/1.jpg"/><media:description>Desc</media:description></media:group></entry></feed>"#;
        let m = feed::parse_feed(atom).unwrap();
        let items = m["items"].clone().into_array().unwrap();
        let i0 = items[0].clone().cast::<Map>();
        assert_eq!(i0["url"].clone().into_string().unwrap(), "https://c.test/v1");
        assert_eq!(i0["image"].clone().into_string().unwrap(), "https://i.test/1.jpg");
        assert_eq!(i0["text"].clone().into_string().unwrap(), "Desc");

        let rss = r#"<rss version="2.0"><channel><title>R</title><link>https://r.test</link>
            <item><title>T1</title><link>https://r.test/1</link><guid>g1</guid><description>&lt;b&gt;hi&lt;/b&gt;</description></item></channel></rss>"#;
        let m = feed::parse_feed(rss).unwrap();
        let i0 = m["items"].clone().into_array().unwrap()[0].clone().cast::<Map>();
        assert_eq!(i0["id"].clone().into_string().unwrap(), "g1");
        assert_eq!(i0["text"].clone().into_string().unwrap(), "hi");
    }

    #[test]
    fn runaway_scripts_are_stopped() {
        assert!(eval("loop {}", Default::default()).0.unwrap_err().to_lowercase().contains("too many operations"));
    }
}
