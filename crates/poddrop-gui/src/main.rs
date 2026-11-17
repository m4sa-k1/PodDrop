mod dialogs;
mod window;

use gtk4::gdk;
use gtk4::prelude::*;

const APP_ID: &str = "io.github.m4sa_k1.PodDrop";
const APP_CSS: &str = include_str!("style.css");

fn main() -> gtk4::glib::ExitCode {
    let app = gtk4::Application::builder().application_id(APP_ID).build();
    app.connect_startup(|_| {
        if let Some(settings) = gtk4::Settings::default() {
            settings.set_gtk_application_prefer_dark_theme(true);
        }
        if let Some(display) = gdk::Display::default() {
            let provider = gtk4::CssProvider::new();
            provider.load_from_data(APP_CSS);
            gtk4::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk4::STYLE_PROVIDER_PRIORITY_USER + 100,
            );
        }
    });
    app.connect_activate(|app| {
        window::build_main_window(app);
    });
    app.run()
}
