//! Message catalogs shared with the former web interface. French source strings
//! are keys; other languages are generated catalogs in `locales/messages`.
use std::{
    collections::HashMap,
    sync::{OnceLock, RwLock},
};

pub struct Language {
    pub code: &'static str,
    pub name: &'static str,
}

pub const LANGUAGES: &[Language] = &[
    Language {
        code: "en",
        name: "English",
    },
    Language {
        code: "fr",
        name: "Français",
    },
    Language {
        code: "es",
        name: "Español",
    },
    Language {
        code: "pt-BR",
        name: "Português (Brasil)",
    },
    Language {
        code: "de",
        name: "Deutsch",
    },
    Language {
        code: "it",
        name: "Italiano",
    },
    Language {
        code: "nl",
        name: "Nederlands",
    },
    Language {
        code: "pl",
        name: "Polski",
    },
    Language {
        code: "ru",
        name: "Русский",
    },
    Language {
        code: "uk",
        name: "Українська",
    },
    Language {
        code: "tr",
        name: "Türkçe",
    },
    Language {
        code: "ar",
        name: "العربية",
    },
    Language {
        code: "hi",
        name: "हिन्दी",
    },
    Language {
        code: "bn",
        name: "বাংলা",
    },
    Language {
        code: "ur",
        name: "اردو",
    },
    Language {
        code: "id",
        name: "Bahasa Indonesia",
    },
    Language {
        code: "vi",
        name: "Tiếng Việt",
    },
    Language {
        code: "th",
        name: "ไทย",
    },
    Language {
        code: "ko",
        name: "한국어",
    },
    Language {
        code: "ja",
        name: "日本語",
    },
    Language {
        code: "zh-CN",
        name: "简体中文",
    },
    Language {
        code: "zh-TW",
        name: "繁體中文",
    },
    Language {
        code: "fa",
        name: "فارسی",
    },
    Language {
        code: "he",
        name: "עברית",
    },
    Language {
        code: "sw",
        name: "Kiswahili",
    },
];

macro_rules! catalogs {
    ($($code:literal => $file:literal),* $(,)?) => {
        fn raw(code: &str) -> Option<&'static str> {
            match code {
                "fr" => Some(include_str!("../../locales/source.json")),
                $($code => Some(include_str!(concat!("../../locales/messages/", $file))),)*
                _ => None,
            }
        }
    };
}

catalogs! {
    "en" => "en.json", "es" => "es.json", "pt-BR" => "pt-BR.json", "de" => "de.json",
    "it" => "it.json", "nl" => "nl.json", "pl" => "pl.json", "ru" => "ru.json",
    "uk" => "uk.json", "tr" => "tr.json", "ar" => "ar.json", "hi" => "hi.json",
    "bn" => "bn.json", "ur" => "ur.json", "id" => "id.json", "vi" => "vi.json",
    "th" => "th.json", "ko" => "ko.json", "ja" => "ja.json", "zh-CN" => "zh-CN.json",
    "zh-TW" => "zh-TW.json", "fa" => "fa.json", "he" => "he.json", "sw" => "sw.json",
}

type Catalog = HashMap<String, String>;

struct State {
    locale: &'static str,
    catalog: &'static Catalog,
}

fn load(code: &str) -> &'static Catalog {
    static CACHE: OnceLock<RwLock<HashMap<String, &'static Catalog>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(catalog) = cache.read().unwrap_or_else(|e| e.into_inner()).get(code) {
        return catalog;
    }
    let catalog: Catalog = raw(code)
        .and_then(|text| serde_json::from_str(text).ok())
        .unwrap_or_default();
    // Catalogs live for the whole process; at most one per selected language.
    let catalog: &'static Catalog = Box::leak(Box::new(catalog));
    cache
        .write()
        .unwrap_or_else(|e| e.into_inner())
        .insert(code.to_owned(), catalog);
    catalog
}

fn state() -> &'static RwLock<State> {
    static STATE: OnceLock<RwLock<State>> = OnceLock::new();
    STATE.get_or_init(|| {
        RwLock::new(State {
            locale: "fr",
            catalog: load("fr"),
        })
    })
}

/// Same resolution as the web interface: system languages, regional fallbacks, English.
pub fn resolve(requested: &str, system: &[String]) -> &'static str {
    let requested_list = [requested.to_owned()];
    let candidates: &[String] = if requested == "system" {
        system
    } else {
        &requested_list
    };
    for candidate in candidates {
        if let Some(language) = LANGUAGES.iter().find(|l| l.code == candidate) {
            return language.code;
        }
        let lower = candidate.to_lowercase();
        if lower.starts_with("zh") {
            return if ["tw", "hk", "hant"].iter().any(|v| lower.contains(v)) {
                "zh-TW"
            } else {
                "zh-CN"
            };
        }
        if lower.starts_with("pt") {
            return "pt-BR";
        }
        let base = candidate.split(['-', '_']).next().unwrap_or_default();
        if let Some(language) = LANGUAGES.iter().find(|l| l.code == base) {
            return language.code;
        }
    }
    "en"
}

pub fn system_languages() -> Vec<String> {
    sys_locale::get_locales()
        .map(|locale| locale.replace('_', "-"))
        .collect()
}

pub fn set_language(requested: &str) -> &'static str {
    let locale = resolve(requested, &system_languages());
    let catalog = load(locale);
    let mut state = state().write().unwrap_or_else(|e| e.into_inner());
    state.locale = locale;
    state.catalog = catalog;
    locale
}

pub fn locale() -> &'static str {
    state().read().unwrap_or_else(|e| e.into_inner()).locale
}

fn lookup(key: &str) -> Option<&'static str> {
    let state = state().read().unwrap_or_else(|e| e.into_inner());
    state
        .catalog
        .get(key)
        .filter(|value| !value.is_empty())
        .or_else(|| load("en").get(key).filter(|value| !value.is_empty()))
        .map(String::as_str)
}

fn exists(key: &str) -> bool {
    let state = state().read().unwrap_or_else(|e| e.into_inner());
    state.catalog.contains_key(key) || load("en").contains_key(key)
}

/// Interpolation value: numbers are localized, except `count` (i18next behavior).
pub enum Arg<'a> {
    Text(&'a str),
    Number(f64),
}

impl<'a> From<&'a str> for Arg<'a> {
    fn from(value: &'a str) -> Self {
        Arg::Text(value)
    }
}
impl<'a> From<&'a String> for Arg<'a> {
    fn from(value: &'a String) -> Self {
        Arg::Text(value)
    }
}
impl From<f64> for Arg<'_> {
    fn from(value: f64) -> Self {
        Arg::Number(value)
    }
}
impl From<usize> for Arg<'_> {
    fn from(value: usize) -> Self {
        Arg::Number(value as f64)
    }
}
impl From<i64> for Arg<'_> {
    fn from(value: i64) -> Self {
        Arg::Number(value as f64)
    }
}
impl From<u64> for Arg<'_> {
    fn from(value: u64) -> Self {
        Arg::Number(value as f64)
    }
}

/// `t(key, params)`: translated message with `{{name}}` interpolation.
pub fn tp(key: &str, params: &[(&str, Arg)]) -> String {
    let mut text = lookup(key).unwrap_or(key).to_owned();
    for (name, value) in params {
        let value = match value {
            Arg::Text(text) => (*text).to_owned(),
            Arg::Number(n) if *name == "count" => {
                if n.fract() == 0.0 {
                    format!("{}", *n as i64)
                } else {
                    n.to_string()
                }
            }
            Arg::Number(n) => crate::intl::plain(locale(), *n),
        };
        text = text.replace(&format!("{{{{{name}}}}}"), &value);
    }
    text
}

/// Translation in a given language without changing the interface language
/// (tray indicators run outside the UI thread).
pub fn translate_in(language: &str, key: &str) -> String {
    load(language)
        .get(key)
        .filter(|v| !v.is_empty())
        .or_else(|| load("en").get(key).filter(|v| !v.is_empty()))
        .cloned()
        .unwrap_or_else(|| key.to_owned())
}

pub fn t(key: &str) -> String {
    tp(key, &[])
}

struct Template {
    key: &'static str,
    literals: Vec<&'static str>,
    variables: Vec<&'static str>,
}

fn templates() -> &'static [Template] {
    static TEMPLATES: OnceLock<Vec<Template>> = OnceLock::new();
    TEMPLATES.get_or_init(|| {
        let source: HashMap<&'static str, &'static str> =
            serde_json::from_str(raw("fr").unwrap_or("{}")).unwrap_or_default();
        let mut result: Vec<Template> = source
            .into_keys()
            .filter(|key| !key.contains("{{"))
            .filter_map(|key| {
                let mut literals = Vec::new();
                let mut variables = Vec::new();
                let mut rest = key;
                let mut start = 0;
                while let Some(open) = rest[start..].find('{').map(|i| start + i) {
                    let close = rest[open..].find('}').map(|i| open + i)?;
                    let name = &rest[open + 1..close];
                    if name.is_empty()
                        || !name.chars().next().unwrap().is_ascii_alphabetic()
                            && !name.starts_with('_')
                        || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                    {
                        start = close + 1;
                        continue;
                    }
                    literals.push(&rest[..open]);
                    variables.push(name);
                    rest = &rest[close + 1..];
                    start = 0;
                }
                if variables.is_empty() {
                    return None;
                }
                literals.push(rest);
                Some(Template {
                    key,
                    literals,
                    variables,
                })
            })
            .collect();
        result.sort_by_key(|t| t.key);
        result
    })
}

/// Lazy (`.+?`) template match anchored at both ends, like the web
/// implementation's generated regular expressions.
fn capture<'a>(text: &'a str, literals: &[&str], out: &mut Vec<&'a str>) -> bool {
    let Some(rest) = text.strip_prefix(literals[0]) else {
        return false;
    };
    fill(rest, &literals[1..], out)
}

fn fill<'a>(text: &'a str, literals: &[&str], out: &mut Vec<&'a str>) -> bool {
    let Some((next, remaining)) = literals.split_first() else {
        return text.is_empty();
    };
    // Each variable captures at least one character.
    let mut from = text.chars().next().map_or(1, char::len_utf8);
    while from <= text.len() {
        let Some(position) = text[from..].find(next).map(|p| p + from) else {
            return false;
        };
        out.push(&text[..position]);
        if fill(&text[position + next.len()..], remaining, out) {
            return true;
        }
        out.pop();
        from = position + text[position..].chars().next().map_or(1, char::len_utf8);
    }
    false
}

/// `tx(value)`: translate interface text, keeping surrounding spaces. Unknown
/// values (hardware names, user labels) are returned unchanged.
pub fn tx(value: &str) -> String {
    if value.trim().is_empty() {
        return value.to_owned();
    }
    let key = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let prefix = if value.starts_with(char::is_whitespace) {
        " "
    } else {
        ""
    };
    let suffix = if value.ends_with(char::is_whitespace) {
        " "
    } else {
        ""
    };
    if exists(&key) {
        return format!("{prefix}{}{suffix}", t(&key));
    }
    for template in templates() {
        let mut values = Vec::new();
        if capture(&key, &template.literals, &mut values) {
            let mut text = t(template.key);
            for (name, value) in template.variables.iter().zip(values) {
                text = text.replace(&format!("{{{name}}}"), value);
            }
            return format!("{prefix}{text}{suffix}");
        }
    }
    value.to_owned()
}

/// Localized integer, as `tx(number)` rendered numbers.
pub fn tn(value: impl Into<f64>) -> String {
    crate::intl::plain(locale(), value.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::prelude::v1::test;

    #[test]
    fn resolves_like_the_web_interface() {
        assert_eq!(resolve("system", &["fr-FR".into()]), "fr");
        assert_eq!(resolve("system", &["zh-HK".into()]), "zh-TW");
        assert_eq!(resolve("system", &["pt-PT".into()]), "pt-BR");
        assert_eq!(resolve("system", &["xx".into()]), "en");
        assert_eq!(resolve("ja", &[]), "ja");
    }

    #[test]
    fn translates_and_interpolates() {
        set_language("en");
        assert_eq!(tx("Aller au contenu"), "Skip to content");
        assert_eq!(tx(" Aller au contenu "), " Skip to content ");
        assert_eq!(tx("Une valeur inconnue"), "Une valeur inconnue");
        assert_eq!(
            tp("Non vérifiés : {{count}}", &[("count", 3usize.into())]),
            t("Non vérifiés : {{count}}").replace("{{count}}", "3")
        );
        set_language("fr");
        assert_eq!(
            tx("Rapport non sauvegardé : disque plein"),
            "Rapport non sauvegardé : disque plein"
        );
    }

    #[test]
    fn catalogs_translate_interface_labels() {
        assert_eq!(translate_in("ja", "Vue d’ensemble"), "概要");
        assert_eq!(translate_in("ja", "Anomalie détectée"), "問題を検出");
        assert_eq!(translate_in("ar", "Non vérifié"), "لم يتم التحقق");
        assert_eq!(translate_in("ja", "FIRMWARE-PART-123"), "FIRMWARE-PART-123");
    }

    #[test]
    fn dynamic_templates_capture_values() {
        set_language("fr");
        let mut values = Vec::new();
        assert!(capture(
            "3 erreur(s) corrigée(s), 1 non corrigée(s).",
            &["", " erreur(s) corrigée(s), ", " non corrigée(s)."],
            &mut values
        ));
        assert_eq!(values, ["3", "1"]);
    }
}
