use crate::{Config, Result};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub fn default_directory() -> PathBuf {
    local_directory().join("Alcnaka").join("AlcMute")
}

pub fn legacy_directory() -> PathBuf {
    local_directory().join("VRChatMuteController")
}

fn local_directory() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

pub fn settings(old: &Path, new: &Path) -> Result<()> {
    let marker = new.join("legacy-settings-migrated");
    let source = old.join("settings.toml");
    if marker.exists() || !source.exists() {
        return Ok(());
    }
    fs::create_dir_all(new).map_err(|e| e.to_string())?;
    let destination = new.join("settings.toml");
    if !destination.exists() {
        let text =
            fs::read_to_string(&source).map_err(|e| format!("Cannot read legacy settings: {e}"))?;
        let config: Config =
            toml::from_str(&text).map_err(|e| format!("Invalid legacy settings: {e}"))?;
        config.validate()?;
        // Never overwrite new preferences, including if another process created them.
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination)
        {
            Ok(mut file) => {
                if let Err(error) = file
                    .write_all(text.as_bytes())
                    .and_then(|_| file.sync_all())
                {
                    drop(file);
                    let _ = fs::remove_file(&destination);
                    return Err(format!("Cannot migrate legacy settings: {error}"));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(format!("Cannot create migrated settings: {error}")),
        }
    }
    // Keep the old file as a backup, but never import it again after a reset.
    fs::write(marker, "Legacy settings migration completed.\n").map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_preserves_preferences_and_reset_does_not_reimport_backup() {
        let root = std::env::temp_dir().join(format!("alcmute-migration-{}", std::process::id()));
        let old = root.join("old");
        let new = root.join("new");
        fs::create_dir_all(&old).unwrap();
        let original = "shortcut = 'RightCtrl+LeftAlt'\ntray_double_click_settings = true\n";
        fs::write(old.join("settings.toml"), original).unwrap();
        settings(&old, &new).unwrap();
        assert_eq!(
            fs::read_to_string(new.join("settings.toml")).unwrap(),
            original
        );
        assert_eq!(
            fs::read_to_string(old.join("settings.toml")).unwrap(),
            original
        );
        fs::remove_file(new.join("settings.toml")).unwrap();
        settings(&old, &new).unwrap();
        assert!(!new.join("settings.toml").exists());
        fs::remove_file(new.join("legacy-settings-migrated")).unwrap();
        fs::write(new.join("settings.toml"), "shortcut = 'KeyK+KeyL'").unwrap();
        settings(&old, &new).unwrap();
        assert_eq!(
            fs::read_to_string(new.join("settings.toml")).unwrap(),
            "shortcut = 'KeyK+KeyL'"
        );
        for directory in [&old, &new] {
            for file in fs::read_dir(directory).unwrap() {
                fs::remove_file(file.unwrap().path()).unwrap();
            }
            fs::remove_dir(directory).unwrap();
        }
        fs::remove_dir(root).unwrap();
    }
}
