//! One module per page, like the Dioxus `ui/` routes.

pub mod dashboard;
pub mod editor;
pub mod interests;
pub mod settings;
pub mod stats;
pub mod updates;

use iced::widget::column;

use crate::widgets::{btn, h2, panel, Element};
use crate::{App, Message, Route};

/// The current page and its view state.
pub enum Page {
    Dashboard,
    /// `confirm_delete`: the interest whose Delete button was clicked.
    Interests { confirm_delete: Option<String> },
    Editor(Box<editor::Editor>),
    NotFound,
    /// `shown`: how many events are listed.
    Updates { confirm_clear: bool, shown: usize },
    Stats,
    Settings(Box<settings::SettingsPage>),
}

impl Page {
    pub fn open(app: &App, route: Route) -> Page {
        match route {
            Route::Dashboard => Page::Dashboard,
            Route::Interests => Page::Interests { confirm_delete: None },
            Route::Editor(id) => editor::Editor::open(app, &id).map(|e| Page::Editor(Box::new(e))).unwrap_or(Page::NotFound),
            Route::Updates => Page::Updates { confirm_clear: false, shown: updates::PAGE_SIZE },
            Route::Stats => Page::Stats,
            Route::Settings => Page::Settings(Box::new(settings::SettingsPage::new(app))),
        }
    }

    /// Whether this page is `route`, for highlighting the sidebar.
    pub fn is(&self, route: &Route) -> bool {
        matches!(
            (self, route),
            (Page::Dashboard, Route::Dashboard)
                | (Page::Interests { .. }, Route::Interests)
                | (Page::Updates { .. }, Route::Updates)
                | (Page::Stats, Route::Stats)
                | (Page::Settings(_), Route::Settings)
        )
    }
}

pub fn view(app: &App) -> Element<'_> {
    match &app.page {
        Page::Dashboard => dashboard::view(app),
        Page::Interests { confirm_delete } => interests::view(app, confirm_delete.as_deref()),
        Page::Editor(e) => editor::view(app, e),
        Page::NotFound => panel(
            column![h2("Interest not found"), btn("Back to interests", Some(Message::Navigate(Route::Interests)))].spacing(12),
        )
        .into(),
        Page::Updates { confirm_clear, shown } => updates::view(app, *confirm_clear, *shown),
        Page::Stats => stats::view(app),
        Page::Settings(s) => settings::view(app, s),
    }
}
