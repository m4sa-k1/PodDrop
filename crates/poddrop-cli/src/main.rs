use poddrop_core::i18n::{Language, Translator};
use poddrop_core::{APP_NAME, APP_VERSION};

fn main() {
    let mut args = std::env::args().skip(1);
    let mut lang = Language::from_env();
    let mut show_version = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--version" | "-V" => show_version = true,
            "--lang" => {
                if let Some(val) = args.next() {
                    lang = Language::from_locale_str(&val);
                }
            }
            "--help" | "-h" => {
                let tr = Translator::new(lang);
                println!("{APP_NAME} {APP_VERSION} - {}", tr.t("app.tagline"));
                println!("{}", tr.t("cli.usage"));
                return;
            }
            _ => {}
        }
    }

    if show_version {
        println!("{APP_NAME} {APP_VERSION}");
        return;
    }

    let tr = Translator::new(lang);
    println!("{APP_NAME} {APP_VERSION} - {}", tr.t("app.tagline"));
    println!("{}", tr.t("cli.usage"));
}
