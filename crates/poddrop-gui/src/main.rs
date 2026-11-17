use poddrop_core::i18n::{Language, Translator};
use poddrop_core::{APP_NAME, APP_VERSION};

fn main() {
    let tr = Translator::new(Language::from_env());
    println!("{APP_NAME} GUI ({APP_VERSION}) - {}", tr.t("app.tagline"));
    println!("GTK4 UI will be implemented in Phase 3.");
}
