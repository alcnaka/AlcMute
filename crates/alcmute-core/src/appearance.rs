//! Theme values persisted as part of settings.toml.
use crate::Result;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    #[default]
    Auto,
    Light,
    Dark,
}

impl Theme {
    pub fn index(self) -> i32 {
        match self {
            Self::Auto => 0,
            Self::Light => 1,
            Self::Dark => 2,
        }
    }
    pub fn from_index(index: i32) -> Result<Self> {
        match index {
            0 => Ok(Self::Auto),
            1 => Ok(Self::Light),
            2 => Ok(Self::Dark),
            _ => Err("テーマは自動・ライト・ダークから選択してください".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Config, Options};
    use std::fs;

    struct Fixture(Options);
    impl Fixture {
        fn new(name: &str) -> Self {
            let data_dir =
                std::env::temp_dir().join(format!("alcmute-unified-{name}-{}", std::process::id()));
            fs::create_dir_all(&data_dir).unwrap();
            Self(Options {
                data_dir,
                benchmark: true,
            })
        }
        fn write(&self, name: &str, text: &str) {
            fs::write(self.0.data_dir.join(name), text).unwrap();
        }
        fn read(&self, name: &str) -> String {
            fs::read_to_string(self.0.data_dir.join(name)).unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0.data_dir).unwrap();
        }
    }

    #[test]
    fn unified_settings_roundtrip_and_invalid_values_are_preserved() {
        let f = Fixture::new("roundtrip");
        assert_eq!(Config::load(&f.0).unwrap(), Config::default());
        for theme in [Theme::Auto, Theme::Light, Theme::Dark] {
            assert_eq!(Theme::from_index(theme.index()).unwrap(), theme);
            let config = Config {
                theme,
                auto_check_updates: false,
                ..Default::default()
            };
            f.write("settings.toml", &toml::to_string(&config).unwrap());
            assert_eq!(Config::load(&f.0).unwrap(), config);
        }
        assert!(Theme::from_index(-1).is_err());
        assert!(Theme::from_index(3).is_err());
        for invalid in ["theme = 'invalid'", "auto_check_updates = 'false'"] {
            f.write("settings.toml", invalid);
            assert!(Config::load(&f.0).is_err());
            assert_eq!(f.read("settings.toml"), invalid);
        }
        assert!(!f.0.data_dir.join("appearance.toml").exists());
    }

    #[test]
    fn legacy_updates_are_merged_archived_and_not_reimported_after_reset() {
        let f = Fixture::new("migration");
        let original = "shortcut = 'KeyK+KeyL'\n";
        f.write("settings.toml", original);
        f.write("updates.toml", "auto_check = false\n");
        let config = Config::load(&f.0).unwrap();
        assert!(!config.auto_check_updates);
        assert_eq!(config.shortcut, "KeyK+KeyL");
        assert_eq!(f.read("settings.toml.bak"), original);
        assert_eq!(f.read("updates.toml.bak"), "auto_check = false\n");
        assert!(!f.0.data_dir.join("updates.toml").exists());
        assert_eq!(Config::load(&f.0).unwrap(), config);
        fs::remove_file(f.0.data_dir.join("settings.toml")).unwrap();
        assert_eq!(Config::load(&f.0).unwrap(), Config::default());
    }

    #[test]
    fn explicit_unified_settings_take_precedence_and_keep_existing_archive() {
        let f = Fixture::new("precedence");
        f.write(
            "settings.toml",
            "auto_check_updates = true\ntheme = 'light'\n",
        );
        f.write("updates.toml", "auto_check = false\n");
        f.write("updates.toml.bak", "existing archive");
        let config = Config::load(&f.0).unwrap();
        assert!(config.auto_check_updates);
        assert_eq!(config.theme, Theme::Light);
        assert_eq!(f.read("updates.toml.bak"), "existing archive");
        assert!(!f.0.data_dir.join("updates.toml").exists());
        assert_eq!(Config::load(&f.0).unwrap(), config);
    }

    #[test]
    fn failed_migration_preserves_both_original_files() {
        let f = Fixture::new("failure");
        let original = "theme = 'dark'\n";
        f.write("settings.toml", original);
        f.write("updates.toml", "auto_check = 'broken'\n");
        assert!(Config::load(&f.0).is_err());
        assert_eq!(f.read("settings.toml"), original);
        assert_eq!(f.read("updates.toml"), "auto_check = 'broken'\n");
        f.write("updates.toml", "auto_check = false\n");
        fs::create_dir(f.0.data_dir.join("settings.pending.toml")).unwrap();
        assert!(Config::load(&f.0).is_err());
        assert_eq!(f.read("settings.toml"), original);
        assert_eq!(f.read("updates.toml"), "auto_check = false\n");
    }
}
