mod notify;
mod ui;

fn main() {
    #[cfg(feature = "desktop")]
    {
        use dioxus::desktop::{Config, LogicalSize, WindowBuilder};
        let window = WindowBuilder::new().with_title("Tabkeeper").with_inner_size(LogicalSize::new(1280.0, 860.0));
        dioxus::LaunchBuilder::desktop().with_cfg(Config::new().with_window(window).with_menu(None)).launch(ui::App);
    }
    #[cfg(not(feature = "desktop"))]
    dioxus::launch(ui::App);
}
