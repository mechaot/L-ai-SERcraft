// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Localization (replaces .resx `ResourceManager` + WinForms `ApplyResources`).
//!
//! Catalogs are `resources/i18n/<culture>.json`, embedded. Lookup per key: current culture, then `en`,
//! then the key itself. Culture comes from setting "User Language" (C# `Program.cs`/`MainForm.SetLanguage`;
//! stored as culture name string, e.g. "de", "pt-BR"); absent = system locale, else `en`.

use serde::Deserialize;
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::OnceLock;

/// Settings key (exact C# spelling, with space).
pub const SETTING_KEY: &str = "User Language";

static CATALOGS: &[(&str, &str)] = &[
    ("en", include_str!("../../../resources/i18n/en.json")),
    ("it", include_str!("../../../resources/i18n/it.json")),
    ("es", include_str!("../../../resources/i18n/es.json")),
    ("fr", include_str!("../../../resources/i18n/fr.json")),
    ("de", include_str!("../../../resources/i18n/de.json")),
    ("da", include_str!("../../../resources/i18n/da.json")),
    ("pt-BR", include_str!("../../../resources/i18n/pt-BR.json")),
    ("ru", include_str!("../../../resources/i18n/ru.json")),
    ("zh-CN", include_str!("../../../resources/i18n/zh-CN.json")),
    ("zh-TW", include_str!("../../../resources/i18n/zh-TW.json")),
    ("sk-SK", include_str!("../../../resources/i18n/sk-SK.json")),
    ("hu-HU", include_str!("../../../resources/i18n/hu-HU.json")),
    ("cs-CZ", include_str!("../../../resources/i18n/cs-CZ.json")),
    ("pl-PL", include_str!("../../../resources/i18n/pl-PL.json")),
    ("el-GR", include_str!("../../../resources/i18n/el-GR.json")),
    ("tr-TR", include_str!("../../../resources/i18n/tr-TR.json")),
    ("ro-RO", include_str!("../../../resources/i18n/ro-RO.json")),
    ("nl-NL", include_str!("../../../resources/i18n/nl-NL.json")),
    ("uk", include_str!("../../../resources/i18n/uk.json")),
    ("ja-JP", include_str!("../../../resources/i18n/ja-JP.json")),
];
static CULTURES_JSON: &str = include_str!("../../../resources/i18n/cultures.json");

/// One entry of the language menu (`cultures.json`).
#[derive(Clone, Debug, Deserialize)]
pub struct Culture {
    /// Culture name, e.g. "pt-BR".
    pub culture: String,
    /// English display name.
    pub name: String,
    /// Icon key of the flag, e.g. "flags-gb".
    pub flag: String,
}

/// Language menu entries in C# menu order.
pub fn cultures() -> &'static [Culture] {
    static C: OnceLock<Vec<Culture>> = OnceLock::new();
    C.get_or_init(|| serde_json::from_str(CULTURES_JSON).unwrap_or_default())
}

/// A loaded string catalog for one culture with `en` fallback.
pub struct Catalog {
    culture: &'static str,
    main: HashMap<String, String>,
    fallback: HashMap<String, String>,
}

/// Parse one embedded catalog.
fn parse(culture: &str) -> HashMap<String, String> {
    CATALOGS
        .iter()
        .find(|(c, _)| *c == culture)
        .and_then(|(_, j)| serde_json::from_str(j).ok())
        .unwrap_or_default()
}

/// Map a free-form locale (e.g. "de_DE.UTF-8", "pt-br", "zh_TW") to an embedded culture name.
///
/// `locale`: locale text. Returns culture name (exact, then language match) or None.
pub fn resolve_culture(locale: &str) -> Option<&'static str> {
    let l = locale.split(['.', '@']).next()?.replace('_', "-").to_lowercase();
    if l.is_empty() || l == "c" || l == "posix" {
        return None;
    }
    if let Some((c, _)) = CATALOGS.iter().find(|(c, _)| c.to_lowercase() == l) {
        return Some(c);
    }
    let lang = l.split('-').next()?;
    // prefer bare language file, else first culture with that language
    CATALOGS
        .iter()
        .find(|(c, _)| c.to_lowercase() == lang)
        .or_else(|| CATALOGS.iter().find(|(c, _)| c.to_lowercase().split('-').next() == Some(lang)))
        .map(|(c, _)| *c)
}

/// System locale from env (`LC_ALL`, `LC_MESSAGES`, `LANG`, `LANGUAGE`).
fn system_locale() -> Option<String> {
    ["LC_ALL", "LC_MESSAGES", "LANG", "LANGUAGE"]
        .iter()
        .filter_map(|k| std::env::var(k).ok())
        .map(|v| v.split(':').next().unwrap_or("").to_owned())
        .find(|v| !v.is_empty())
}

impl Catalog {
    /// Load catalog.
    ///
    /// `culture`: culture name or locale text; unknown -> `en` only.
    pub fn load(culture: &str) -> Catalog {
        let c = resolve_culture(culture).unwrap_or("en");
        Catalog {
            culture: c,
            main: if c == "en" { HashMap::new() } else { parse(c) },
            fallback: parse("en"),
        }
    }

    /// Culture actually in use.
    pub fn culture(&self) -> &'static str {
        self.culture
    }

    /// Translate: current -> en -> key.
    ///
    /// `key`: e.g. "MainForm.MnConnect.Text". Returns text (borrowed from catalog or `key`).
    pub fn get<'a>(&'a self, key: &'a str) -> &'a str {
        self.main
            .get(key)
            .or_else(|| self.fallback.get(key))
            .map(String::as_str)
            .unwrap_or(key)
    }

    /// True if key exists in current or fallback catalog.
    pub fn has(&self, key: &str) -> bool {
        self.main.contains_key(key) || self.fallback.contains_key(key)
    }
}

static GLOBAL: OnceLock<Catalog> = OnceLock::new();

/// Global catalog; first use reads the "User Language" setting (restart needed to change, as in C#).
pub fn catalog() -> &'static Catalog {
    GLOBAL.get_or_init(|| {
        let saved: Option<String> = lg_base::settings::get(SETTING_KEY, None::<String>);
        let loc = saved.or_else(system_locale).unwrap_or_else(|| "en".into());
        Catalog::load(&loc)
    })
}

/// Persist the language choice (C# `MainForm.SetLanguage`); takes effect after restart.
///
/// `culture`: culture name to store, or None to delete the setting (= system default).
pub fn set_language(culture: Option<&str>) {
    match culture {
        Some(c) => lg_base::settings::set(SETTING_KEY, c),
        None => lg_base::settings::delete(SETTING_KEY),
    }
}

/// Translate a key with fallback to `en` then to the key.
///
/// `key`: resource key. Returns the localized text.
pub fn tr(key: &str) -> &str {
    catalog().get(key)
}

/// Translate and strip WinForms mnemonic `&` (`&Connect` -> `Connect`, `&&` -> `&`).
///
/// `key`: resource key. Returns plain text.
pub fn tr_plain(key: &str) -> String {
    strip_mnemonic(tr(key))
}

/// Translate and format `{0}` placeholders (C# `string.Format(Strings.X, a, b)`).
///
/// `key`: resource key. `args`: positional args (already culture-correct). Returns formatted text.
pub fn tr_fmt(key: &str, args: &[&dyn Display]) -> String {
    format_cs(tr(key), args)
}

/// C# `string.Format` subset: `{n}`, `{n:spec}` (spec ignored, pre-format args), `{{`, `}}`.
///
/// `fmt`: format text. `args`: positional args. Returns result; missing arg index stays literal.
pub fn format_cs(fmt: &str, args: &[&dyn Display]) -> String {
    let mut out = String::with_capacity(fmt.len() + 16);
    let mut it = fmt.chars().peekable();
    while let Some(ch) = it.next() {
        match ch {
            '{' if it.peek() == Some(&'{') => {
                it.next();
                out.push('{');
            }
            '}' if it.peek() == Some(&'}') => {
                it.next();
                out.push('}');
            }
            '{' => {
                // read until closing brace
                let mut inner = String::new();
                let mut closed = false;
                for c2 in it.by_ref() {
                    if c2 == '}' {
                        closed = true;
                        break;
                    }
                    inner.push(c2);
                }
                let idx = inner.split([':', ',']).next().and_then(|s| s.trim().parse::<usize>().ok());
                match (closed, idx.and_then(|i| args.get(i))) {
                    (true, Some(a)) => out.push_str(&a.to_string()),
                    _ => {
                        out.push('{');
                        out.push_str(&inner);
                        if closed {
                            out.push('}');
                        }
                    }
                }
            }
            _ => out.push(ch),
        }
    }
    out
}

/// Remove WinForms mnemonic markers.
///
/// `s`: text with `&`. Returns text where `&x` -> `x` and `&&` -> `&`.
pub fn strip_mnemonic(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c == '&' {
            if it.peek() == Some(&'&') {
                it.next();
                out.push('&');
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_chain() {
        let it = Catalog::load("it");
        assert_eq!(it.culture(), "it");
        assert_eq!(it.get("No.Such.Key"), "No.Such.Key");
        let en = Catalog::load("en");
        assert_eq!(en.get("MainForm.MnConnect.Text"), "&Connect");
        assert_eq!(it.get("SaveOptionForm.BtnSave.Text"), "Salva");
    }

    #[test]
    fn locale_resolution() {
        assert_eq!(resolve_culture("de_DE.UTF-8"), Some("de"));
        assert_eq!(resolve_culture("pt_BR"), Some("pt-BR"));
        assert_eq!(resolve_culture("zh_TW.UTF-8"), Some("zh-TW"));
        assert_eq!(resolve_culture("cs_CZ"), Some("cs-CZ"));
        assert_eq!(resolve_culture("C"), None);
        assert_eq!(resolve_culture("xx_YY"), None);
    }

    #[test]
    fn formatting() {
        assert_eq!(format_cs("{0} of {1} {{x}}", &[&1, &"b"]), "1 of b {x}");
        assert_eq!(format_cs("{0:0.00}", &[&"1.50"]), "1.50");
        assert_eq!(format_cs("{2}", &[&1]), "{2}");
        assert_eq!(strip_mnemonic("&File && &Edit"), "File & Edit");
    }

    #[test]
    fn cultures_listed() {
        assert_eq!(cultures().len(), 20);
        assert!(CATALOGS.iter().all(|(c, _)| cultures().iter().any(|x| &x.culture == c)));
    }
}
