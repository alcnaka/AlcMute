//! Only the installed build may mutate startup. Tests are confined to a private key.
use super::quote_arg;
use crate::{Options, Result};
use std::path::{Path, PathBuf};
use winreg::{
    enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE},
    RegKey,
};

const NAME: &str = "AlcMute";
pub(super) const LEGACY_STARTUP_NAME: &str = "VRChatMuteController";
const INSTALL_KEY: &str = r"Software\Alcnaka\AlcMute";
pub(super) fn startup_path(options: &Options) -> String {
    if options.benchmark {
        format!(r"Software\AlcMute\Testing\{}\Run", std::process::id())
    } else {
        r"Software\Microsoft\Windows\CurrentVersion\Run".into()
    }
}
fn approved_path(options: &Options) -> String {
    if options.benchmark {
        format!(
            r"Software\AlcMute\Testing\{}\StartupApproved",
            std::process::id()
        )
    } else {
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run".into()
    }
}
fn test_registration(options: &Options) -> bool {
    cfg!(test) && options.benchmark
}
pub fn installation_directory() -> Result<PathBuf> {
    let directory = registered_directory()?;
    let exe = std::fs::canonicalize(directory.join("alcmute.exe")).map_err(|e| e.to_string())?;
    let current = std::fs::canonicalize(std::env::current_exe().map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    if exe != current {
        return Err("管理対象の実行パスではありません".into());
    }
    Ok(directory)
}
pub fn registered_directory() -> Result<PathBuf> {
    if !cfg!(feature = "installer") {
        return Err("インストーラー版でのみ利用できます。".into());
    }
    let key = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(INSTALL_KEY)
        .map_err(|e| e.to_string())?;
    let path: String = key.get_value("InstallDir").map_err(|e| e.to_string())?;
    let directory = std::fs::canonicalize(path).map_err(|e| e.to_string())?;
    let exe = std::fs::canonicalize(directory.join("alcmute.exe")).map_err(|e| e.to_string())?;
    let local =
        std::fs::canonicalize(std::env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA is missing")?)
            .map_err(|e| e.to_string())?;
    if !directory.starts_with(&local) || !exe.starts_with(local) || !product_identity(&exe) {
        return Err("管理対象のインストール先から起動してください".into());
    }
    Ok(directory)
}
pub fn installed_version() -> Result<String> {
    let key=RegKey::predef(HKEY_CURRENT_USER).open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{D7267BA5-A7BD-46DB-A39F-47E9F7843CF6}_is1").map_err(|e|e.to_string())?;
    key.get_value("DisplayVersion").map_err(|e| e.to_string())
}
pub fn startup_available(options: &Options) -> bool {
    test_registration(options) || installation_directory().is_ok()
}
fn require_managed(options: &Options) -> Result<()> {
    if startup_available(options) {
        Ok(())
    } else {
        Err("インストーラー版でのみ利用できます。管理対象の実行パスも必要です。".into())
    }
}
pub fn startup_snapshot(options: &Options) -> Result<Option<String>> {
    let key = match RegKey::predef(HKEY_CURRENT_USER).open_subkey(startup_path(options)) {
        Ok(key) => key,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    match key.get_value(NAME) {
        Ok(value) => Ok(Some(value)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}
pub(super) fn startup_command(exe: &Path, directory: &Path) -> Result<String> {
    let command = format!(
        "{} --background --data-dir {}",
        quote_arg(exe.to_str().ok_or("Invalid executable path")?),
        quote_arg(directory.to_str().ok_or("Invalid settings path")?)
    );
    if command.encode_utf16().count() > 260 {
        return Err("Startup command exceeds 260 characters".into());
    }
    Ok(command)
}
fn expected_command(options: &Options) -> Result<String> {
    startup_command(
        &std::env::current_exe().map_err(|e| e.to_string())?,
        &std::path::absolute(&options.data_dir).map_err(|e| e.to_string())?,
    )
}
fn approval(options: &Options, name: &str) -> Result<Option<winreg::RegValue>> {
    let key = match RegKey::predef(HKEY_CURRENT_USER).open_subkey(approved_path(options)) {
        Ok(key) => key,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    match key.get_raw_value(name) {
        Ok(value) => Ok(Some(value)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}
fn disabled(value: Option<&winreg::RegValue>) -> bool {
    // Only the known enabled encoding is considered enabled. Preserve unknown states.
    value.is_some_and(|value| value.bytes.first() != Some(&2))
}
pub fn startup_enabled(options: &Options) -> Result<bool> {
    if !startup_available(options) {
        return Ok(false);
    }
    Ok(startup_snapshot(options)?
        .is_some_and(|saved| expected_command(options).is_ok_and(|expected| expected == saved))
        && !disabled(approval(options, NAME)?.as_ref()))
}
pub fn restore_startup(options: &Options, command: Option<&str>) -> Result<()> {
    require_managed(options)?;
    if command.is_none() && startup_snapshot(options)?.is_none() {
        return Ok(());
    }
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER)
        .create_subkey(startup_path(options))
        .map_err(|e| e.to_string())?;
    match command {
        Some(value) => key.set_value(NAME, &value).map_err(|e| e.to_string()),
        None => match key.delete_value(NAME) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        },
    }
}
pub fn set_startup(options: &Options, enabled: bool) -> Result<()> {
    require_managed(options)?;
    let previous = startup_snapshot(options)?;
    // Never remove a different product's/portable executable's registration.
    if previous.as_deref().is_some_and(|command| {
        expected_command(options).as_deref() != Ok(command) && !owned_command(command)
    }) {
        return Err("既存のAlcMute登録を安全に識別できません。変更しませんでした。".into());
    }
    if enabled && disabled(approval(options, NAME)?.as_ref()) {
        return Err("Windowsのスタートアップ設定で無効化されています。Windowsの設定から有効にしてください。".into());
    }
    let desired = if enabled {
        Some(expected_command(options)?)
    } else {
        None
    };
    if previous == desired {
        return Ok(());
    }
    restore_startup(options, desired.as_deref())
}
fn arguments(command: &str) -> Option<Vec<String>> {
    use windows_sys::Win32::{Foundation::LocalFree, UI::Shell::CommandLineToArgvW};
    let wide: Vec<u16> = command.encode_utf16().chain(Some(0)).collect();
    let mut count = 0;
    let argv = unsafe { CommandLineToArgvW(wide.as_ptr(), &mut count) };
    if argv.is_null() {
        return None;
    }
    let mut args = Vec::new();
    for index in 0..count as usize {
        let ptr = unsafe { *argv.add(index) };
        let mut len = 0;
        while unsafe { *ptr.add(len) } != 0 {
            len += 1;
        }
        args.push(String::from_utf16_lossy(unsafe {
            std::slice::from_raw_parts(ptr, len)
        }));
    }
    unsafe {
        LocalFree(argv.cast());
    }
    Some(args)
}
fn data_directory(args: &[String]) -> Option<PathBuf> {
    args.windows(2)
        .find(|pair| pair[0] == "--data-dir")
        .map(|pair| PathBuf::from(&pair[1]))
}
fn product_identity(path: &Path) -> bool {
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW,
    };
    let wide: Vec<u16> = path
        .as_os_str()
        .to_string_lossy()
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let size = unsafe { GetFileVersionInfoSizeW(wide.as_ptr(), std::ptr::null_mut()) };
    if size == 0 || size > 1024 * 1024 {
        return false;
    }
    let mut data = vec![0u32; (size as usize).div_ceil(4)];
    if unsafe { GetFileVersionInfoW(wide.as_ptr(), 0, size, data.as_mut_ptr().cast()) } == 0 {
        return false;
    }
    // winresource's fixed language/codepage. Unknown resources are conservatively skipped.
    let query = |field: &str| -> Option<String> {
        let name: Vec<u16> = format!("\\StringFileInfo\\040904b0\\{field}")
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let mut ptr = std::ptr::null_mut();
        let mut len = 0;
        if unsafe { VerQueryValueW(data.as_ptr().cast(), name.as_ptr(), &mut ptr, &mut len) } == 0
            || len == 0
        {
            return None;
        }
        Some(String::from_utf16_lossy(unsafe {
            std::slice::from_raw_parts(ptr.cast::<u16>(), len as usize - 1)
        }))
    };
    query("CompanyName").as_deref() == Some("Alcnaka")
        && query("ProductName").as_deref() == Some("AlcMute")
}
fn owned_command(command: &str) -> bool {
    let Some(args) = arguments(command) else {
        return false;
    };
    let Some(exe) = args.first().map(Path::new) else {
        return false;
    };
    let name = exe
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_lowercase();
    if !exe.is_absolute()
        || !matches!(
            name.as_str(),
            "alcmute.exe" | "poc-slint.exe" | "poc-slint-tray.exe"
        )
    {
        return false;
    }
    if product_identity(exe) {
        return true;
    }
    let data = data_directory(&args);
    let known_data = data.as_ref().is_some_and(|dir| {
        *dir == crate::migration::default_directory()
            || *dir == crate::migration::legacy_directory()
    });
    let known_poc = exe
        .to_string_lossy()
        .to_ascii_lowercase()
        .contains("\\vrchat-ui-poc\\slint-tray\\");
    // Legacy binaries without VERSIONINFO require both a known settings argument and PoC location.
    known_data && known_poc
}
pub fn migrate_startup(options: &Options) -> Result<()> {
    require_managed(options)?;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = match hkcu.open_subkey_with_flags(startup_path(options), KEY_READ | KEY_WRITE) {
        Ok(key) => key,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.to_string()),
    };
    let entries: Vec<_> = key
        .enum_values()
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;
    let mut recognized = Vec::new();
    for (name, value) in entries {
        let Ok(command) = key.get_value::<String, _>(&name) else {
            continue;
        };
        if owned_command(&command) {
            recognized.push((name, value, command));
        } else if name == NAME || name == LEGACY_STARTUP_NAME {
            options.log(&format!(
                "Unidentified startup entry retained: {name}: {command}"
            ));
        }
    }
    if recognized.is_empty() {
        return Ok(());
    }
    if startup_snapshot(options)?.is_some() && !recognized.iter().any(|(name, _, _)| name == NAME) {
        return Err(
            "AlcMute startup name belongs to an unidentified entry; migration cancelled".into(),
        );
    }
    let selected = recognized
        .iter()
        .find(|(name, _, _)| name == NAME)
        .unwrap_or(&recognized[0]);
    let args = arguments(&selected.2).ok_or("Invalid startup command")?;
    let dir = data_directory(&args).unwrap_or_else(|| options.data_dir.clone());
    let command = startup_command(&std::env::current_exe().map_err(|e| e.to_string())?, &dir)?;
    let approvals: Vec<_> = recognized
        .iter()
        .map(|(name, _, _)| approval(options, name).map(|value| (name.clone(), value)))
        .collect::<Result<Vec<_>>>()?;
    let preserve_disabled = approvals
        .iter()
        .find_map(|(_, value)| value.as_ref().filter(|value| disabled(Some(value))));
    // The entire original registry data is durable before the first mutation.
    std::fs::create_dir_all(&options.data_dir).map_err(|e| e.to_string())?;
    let journal = options.data_dir.join("startup-migration-backup.txt");
    let snapshot = recognized
        .iter()
        .map(|(name, _, command)| format!("{name} = {command}\n"))
        .collect::<String>();
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&journal)
        .or_else(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                std::fs::OpenOptions::new().append(true).open(&journal)
            } else {
                Err(e)
            }
        })
        .map_err(|e| e.to_string())?;
    use std::io::Write;
    file.write_all(snapshot.as_bytes())
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    let previous = startup_snapshot(options)?;
    let previous_approval = approval(options, NAME)?;
    let (approved, _) = hkcu
        .create_subkey(approved_path(options))
        .map_err(|e| e.to_string())?;
    let transaction = (|| -> Result<()> {
        key.set_value(NAME, &command).map_err(|e| e.to_string())?;
        if let Some(value) = preserve_disabled {
            approved
                .set_raw_value(NAME, value)
                .map_err(|e| e.to_string())?;
        }
        if key
            .get_value::<String, _>(NAME)
            .map_err(|e| e.to_string())?
            != command
        {
            return Err("Startup verification failed".into());
        }
        for (name, _, _) in &recognized {
            if name != NAME {
                key.delete_value(name).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    })();
    if let Err(error) = transaction {
        let mut recovery = Vec::new();
        for (name, value, _) in &recognized {
            if let Err(e) = key.set_raw_value(name, value) {
                recovery.push(e.to_string());
            }
        }
        if previous.is_none() {
            if let Err(e) = key.delete_value(NAME) {
                recovery.push(e.to_string());
            }
        }
        let restored = if let Some(value) = previous_approval {
            approved.set_raw_value(NAME, &value)
        } else {
            approved.delete_value(NAME).or_else(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Ok(())
                } else {
                    Err(e)
                }
            })
        };
        if let Err(e) = restored {
            recovery.push(e.to_string());
        }
        return Err(format!("{error}; recovery: {}", recovery.join("; ")));
    }
    Ok(())
}
pub fn uninstall_cleanup(options: &Options) -> Result<()> {
    require_managed(options)?;
    if let Some(command) = startup_snapshot(options)? {
        if arguments(&command)
            .and_then(|args| args.first().cloned())
            .is_some_and(|exe| {
                std::fs::canonicalize(exe).ok()
                    == std::env::current_exe()
                        .ok()
                        .and_then(|p| std::fs::canonicalize(p).ok())
            })
        {
            restore_startup(options, None)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unmanaged_binary_cannot_mutate_startup_even_when_requested() {
        let options = Options::default();
        assert!(!startup_available(&options));
        assert!(!startup_enabled(&options).unwrap());
        assert!(set_startup(&options, true).is_err());
        assert!(set_startup(&options, false).is_err());
        assert!(restore_startup(&options, None).is_err());
        assert!(migrate_startup(&options).is_err());
        assert!(uninstall_cleanup(&options).is_err());
    }
    #[test]
    fn legacy_identity_needs_more_than_a_filename() {
        assert!(!owned_command(r#""C:\Unrelated\alcmute.exe""#));
        assert!(!owned_command(
            r#""C:\Unrelated\poc-slint.exe" --data-dir "C:\Unrelated""#
        ));
        assert!(!owned_command("old application command"));
        let command = format!(
            "{} --data-dir {}",
            quote_arg(r"C:\Work\vrchat-ui-poc\slint-tray\target\release\poc-slint.exe"),
            quote_arg(crate::migration::default_directory().to_str().unwrap())
        );
        assert!(owned_command(&command));
    }
    #[test]
    fn private_registry_migration_preserves_disabled_entries_and_is_idempotent() {
        let options = Options {
            data_dir: std::env::temp_dir()
                .join(format!("alcmute-startup-migrate-{}", std::process::id())),
            benchmark: true,
        };
        let (key, _) = RegKey::predef(HKEY_CURRENT_USER)
            .create_subkey(startup_path(&options))
            .unwrap();
        let previous: Vec<_> = key.enum_values().collect::<std::io::Result<_>>().unwrap();
        for (name, _) in &previous {
            key.delete_value(name).unwrap();
        }
        let (approved, _) = RegKey::predef(HKEY_CURRENT_USER)
            .create_subkey(approved_path(&options))
            .unwrap();
        let old_approval = approved.get_raw_value(NAME).ok();
        let command = format!(
            "{} --data-dir {}",
            quote_arg(r"C:\Work\vrchat-ui-poc\slint-tray\target\release\poc-slint.exe"),
            quote_arg(crate::migration::default_directory().to_str().unwrap())
        );
        key.set_value(LEGACY_STARTUP_NAME, &command).unwrap();
        key.set_value("unrelated", &r#""C:\Unrelated\alcmute.exe""#)
            .unwrap();
        approved
            .set_raw_value(
                LEGACY_STARTUP_NAME,
                &winreg::RegValue {
                    vtype: winreg::enums::REG_BINARY,
                    bytes: vec![3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                },
            )
            .unwrap();
        migrate_startup(&options).unwrap();
        assert!(key.get_value::<String, _>(LEGACY_STARTUP_NAME).is_err());
        let migrated = key.get_value::<String, _>(NAME).unwrap();
        assert!(migrated.contains("--background"));
        assert!(migrated.contains(crate::migration::default_directory().to_str().unwrap()));
        assert!(!startup_enabled(&options).unwrap());
        assert!(set_startup(&options, true).is_err());
        assert!(key.get_value::<String, _>("unrelated").is_ok());
        // Already migrated command points to this test binary. No legacy registration is recreated.
        migrate_startup(&options).unwrap();
        assert_eq!(
            startup_snapshot(&options).unwrap().as_deref(),
            Some(migrated.as_str())
        );
        approved.delete_value(NAME).unwrap();
        approved.delete_value(LEGACY_STARTUP_NAME).unwrap();
        restore_startup(&options, None).unwrap();
        set_startup(&options, true).unwrap();
        assert!(startup_enabled(&options).unwrap());
        uninstall_cleanup(&options).unwrap();
        assert!(startup_snapshot(&options).unwrap().is_none());
        for entry in key
            .enum_values()
            .collect::<std::io::Result<Vec<_>>>()
            .unwrap()
        {
            key.delete_value(entry.0).unwrap();
        }
        for (name, value) in previous {
            key.set_raw_value(name, &value).unwrap();
        }
        if let Some(value) = old_approval {
            approved.set_raw_value(NAME, &value).unwrap();
        }
        std::fs::remove_dir_all(options.data_dir).unwrap();
    }
}
