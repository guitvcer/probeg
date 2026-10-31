//! Translations. Every language is one `Locale` value, so a missing string is a
//! compile error rather than a blank in the report. To add a language, copy
//! `en.rs`, translate it and register it in `Lang` below.

mod de;
mod en;
mod es;
mod fr;
mod ja;
mod ko;
mod pt;
mod ru;
mod zh;

use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    En,
    Ru,
    Zh,
    Es,
    Pt,
    Fr,
    De,
    Ja,
    Ko,
}

impl Lang {
    pub const ALL: [Lang; 9] = [
        Lang::En,
        Lang::Ru,
        Lang::Zh,
        Lang::Es,
        Lang::Pt,
        Lang::Fr,
        Lang::De,
        Lang::Ja,
        Lang::Ko,
    ];

    pub fn locale(self) -> &'static Locale {
        match self {
            Lang::En => &en::LOCALE,
            Lang::Ru => &ru::LOCALE,
            Lang::Zh => &zh::LOCALE,
            Lang::Es => &es::LOCALE,
            Lang::Pt => &pt::LOCALE,
            Lang::Fr => &fr::LOCALE,
            Lang::De => &de::LOCALE,
            Lang::Ja => &ja::LOCALE,
            Lang::Ko => &ko::LOCALE,
        }
    }

    /// Understands `pt`, `pt-BR`, `pt_BR.UTF-8` and the like.
    pub fn parse(tag: &str) -> Option<Lang> {
        let primary = tag.split(['-', '_', '.', '@']).next()?.to_ascii_lowercase();
        Lang::ALL
            .into_iter()
            .find(|lang| lang.locale().code == primary)
    }

    /// Environment first, so one command can override it, then the macOS language
    /// list. `system` is lazy because asking macOS means spawning a process.
    pub fn detect(
        env: impl Fn(&str) -> Option<String>,
        system: impl FnOnce() -> Vec<String>,
    ) -> Lang {
        for key in ["PROBEG_LANG", "LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Some(lang) = env(key).as_deref().and_then(Lang::parse) {
                return lang;
            }
        }
        system()
            .iter()
            .find_map(|tag| Lang::parse(tag))
            .unwrap_or(Lang::En)
    }

    pub fn current() -> Lang {
        Lang::detect(|key| std::env::var(key).ok(), system_languages)
    }
}

fn system_languages() -> Vec<String> {
    crate::sys::run("defaults", &["read", "-g", "AppleLanguages"])
        .map(|text| parse_language_list(&text))
        .unwrap_or_default()
}

/// `( "en-KZ", "ru-KZ" )` in the old-style plist that `defaults` prints.
fn parse_language_list(text: &str) -> Vec<String> {
    text.split(['(', ')', ',', '\n'])
        .map(|item| item.trim().trim_matches('"').to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

/// Word forms for a count. How many forms a language uses is set by its `Plural`.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Forms {
    pub one: &'static str,
    pub few: &'static str,
    pub many: &'static str,
}

impl Forms {
    pub const fn two(one: &'static str, many: &'static str) -> Self {
        Self {
            one,
            few: many,
            many,
        }
    }

    pub const fn three(one: &'static str, few: &'static str, many: &'static str) -> Self {
        Self { one, few, many }
    }

    pub const fn same(word: &'static str) -> Self {
        Self {
            one: word,
            few: word,
            many: word,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub enum Plural {
    /// English, German, Spanish: 1 is singular.
    OneOther,
    /// French, Brazilian Portuguese: 0 and 1 are singular.
    ZeroOneOther,
    /// Russian: 1 / 2-4 / 5-20 with the last-digit rule.
    Slavic,
    /// Chinese, Japanese, Korean: no plural forms.
    None,
}

/// Fills `{0}`, `{1}`… so a translation can reorder its arguments.
pub fn fill(template: &str, args: &[&str]) -> String {
    let mut out = template.to_string();
    for (i, arg) in args.iter().enumerate() {
        out = out.replace(&format!("{{{i}}}"), arg);
    }
    out
}

#[derive(Debug, Serialize)]
pub struct Locale {
    /// ISO 639-1 code, what `--lang` and `LANG` use.
    pub code: &'static str,
    /// The language's own name, shown in `--help`.
    pub name: &'static str,
    pub plural: Plural,
    pub group_separator: char,
    pub decimal_separator: char,
    /// `true` for «12 d», `false` for «12天».
    pub unit_space: bool,

    pub kb: &'static str,
    pub mb: &'static str,
    pub gb: &'static str,
    pub tb: &'static str,
    pub pb: &'static str,
    pub mah: &'static str,
    pub unit_day: &'static str,
    pub unit_hour: &'static str,
    pub unit_minute: &'static str,
    pub unit_second: &'static str,

    pub months: [&'static str; 12],
    pub months_full: [&'static str; 12],
    /// `{d}` day, `{m}` month, `{y}` year, `{t}` time.
    pub date_format: &'static str,
    pub date_time_format: &'static str,
    pub month_year_format: &'static str,

    pub days: Forms,
    pub years: Forms,
    pub months_count: Forms,
    pub power_ons: Forms,

    pub section_history: &'static str,
    pub section_session: &'static str,
    pub section_lifetime: &'static str,
    pub section_battery: &'static str,
    pub section_memory: &'static str,
    pub section_storage: &'static str,
    pub section_network: &'static str,
    pub section_usage: &'static str,
    pub section_software: &'static str,
    pub spec_ram: &'static str,
    pub spec_ssd: &'static str,
    pub built: &'static str,
    pub first_setup: &'static str,
    pub os_installed: &'static str,
    pub last_update: &'static str,
    pub os_versions: &'static str,
    pub versions_major: &'static str,
    pub powered_on: &'static str,
    pub logged_in: &'static str,
    pub last_wake: &'static str,
    pub awake: &'static str,
    pub sleeps: &'static str,
    pub wakes: &'static str,
    pub wakes_note: &'static str,
    pub reboots: &'static str,
    pub ago: &'static str,
    pub since: &'static str,
    pub written: &'static str,
    pub read: &'static str,
    pub hours_on: &'static str,
    pub per_day: &'static str,
    pub wear: &'static str,
    pub years_left: &'static str,
    pub unsafe_shutdowns: &'static str,
    pub spare: &'static str,
    pub temperature: &'static str,
    pub media_errors: &'static str,
    pub charge: &'static str,
    pub state_charging: &'static str,
    pub state_plugged: &'static str,
    pub state_on_battery: &'static str,
    pub cycles: &'static str,
    pub per_month: &'static str,
    pub capacity: &'static str,
    pub on_battery: &'static str,
    pub last_n: &'static str,
    pub charger: &'static str,
    pub used: &'static str,
    pub mem_apps: &'static str,
    pub mem_wired: &'static str,
    pub mem_compressed: &'static str,
    pub mem_cached: &'static str,
    pub free: &'static str,
    pub swap: &'static str,
    pub largest: &'static str,
    pub files: &'static str,
    pub read_since_boot: &'static str,
    pub written_since_boot: &'static str,
    pub down: &'static str,
    pub up: &'static str,
    pub top_traffic: &'static str,
    pub known_wifi: &'static str,
    pub bluetooth: &'static str,
    pub bluetooth_note: &'static str,
    pub screen_today: &'static str,
    pub screen_avg: &'static str,
    pub by_hour: &'static str,
    pub top_apps: &'static str,
    pub switches_per_day: &'static str,
    pub notifications_per_day: &'static str,
    pub full_disk_access_hint: &'static str,
    pub apps: &'static str,
    pub apps_note: &'static str,
    pub unopened: &'static str,
    pub homebrew: &'static str,
    pub homebrew_value: &'static str,
    pub users: &'static str,
    pub processes: &'static str,
    pub crashes: &'static str,
    pub crashes_note: &'static str,
    pub panics: &'static str,
    pub shell_commands: &'static str,
    pub top_commands: &'static str,
    pub cli_about: &'static str,
    pub cli_usage: &'static str,
    pub cli_options: &'static str,
    pub cli_json: &'static str,
    pub cli_plain: &'static str,
    pub cli_demo: &'static str,
    pub cli_lang: &'static str,
    pub cli_help: &'static str,
    pub cli_version: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn no_env(_: &str) -> Option<String> {
        None
    }

    #[test]
    fn tags_are_parsed_loosely() {
        assert_eq!(Lang::parse("ru"), Some(Lang::Ru));
        assert_eq!(Lang::parse("pt-BR"), Some(Lang::Pt));
        assert_eq!(Lang::parse("zh_CN.UTF-8"), Some(Lang::Zh));
        assert_eq!(Lang::parse("DE"), Some(Lang::De));
        assert_eq!(Lang::parse("C"), None);
        assert_eq!(Lang::parse("POSIX"), None);
        assert_eq!(Lang::parse(""), None);
    }

    #[test]
    fn environment_wins_in_order() {
        let env = |key: &str| match key {
            "LC_ALL" => Some("C".to_string()),
            "LANG" => Some("fr_FR.UTF-8".to_string()),
            _ => None,
        };
        assert_eq!(
            Lang::detect(env, || panic!("macOS need not be asked")),
            Lang::Fr
        );
        let forced = |key: &str| (key == "PROBEG_LANG").then(|| "ja".to_string());
        assert_eq!(Lang::detect(forced, Vec::new), Lang::Ja);
    }

    #[test]
    fn system_list_uses_first_supported_language() {
        let list = || vec!["kk-KZ".to_string(), "ru-KZ".to_string(), "en".to_string()];
        assert_eq!(Lang::detect(no_env, list), Lang::Ru);
        assert_eq!(Lang::detect(no_env, Vec::new), Lang::En);
    }

    #[test]
    fn defaults_output_is_parsed() {
        let text = "(\n    \"en-KZ\",\n    \"ru-KZ\",\n    \"zh-Hans-KZ\"\n)\n";
        assert_eq!(parse_language_list(text), ["en-KZ", "ru-KZ", "zh-Hans-KZ"]);
    }

    #[test]
    fn fill_allows_reordering() {
        assert_eq!(fill("{1} / {0}", &["a", "b"]), "b / a");
    }

    fn placeholders(text: &str) -> Vec<String> {
        let mut found: Vec<String> = text
            .split('{')
            .skip(1)
            .filter_map(|part| part.split_once('}').map(|(name, _)| name.to_string()))
            .collect();
        found.sort();
        found
    }

    fn strings(lang: Lang) -> serde_json::Map<String, Value> {
        match serde_json::to_value(lang.locale()).unwrap() {
            Value::Object(map) => map,
            _ => unreachable!(),
        }
    }

    #[test]
    fn translations_keep_every_placeholder() {
        let english = strings(Lang::En);
        for lang in Lang::ALL {
            for (key, value) in strings(lang) {
                let (Some(theirs), Some(ours)) = (value.as_str(), english[&key].as_str()) else {
                    continue;
                };
                assert_eq!(
                    placeholders(theirs),
                    placeholders(ours),
                    "{lang:?}.{key}: {theirs}"
                );
            }
        }
    }

    #[test]
    fn no_string_is_left_empty() {
        for lang in Lang::ALL {
            for (key, value) in strings(lang) {
                // A space is a real thousands separator, in French and Russian.
                if let Some(text) = value.as_str().filter(|_| key != "group_separator") {
                    assert!(!text.trim().is_empty(), "{lang:?}.{key}");
                }
            }
        }
    }

    #[test]
    fn non_english_locales_are_actually_translated() {
        // Technical words legitimately match; a large overlap means forgotten copy-paste.
        let english = strings(Lang::En);
        for lang in Lang::ALL.into_iter().filter(|l| *l != Lang::En) {
            let same = strings(lang)
                .iter()
                .filter(|(key, _)| key.starts_with("cli_") || key.starts_with("section_"))
                .filter(|(key, value)| value.as_str().is_some() && english[*key] == **value)
                .count();
            assert!(same <= 3, "{lang:?} repeats {same} English strings");
        }
    }

    #[test]
    fn codes_are_unique() {
        let mut codes: Vec<&str> = Lang::ALL.iter().map(|l| l.locale().code).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), Lang::ALL.len());
    }
}
