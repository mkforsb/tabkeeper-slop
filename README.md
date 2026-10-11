# Tabkeeper

Keep tabs on interesting things on the internet. Tabkeeper is a Rust app for
desktop and web, with a Dioxus UI and an alternative native desktop UI in Iced: each *Interest* is a small script that fetches a page or
feed and extracts what matters. Background workers re-run every Interest on a
schedule, detect changes, and show desktop notifications.

There is no server. State lives in `localStorage` (web) or JSON files in the
user data directory (desktop).

## Running

Requires a stable Rust toolchain and a C compiler (`cc`, used to build `ring`);
that's enough for `cargo test` and the `run_script` example. The Dioxus app
also needs the Dioxus CLI (`dx`, 0.7).

```sh
curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal   # Rust via rustup
```

```sh
dx serve --platform desktop      # desktop app
dx serve --platform web          # web app at http://localhost:8080
cargo test                       # engine, script API and change-detection tests
cargo run --example run_script -- --template biorio    # run a script headlessly
cargo run --example run_script -- my-script.rhai
```

`cargo run --features desktop` also works for the desktop app.

### Iced desktop app

`tabkeeper-iced/` is an alternative desktop front end written in
[Iced](https://iced.rs): native rendering instead of a webview, same features
and pages. Both apps use the same core crate and the same data files, so you
can switch between them (don't run both at once; each overwrites the files with
its own state).

```sh
cargo run -p tabkeeper-iced      # or: make desktop-iced
```

It follows the system light/dark setting, and opens links in your browser.
Settings → Appearance has a UI scale (applies immediately) and the interface
and editor fonts (applied on restart). These are stored separately, in
`tabkeeper.appearance.json`, and aren't part of Export/Import.

### Makefile

| Target | Command |
|---|---|
| `desktop-dioxus-release` | `dx build --desktop --release` → `target/dx/tabkeeper/release/linux/app/` |
| `desktop-iced-release` | `cargo build --release -p tabkeeper-iced` → `target/release/tabkeeper-iced` |
| `web-release` | `dx build --web --release` → `target/dx/tabkeeper/release/web/public/` |
| `desktop-dioxus`, `desktop-iced`, `web` | run in development mode |
| `test` | `cargo test --workspace` |

Desktop data goes to `$XDG_DATA_HOME/tabkeeper` (override with
`TABKEEPER_DATA_DIR`). Workers run while the app window is open.

### Linux setup

The desktop apps need some system libraries. On Debian (forky), this is what
was installed to build both apps and drive them on a virtual display; it may be
more than strictly necessary:

```sh
# Building the Dioxus desktop app (WebKitGTK webview)
apt-get install pkg-config libwebkit2gtk-4.1-dev libgtk-3-dev libxdo-dev libssl-dev
# Running the Iced app (it panics at startup without libxkbcommon-x11)
apt-get install libxkbcommon-dev libxkbcommon-x11-0 libwayland-dev libxcursor1 libxrandr2 libxi6 libx11-xcb1 mesa-vulkan-drivers libvulkan1
# A virtual display, screenshots, input, and fonts (emoji for the 🌐 button)
apt-get install xvfb x11-utils imagemagick xdotool dbus-x11 fonts-noto-color-emoji fonts-dejavu-core
# Type-checking the web build without dx
rustup target add wasm32-unknown-unknown
cargo check --target wasm32-unknown-unknown --features web
```

To run either app without a display, and take screenshots:

```sh
Xvfb :99 -screen 0 1280x800x24 &
export DISPLAY=:99 TABKEEPER_DATA_DIR=/tmp/tk-data   # keep test data apart
./target/debug/tabkeeper-iced &                       # or, for Dioxus:
WEBKIT_DISABLE_DMABUF_RENDERER=1 dbus-run-session -- ./target/debug/tabkeeper &
xdotool mousemove 380 300 click 1                     # click; `xdotool key` needs the window focused
import -window root shot.png                          # screenshot (ImageMagick)
```

Links open with `xdg-open` (Iced) or the `webbrowser` crate (Dioxus, which
honours `$BROWSER`). Without a browser installed, point both at a script that
logs its argument to check which URL a click opens.

### Web caveats

- **CORS.** Browsers only let a page read another site's responses if that site
  allows it, and most don't (Bio Rio does; YouTube, SoundCloud, Bio Aspen and
  Slakthuset don't). Set a CORS proxy under Settings. `{url}` in the template
  is replaced with the encoded target URL. Public proxies see everything you
  fetch, so prefer one you run yourself. The desktop app needs no proxy.
- Workers only run while the tab is open. Browsers throttle timers in
  background tabs, so refreshes may lag there.
- Browsers ignore custom `Cookie`/`User-Agent` headers, so scripts that need a
  login (Instagram) only work on desktop.
- `localStorage` holds roughly 5 MB. Use Settings → Export for backups.

## Scripts

Scripts are written in [Rhai](https://rhai.rs/book/), a small, Rust-like
scripting language. The value of the last expression is the script's output:

```rhai
let doc = html(fetch("https://slakthusetclub.se/events"));
#{
    title: "Slakthuset",
    items: doc.select("main li a[href^='/event/']").map(|a| #{
        url: url_join("https://slakthusetclub.se", a.attr("href")),
        title: a.select_one("h3").text(),
        date: a.select_one("p").text(),
    }),
}
```

Output fields: `title`, `text`, `image`, `url`, `extra_urls`, `items` (each
with `id`, `title`, `url`, `image`, `text`, `date`, `extra_urls`; `id` defaults
to `url`, then `title`, then `text`), and an optional `key`. The dashboard
renders all of them. `extra_urls` is a list of `[title, url]` pairs, e.g.
`[["IMDb", imdb_url], ["Rotten Tomatoes", rt_url]]`, shown in a menu behind a
🌐 button: under an item's star, or next to the output's title. Pairs without
a URL are dropped, and an empty title shows the URL. A bare array
is shorthand for `#{ items: [...] }`, and any other value for `#{ text: ... }`.
[`docs/script-output.schema.json`](docs/script-output.schema.json) is a JSON
Schema for the output, with `()` as `null`. It is a little stricter than the
app: it rejects unknown fields and maps or arrays in text fields, which the app
ignores or writes out as JSON.

**Change detection.** The first successful run sets a baseline. After that, an
update is reported when an item id appears that hasn't been seen before, or
when `text` changes. If the script returns `key`, only a change of `key` counts.
Saving an edited script resets the baseline, so changing an item's id format
doesn't flood you with "new" items.

**API.** `fetch(url[, #{ method, body, headers, allow_error }])` (a map or
array `body` is sent as JSON), `fetch_json`, `html(str)`
with `select` / `select_one` / `text` / `attr` / `html` / `children` / `parent`,
`parse_feed` (RSS/Atom), `parse_json` / `to_json`, `regex_find` /
`regex_find_all` / `regex_test` / `regex_replace`, `between`, `url_join`,
`url_encode` / `url_decode`, `strip_tags`, `squish`, `log`, and the constants
`prev` and `NAME`. The editor has a full reference and a *Test run* button that
shows the fetches, logs, output, and whether the result would count as an
update.

**How async fetching works.** In a script, `fetch()` looks synchronous, but the
network calls are async (in the browser they must be). The runner *replays* the
script. A `fetch()` whose response isn't cached yet records the request and
aborts the pass. The runner then performs every recorded request concurrently
and runs the script again with the responses cached, until a pass completes
without new requests. Scripts are deterministic given their responses, so this
is invisible to the author. A script that fetches a page and then a URL found
in it takes three passes.

Limits per run: 40 fetches, 8 passes, 50M Rhai operations (infinite loops are
stopped), 8 MB per response.

## Included templates

| Template | Approach | Status |
|---|---|---|
| YouTube channel | channel page → channel id → public Atom feed | works |
| SoundCloud new tracks | `/tracks` page's `<noscript>` track list | works |
| SoundCloud bio edited | `window.__sc_hydration` JSON → `description` | works |
| Instagram new posts | `web_profile_info` API with your `sessionid` cookie | untested; needs login, desktop only |
| GitHub review requests | GraphQL `search` for `review-requested:@me` with a personal access token | untested against the live API; needs token |
| GitHub PR comments | GraphQL `search` for e.g. `involves:@me`, with comments, reviews and review threads | untested against the live API; needs token |
| Bio Rio reRUN | `article.showtime-card` | works |
| Bio Aspen Classics | `div.movie > a` | works |
| Slakthuset events | `main li a[href^='/event/']` | works |
| Any feed / page section / links | generic | — |

"Works" means it was verified against the live sites on 2026-10-02. Scrapers
break when sites change their markup; a failing Interest shows its error on the
dashboard, and its failures are counted in Stats.

## Layout

```
src/
  lib.rs           core crate (no UI)
  model.rs         Interest, Output/Item, state, stats, settings
  script/          Rhai engine: replaying runner, HTML (scraper), RSS/Atom
  engine.rs        change detection, stats bookkeeping, backoff
  app.rs           logic shared by both UIs: editing rules, scheduling, formatting
  fetch.rs         reqwest on both targets, CORS proxy on web
  storage.rs       localStorage / JSON files
  templates.rs     starter scripts
  notify.rs        notify-rust / Notification API
  main.rs          Dioxus launch (desktop window / web)
  ui/              Dioxus pages, global signals, background worker
docs/
  script-output.schema.json  JSON Schema for a script's output
tabkeeper-iced/
  src/main.rs      Iced app: state, messages, worker, persistence, shell
  src/pages/       one module per page
  src/widgets.rs   shared view pieces; style.rs: the CSS palette as Iced styles
  src/images.rs    downloads remote images as they scroll into view
  src/popup.rs     click-to-open popup widget (the 🌐 menu)
  src/appearance.rs  UI scale and font settings
```

`dioxus` is an optional dependency enabled by the `desktop`/`web` features
(which `dx` picks per platform), so building the Iced app doesn't compile it.
