use std::{collections::HashMap, sync::OnceLock};

static TRANSLATIONS: OnceLock<HashMap<String, String>> = OnceLock::new();

static EN: &str = include_str!("../locales/en.json");
static FR: &str = include_str!("../locales/fr.json");
static ES: &str = include_str!("../locales/es.json");
static DE: &str = include_str!("../locales/de.json");

pub fn init(lang: &str) {
    let json = match lang {
        "fr" => FR,
        "es" => ES,
        "de" => DE,
        _ => EN,
    };
    let map: HashMap<String, String> = serde_json::from_str(json).unwrap_or_default();
    let _ = TRANSLATIONS.set(map);
}

pub fn translate(key: &str, args: &[(&str, &str)]) -> String {
    let map = TRANSLATIONS.get();
    let mut s = map
        .and_then(|m| m.get(key))
        .cloned()
        .unwrap_or_else(|| key.to_string());
    for (k, v) in args {
        s = s.replace(&format!("{{{}}}", k), v);
    }
    s
}

#[macro_export]
macro_rules! t {
    ($key:expr) => {
        $crate::i18n::translate($key, &[])
    };
    ($key:expr, $($k:literal => $v:expr),+ $(,)?) => {
        $crate::i18n::translate($key, &[$( ($k, &format!("{}", $v)) ),+])
    };
}
