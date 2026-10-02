//! Starter scripts offered in the editor.

pub struct Template {
    pub key: &'static str,
    pub label: &'static str,
    pub name: &'static str,
    pub interval_mins: u32,
    pub script: &'static str,
}

pub const TEMPLATES: &[Template] = &[
    Template {
        key: "youtube",
        label: "YouTube channel: new videos",
        name: "YouTube: Linus Tech Tips",
        interval_mins: 30,
        script: r#"// New videos on a YouTube channel, via the channel's public Atom feed.
let channel = "https://www.youtube.com/@LinusTechTips";
// Optional: set the channel id (UC...) to skip looking it up on the channel page.
let channel_id = "";

let image = ();
if channel_id == "" {
    let page = fetch(channel);
    channel_id = regex_find(page, "\"externalId\":\"([\\w-]+)\"") ?? regex_find(page, "channel_id=([\\w-]+)");
    if channel_id == () { throw "could not find the channel id on " + channel; }
    image = regex_find(page, "<meta property=\"og:image\" content=\"([^\"]+)\"");
}

let feed = parse_feed(fetch("https://www.youtube.com/feeds/videos.xml?channel_id=" + channel_id));
#{
    title: feed.title,
    url: channel,
    image: image,
    items: feed.items,
}
"#,
    },
    Template {
        key: "soundcloud-tracks",
        label: "SoundCloud profile: new tracks",
        name: "SoundCloud: Flume tracks",
        interval_mins: 60,
        script: r#"// New tracks on a SoundCloud profile. SoundCloud serves a plain-HTML track
// list inside <noscript>, which html() parses as regular markup.
let profile = "https://soundcloud.com/flume";

let doc = html(fetch(profile + "/tracks"));
let tracks = doc.select("article[itemprop='track']").map(|t| {
    let a = t.select_one("[itemprop='name'] a[itemprop='url']");
    #{
        title: a.text(),
        url: url_join(profile, a.attr("href")),
        date: t.select_one("time").text(),
        text: t.select_one("meta[itemprop='genre']").attr("content"),
    }
});
if tracks.is_empty() { throw "no tracks found; has the page layout changed?"; }

#{
    title: doc.select_one("[itemtype='http://schema.org/MusicGroup'] h1").text(),
    url: profile,
    image: doc.select_one("img[itemprop='image']").attr("src"),
    items: tracks,
}
"#,
    },
    Template {
        key: "soundcloud-bio",
        label: "SoundCloud profile: bio text edited",
        name: "SoundCloud: Flume bio",
        interval_mins: 360,
        script: r#"// Reports when a SoundCloud profile's description changes. The profile data
// is embedded in the page as JSON (window.__sc_hydration).
let profile = "https://soundcloud.com/flume";

let page = fetch(profile);
let data = parse_json(between(page, "window.__sc_hydration = ", ";</script>") ?? "[]");
let user = ();
for entry in data {
    if entry.hydratable == "user" { user = entry.data; }
}
if user == () { throw "user data not found in page"; }

#{
    title: user.username,
    text: user.description ?? "",
    image: user.avatar_url,
    url: user.permalink_url,
}
"#,
    },
    Template {
        key: "instagram",
        label: "Instagram account: new posts (needs login)",
        name: "Instagram: nasa",
        interval_mins: 180,
        script: r#"// New posts from an Instagram account.
// Instagram refuses anonymous requests, so this needs your login session:
// paste the value of the `sessionid` cookie from instagram.com
// (browser dev tools → Storage/Application → Cookies).
// Desktop app only: browsers don't let web pages send cookies to other sites.
// Keep the interval long; Instagram rate-limits aggressively.
let username = "nasa";
let sessionid = "";

if sessionid == "" { throw "set `sessionid` in the script first"; }
let resp = fetch_json("https://www.instagram.com/api/v1/users/web_profile_info/?username=" + username, #{
    headers: #{
        "x-ig-app-id": "936619743392459",
        "cookie": "sessionid=" + sessionid,
    }
});
let user = resp.data.user;
let posts = user.edge_owner_to_timeline_media.edges.map(|e| {
    let n = e.node;
    let captions = n.edge_media_to_caption?.edges ?? [];
    let caption = if captions.is_empty() { "" } else { captions[0].node.text };
    #{
        id: n.shortcode,
        url: "https://www.instagram.com/p/" + n.shortcode + "/",
        image: n.thumbnail_src ?? n.display_url,
        text: caption,
    }
});

#{
    title: user.full_name ?? username,
    url: "https://www.instagram.com/" + username + "/",
    image: user.profile_pic_url,
    items: posts,
}
"#,
    },
    Template {
        key: "biorio",
        label: "Bio Rio: reRUN screenings",
        name: "Bio Rio reRUN",
        interval_mins: 180,
        script: r#"// New screenings in Bio Rio's reRUN category.
let base = "https://www.biorio.se";
let page_url = base + "/sv/kategori/rerun";

// Next.js image URLs wrap the original: /_next/image?url=<encoded>&w=...
fn unwrap_image(src, base) {
    let inner = regex_find(src ?? "", "[?&]url=([^&]+)");
    if inner == () { url_join(base, src) } else { url_join(base, url_decode(inner)) }
}

let doc = html(fetch(page_url));
let items = doc.select("article.showtime-card").map(|card| {
    let img = card.select_one("img.showtime-thumbnail-image");
    #{
        // Each booking link is one screening.
        url: url_join(base, card.select_one("a.showtime-card-link").attr("href")),
        title: img.attr("alt"),
        date: card.select_one(".showtime-date").text() + " " + card.select_one(".showtime-time").text(),
        image: unwrap_image(img.attr("src"), base),
        text: card.select_one(".showtime-synopsis").text(),
    }
});
if items.is_empty() { throw "no screenings found; has the page layout changed?"; }

#{
    title: "Bio Rio · reRUN",
    url: page_url,
    items: items,
}
"#,
    },
    Template {
        key: "bioaspen",
        label: "Bio Aspen: Classics",
        name: "Bio Aspen Classics",
        interval_mins: 180,
        script: r#"// New films in Bio Aspen's Classics series.
let page_url = "https://www.bioaspen.se/visningar/classics/";

let doc = html(fetch(page_url));
let films = doc.select("div.movie > a").map(|a| #{
    url: a.attr("href"),
    title: a.attr("title"),
    image: a.select_one("img").attr("src"),
});
if films.is_empty() { throw "no films found; has the page layout changed?"; }

#{
    title: "Bio Aspen · Classics",
    url: page_url,
    items: films,
}
"#,
    },
    Template {
        key: "slakthuset",
        label: "Slakthuset: events",
        name: "Slakthuset events",
        interval_mins: 180,
        script: r#"// New events at Slakthuset.
let base = "https://slakthusetclub.se";

fn unwrap_image(src, base) {
    let inner = regex_find(src ?? "", "[?&]url=([^&]+)");
    if inner == () { url_join(base, src) } else { url_join(base, url_decode(inner)) }
}

let doc = html(fetch(base + "/events"));
let events = doc.select("main li a[href^='/event/']").map(|a| #{
    url: url_join(base, a.attr("href")),
    title: a.select_one("h3").text(),
    date: a.select_one("p").text(),
    image: unwrap_image(a.select_one("img").attr("src"), base),
});
if events.is_empty() { throw "no events found; has the page layout changed?"; }

#{
    title: "Slakthuset",
    url: base + "/events",
    items: events,
}
"#,
    },
    Template {
        key: "feed",
        label: "Any RSS/Atom feed",
        name: "Feed",
        interval_mins: 60,
        script: r#"// New entries in any RSS or Atom feed.
let feed = parse_feed(fetch("https://example.com/feed.xml"));
#{
    title: feed.title,
    url: feed.url,
    image: feed.image,
    items: feed.items,
}
"#,
    },
    Template {
        key: "page-section",
        label: "Any page: section text changed",
        name: "Page watch",
        interval_mins: 60,
        script: r#"// Reports when the text inside a CSS selector changes.
let page_url = "https://example.com/";
let selector = "main";

let doc = html(fetch(page_url));
let section = doc.select_one(selector);
if section == () { throw "selector matched nothing: " + selector; }

#{
    title: doc.select_one("title").text(),
    url: page_url,
    text: section.text(),
}
"#,
    },
    Template {
        key: "links",
        label: "Any page: new links",
        name: "New links",
        interval_mins: 60,
        script: r#"// Reports new links inside a part of a page.
let page_url = "https://example.com/";
let selector = "main a[href]";

let doc = html(fetch(page_url));
#{
    title: doc.select_one("title").text(),
    url: page_url,
    items: doc.select(selector).map(|a| #{
        title: a.text(),
        url: url_join(page_url, a.attr("href")),
    }),
}
"#,
    },
];

pub fn find(key: &str) -> Option<&'static Template> {
    TEMPLATES.iter().find(|t| t.key == key)
}
