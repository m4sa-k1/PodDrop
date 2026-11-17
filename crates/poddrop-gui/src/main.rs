mod dialogs;
mod window;

use adw::prelude::*;
use gtk4::gdk;
use libadwaita as adw;

const APP_ID: &str = "io.github.m4sa_k1.PodDrop";
const APP_CSS: &str = include_str!("style.css");

fn main() -> gtk4::glib::ExitCode {
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_startup(|_| {
        adw::StyleManager::default().set_color_scheme(adw::ColorScheme::ForceDark);
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
