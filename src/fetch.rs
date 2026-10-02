//! HTTP fetching for scripts. Native builds use reqwest directly; web builds go
//! through the browser's fetch (also via reqwest) and optionally a CORS proxy.

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

/// A request a script asked for. Also used as the replay cache key.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FetchRequest {
    /// Uppercase HTTP method, e.g. "GET" or "POST".
    pub method: String,
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub body: Option<String>,
}

impl FetchRequest {
    pub fn get(url: impl Into<String>) -> Self {
        FetchRequest { method: "GET".into(), url: url.into(), headers: Default::default(), body: None }
    }
}

#[derive(Clone, Debug)]
pub struct FetchResponse {
    pub status: u16,
    pub body: Arc<str>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FetchLog {
    pub method: String,
    pub url: String,
    pub status: Option<u16>,
    pub bytes: usize,
    pub ms: u64,
    pub error: Option<String>,
}

impl FetchLog {
    /// The URL, prefixed with the method unless it's a plain GET.
    pub fn label(&self) -> String {
        if self.method == "GET" { self.url.clone() } else { format!("{} {}", self.method, self.url) }
    }
}

#[cfg(not(target_arch = "wasm32"))]
const USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0 Safari/537.36";

const MAX_BODY: usize = 8 * 1024 * 1024;

fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        let b = reqwest::Client::builder();
        #[cfg(not(target_arch = "wasm32"))]
        let b = b
            .user_agent(USER_AGENT)
            .timeout(std::time::Duration::from_secs(30));
        b.build().expect("failed to build HTTP client")
    })
}

/// Applies a CORS proxy template. `{url}` is replaced by the percent-encoded
/// target; without a placeholder the target is appended as-is.
pub fn apply_proxy(proxy: &str, url: &str) -> String {
    let proxy = proxy.trim();
    if proxy.is_empty() {
        return url.to_string();
    }
    if proxy.contains("{url}") {
        let encoded: String = url::form_urlencoded::byte_serialize(url.as_bytes()).collect();
        proxy.replace("{url}", &encoded)
    } else {
        format!("{proxy}{url}")
    }
}

pub async fn fetch(req: &FetchRequest, cors_proxy: &str) -> (Result<FetchResponse, String>, FetchLog) {
    let start = crate::platform::Instant::now();
    let res = fetch_inner(req, cors_proxy).await;
    let log = FetchLog {
        method: req.method.clone(),
        url: req.url.clone(),
        status: res.as_ref().ok().map(|r| r.status),
        bytes: res.as_ref().map(|r| r.body.len()).unwrap_or(0),
        ms: start.elapsed_ms(),
        error: res.as_ref().err().cloned(),
    };
    (res, log)
}

async fn fetch_inner(req: &FetchRequest, cors_proxy: &str) -> Result<FetchResponse, String> {
    #[cfg(target_arch = "wasm32")]
    let url = apply_proxy(cors_proxy, &req.url);
    #[cfg(not(target_arch = "wasm32"))]
    let url = {
        let _ = cors_proxy;
        req.url.clone()
    };

    let method = reqwest::Method::from_bytes(req.method.as_bytes()).map_err(|_| format!("invalid HTTP method '{}'", req.method))?;
    let mut rb = client().request(method, &url);
    for (k, v) in &req.headers {
        rb = rb.header(k, v);
    }
    if let Some(body) = &req.body {
        rb = rb.body(body.clone());
    }
    let resp = rb.send().await.map_err(|e| describe_error(&e))?;
    let status = resp.status().as_u16();
    let bytes = resp.bytes().await.map_err(|e| describe_error(&e))?;
    let bytes = &bytes[..bytes.len().min(MAX_BODY)];
    let body: Arc<str> = String::from_utf8_lossy(bytes).into();
    Ok(FetchResponse { status, body })
}

/// Downloads binary content (images for the UI), with the same client and size cap as scripts.
pub async fn fetch_bytes(url: &str) -> Result<Vec<u8>, String> {
    let resp = client().get(url).send().await.map_err(|e| describe_error(&e))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status().as_u16()));
    }
    let bytes = resp.bytes().await.map_err(|e| describe_error(&e))?;
    if bytes.len() > MAX_BODY {
        return Err("response too large".into());
    }
    Ok(bytes.to_vec())
}

fn describe_error(e: &reqwest::Error) -> String {
    #[cfg(target_arch = "wasm32")]
    if e.is_request() {
        return format!("{e} (blocked by CORS? configure a CORS proxy in Settings)");
    }
    let mut msg = e.to_string();
    let mut src = std::error::Error::source(e);
    while let Some(s) = src {
        msg.push_str(": ");
        msg.push_str(&s.to_string());
        src = s.source();
    }
    msg
}
