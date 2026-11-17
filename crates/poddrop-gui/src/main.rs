mod window;

use adw::prelude::*;
use libadwaita as adw;

const APP_ID: &str = "io.github.m4sa_k1.PodDrop";

fn main() -> gtk4::glib::ExitCode {
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(|app| {
        window::build_main_window(app);
    });
    app.run()
}
