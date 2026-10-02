//! Remote images (avatars, thumbnails). The webview loads `<img src>` by
//! itself; here each URL is downloaded once, when its widget first scrolls
//! into view, and kept for the session.

use std::collections::HashMap;

use iced::widget::image::Handle;
use iced::widget::{container, image, sensor};
use iced::{ContentFit, Element, Length, Task};

use crate::Message;

enum Entry {
    Loading,
    Ready(Handle),
    Failed,
}

#[derive(Default)]
pub struct Images {
    entries: HashMap<String, Entry>,
}

impl Images {
    /// Starts downloading `url` unless it's already known.
    pub fn load(&mut self, url: String) -> Task<Message> {
        if url.is_empty() || self.entries.contains_key(&url) {
            return Task::none();
        }
        self.entries.insert(url.clone(), Entry::Loading);
        let fetch_url = url.clone();
        Task::perform(async move { tabkeeper::fetch::fetch_bytes(&fetch_url).await }, move |r| {
            Message::ImageLoaded(url.clone(), r)
        })
    }

    pub fn loaded(&mut self, url: String, result: Result<Vec<u8>, String>) {
        let entry = match result {
            Ok(bytes) => Entry::Ready(Handle::from_bytes(bytes)),
            Err(_) => Entry::Failed,
        };
        self.entries.insert(url, entry);
    }

    pub fn failed(&self, url: &str) -> bool {
        matches!(self.entries.get(url), Some(Entry::Failed))
    }

    /// The image at `url`, cropped to fill `w`×`h`. Until it has loaded,
    /// `placeholder` is shown; it requests the download when it becomes visible.
    pub fn view<'a>(
        &self,
        url: &str,
        w: f32,
        h: f32,
        radius: f32,
        placeholder: impl Into<Element<'a, Message>>,
    ) -> Element<'a, Message> {
        match self.entries.get(url) {
            Some(Entry::Ready(handle)) => image(handle.clone())
                .width(w)
                .height(h)
                .content_fit(ContentFit::Cover)
                .border_radius(radius)
                .into(),
            Some(_) => container(placeholder).width(w).height(h).into(),
            None => {
                let url = url.to_string();
                sensor(container(placeholder).width(Length::Fixed(w)).height(Length::Fixed(h)))
                    .key(url.clone())
                    .anticipate(300)
                    .on_show(move |_| Message::LoadImage(url.clone()))
                    .into()
            }
        }
    }
}
