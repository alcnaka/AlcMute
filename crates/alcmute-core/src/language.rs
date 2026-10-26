//! UI language selection. OSC, shortcut identities and stored values are never translated.
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    OnceLock,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    #[default]
    Auto,
    Japanese,
    English,
}
impl Language {
    pub fn is_auto(&self) -> bool {
        *self == Self::Auto
    }
    pub fn index(self) -> i32 {
        match self {
            Self::Auto => 0,
            Self::Japanese => 1,
            Self::English => 2,
        }
    }
    pub fn from_index(index: i32) -> Result<Self, String> {
        match index {
            0 => Ok(Self::Auto),
            1 => Ok(Self::Japanese),
            2 => Ok(Self::English),
            _ => Err("Invalid language selection".into()),
        }
    }
    pub fn english(self) -> bool {
        match self {
            Self::English => true,
            Self::Japanese => false,
            Self::Auto => {
                #[cfg(feature = "windows-integration")]
                {
                    unsafe {
                        windows_sys::Win32::Globalization::GetUserDefaultUILanguage() & 0x3ff
                            != 0x11
                    }
                }
                #[cfg(not(feature = "windows-integration"))]
                {
                    false
                }
            }
        }
    }
}
static ENGLISH: AtomicBool = AtomicBool::new(false);
pub fn set(language: Language) {
    ENGLISH.store(language.english(), Ordering::Relaxed);
}
pub fn english() -> bool {
    ENGLISH.load(Ordering::Relaxed)
}
/// Read only the display preference, without migrating or modifying settings.
pub fn load(directory: &std::path::Path) -> Language {
    #[derive(Default, Deserialize)]
    #[serde(default)]
    struct Selection {
        language: Language,
    }
    std::fs::read_to_string(directory.join("settings.toml"))
        .ok()
        .and_then(|text| toml::from_str::<Selection>(&text).ok())
        .unwrap_or_default()
        .language
}
pub fn text(value: &str) -> String {
    if english() {
        translate_en(value)
    } else {
        value.into()
    }
}

fn catalog() -> &'static std::collections::BTreeMap<String, String> {
    static CATALOG: OnceLock<std::collections::BTreeMap<String, String>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("../../../assets/i18n/en.json"))
            .expect("checked translation catalog")
    })
}

/// Translate exact messages and format templates without altering captured paths/keys.
pub fn translate_en(value: &str) -> String {
    if let Some(translated) = catalog().get(value) {
        return translated.clone();
    }
    for (source, translated) in catalog() {
        if !source.contains('{') {
            continue;
        }
        let mut remaining = value;
        let mut template = source.as_str();
        let mut slots = Vec::new();
        let mut matches = true;
        while let Some(open) = template.find('{') {
            let prefix = &template[..open];
            let Some(rest) = remaining.strip_prefix(prefix) else {
                matches = false;
                break;
            };
            let Some(close) = template[open..].find('}').map(|i| i + open) else {
                matches = false;
                break;
            };
            let slot = &template[open..=close];
            template = &template[close + 1..];
            let delimiter = template.split('{').next().unwrap();
            let end = if delimiter.is_empty() {
                rest.len()
            } else if let Some(end) = rest.find(delimiter) {
                end
            } else {
                matches = false;
                break;
            };
            slots.push((slot, &rest[..end]));
            remaining = &rest[end..];
        }
        if matches && remaining == template {
            let mut output = translated.clone();
            for (slot, captured) in slots {
                output = output.replacen(slot, &translate_en(captured), 1);
            }
            return output;
        }
    }
    // Status labels can include an unmodified diagnostic detail in parentheses.
    if let Some((label, detail)) = value.split_once(" (") {
        if let Some(label) = catalog().get(label) {
            return format!("{label} ({detail}");
        }
    }
    if value.contains('\n') {
        return value
            .split('\n')
            .map(translate_en)
            .collect::<Vec<_>>()
            .join("\n");
    }
    value.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_preserves_format_slots_and_translates_runtime_messages() {
        fn slots(text: &str) -> Vec<&str> {
            let mut result = Vec::new();
            let mut tail = text;
            while let Some(start) = tail.find('{') {
                tail = &tail[start..];
                let end = tail.find('}').expect("balanced template");
                result.push(&tail[..=end]);
                tail = &tail[end + 1..];
            }
            result.sort();
            result
        }
        for (source, english) in catalog() {
            assert!(!english.is_empty(), "Missing translation: {source}");
            assert_eq!(slots(source), slots(english), "Template slots: {source}");
        }
        assert_eq!(
            translate_en("保存しました\n閉じる"),
            "Settings saved\nClose"
        );
        assert_eq!(translate_en("RightCtrl+LeftAlt"), "RightCtrl+LeftAlt");
    }
    #[test]
    fn language_roundtrip_defaults_and_templates_preserve_values() {
        let old: crate::Config = toml::from_str("shortcut = 'RightCtrl+LeftAlt'").unwrap();
        assert_eq!(old.language, Language::Auto);
        for language in [Language::Auto, Language::Japanese, Language::English] {
            let config = crate::Config {
                language,
                ..old.clone()
            };
            assert_eq!(
                toml::from_str::<crate::Config>(&toml::to_string(&config).unwrap()).unwrap(),
                config
            );
            assert_eq!(Language::from_index(language.index()).unwrap(), language);
        }
        assert!(toml::from_str::<crate::Config>("language = 'invalid'").is_err());
        assert_eq!(
            translate_en("新しいバージョン v9.8.7 があります。"),
            "Version v9.8.7 is available."
        );
        assert_eq!(
            translate_en("ログ：C:\\Users\\User\\error.log"),
            "Log: C:\\Users\\User\\error.log"
        );
    }
}
