use crate::{Config, Options, Result};
use std::{fs, io::Write, path::Path};
pub(crate) fn write_pending(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = fs::File::create(path).map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())
}
pub fn replace(source: &Path, destination: &Path) -> Result<()> {
    #[cfg(feature = "windows-integration")]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{
            MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
        };
        let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
        let destination: Vec<u16> = destination
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        if unsafe {
            MoveFileExW(
                source.as_ptr(),
                destination.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(())
    }
    #[cfg(not(feature = "windows-integration"))]
    fs::rename(source, destination).map_err(|e| e.to_string())
}
pub fn validate(text: &str) -> Result<Config> {
    let config: Config = toml::from_str(text).map_err(|e| e.to_string())?;
    config.validate()?;
    Ok(config)
}
pub(crate) fn backup(path: &Path) -> Result<()> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    validate(&text)?;
    let pending = path.with_file_name("settings.backup.pending.toml");
    let mut file = fs::File::create(&pending).map_err(|e| e.to_string())?;
    file.write_all(text.as_bytes())
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    drop(file);
    replace(&pending, &path.with_file_name("settings.toml.bak"))
}
pub fn recovery_available(options: &Options) -> bool {
    let path = options.data_dir.join("settings.toml");
    fs::read_to_string(&path).is_ok_and(|text| validate(&text).is_err())
        && fs::read_to_string(path.with_file_name("settings.toml.bak"))
            .is_ok_and(|text| validate(&text).is_ok())
}
pub fn recover(options: &Options) -> Result<std::path::PathBuf> {
    if !recovery_available(options) {
        return Err("正常なバックアップがありません。設定は変更していません。".into());
    }
    let path = options.data_dir.join("settings.toml");
    let text =
        fs::read_to_string(path.with_file_name("settings.toml.bak")).map_err(|e| e.to_string())?;
    validate(&text)?;
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let preserved = options
        .data_dir
        .join(format!("settings.corrupt.{timestamp}.toml"));
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&preserved)
        .map_err(|e| e.to_string())?;
    file.write_all(&fs::read(&path).map_err(|e| e.to_string())?)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    drop(file);
    let pending = options.data_dir.join("settings.recovery.pending.toml");
    let mut file = fs::File::create(&pending).map_err(|e| e.to_string())?;
    file.write_all(text.as_bytes())
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    drop(file);
    replace(&pending, &path)?;
    Ok(preserved)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn backup_recovery_preserves_corruption_and_rejects_bad_backup() {
        let dir = std::env::temp_dir().join(format!("alcmute-backup-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let options = Options {
            data_dir: dir.clone(),
            benchmark: true,
        };
        let path = dir.join("settings.toml");
        let original = "shortcut = 'RightCtrl+LeftAlt'\n";
        fs::write(&path, original).unwrap();
        backup(&path).unwrap();
        fs::write(&path, "invalid = [").unwrap();
        assert!(recovery_available(&options));
        let preserved = recover(&options).unwrap();
        assert_eq!(fs::read_to_string(&preserved).unwrap(), "invalid = [");
        assert_eq!(fs::read_to_string(&path).unwrap(), original);
        fs::write(&path, "bad[").unwrap();
        fs::write(dir.join("settings.toml.bak"), "bad[").unwrap();
        assert!(!recovery_available(&options));
        assert!(recover(&options).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "bad[");
        fs::remove_dir_all(dir).unwrap();
    }
}
