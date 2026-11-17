use std::collections::HashMap;

const EN_CATALOG: &str = include_str!("../../../locales/en.lang");
const JA_CATALOG: &str = include_str!("../../../locales/ja.lang");

/// Supported UI/CLI languages in PodDrop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    #[default]
    En,
    Ja,
}

impl Language {
    /// Detects language from a POSIX locale string (e.g. `ja_JP.UTF-8`).
    pub fn from_locale_str(locale: &str) -> Self {
        let lower = locale.trim().to_ascii_lowercase();
        if lower.starts_with("ja") {
            Self::Ja
        } else {
            Self::En
        }
    }

    /// Detects language from standard environment variables (`LC_ALL`, `LANG`).
    pub fn from_env() -> Self {
        for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Ok(val) = std::env::var(var) {
                if !val.is_empty() {
                    return Self::from_locale_str(&val);
                }
            }
        }
        Self::En
    }
}

/// Lightweight bilingual string lookup catalog.
#[derive(Debug, Clone)]
pub struct Translator {
    lang: Language,
    en: HashMap<&'static str, &'static str>,
    ja: HashMap<&'static str, &'static str>,
}

impl Translator {
    pub fn new(lang: Language) -> Self {
        Self {
            lang,
            en: parse_catalog(EN_CATALOG),
            ja: parse_catalog(JA_CATALOG),
        }
    }

    pub fn language(&self) -> Language {
        self.lang
    }

    /// Looks up a translation key, falling back to English, then the key itself.
    pub fn t<'a>(&'a self, key: &'a str) -> &'a str {
        let primary = match self.lang {
            Language::En => &self.en,
            Language::Ja => &self.ja,
        };
        primary
            .get(key)
            .copied()
            .or_else(|| self.en.get(key).copied())
            .unwrap_or(key)
    }
}

fn parse_catalog(raw: &'static str) -> HashMap<&'static str, &'static str> {
    raw.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return None;
            }
            trimmed.split_once('=')
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogs_have_matching_keys() {
        let en = parse_catalog(EN_CATALOG);
        let ja = parse_catalog(JA_CATALOG);
        assert!(!en.is_empty(), "English catalog must not be empty");
        for key in en.keys() {
            assert!(
                ja.contains_key(key),
                "Japanese catalog is missing key: {key}"
            );
        }
        for key in ja.keys() {
            assert!(
                en.contains_key(key),
                "English catalog is missing key: {key}"
            );
        }
    }

    #[test]
    fn translation_lookup_and_locale_detection() {
        assert_eq!(Language::from_locale_str("ja_JP.UTF-8"), Language::Ja);
        assert_eq!(Language::from_locale_str("en_US.UTF-8"), Language::En);

        let tr_ja = Translator::new(Language::Ja);
        assert_eq!(tr_ja.t("app.name"), "PodDrop");
        assert!(tr_ja.t("app.tagline").contains("iPod"));
        assert_eq!(tr_ja.t("nonexistent.key"), "nonexistent.key");
    }
}
