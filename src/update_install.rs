//! The copied helper owns no handle to the installed executable while Setup runs.
use alcmute_core::{updater, windows, Options};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
pub fn directory() -> Result<PathBuf, String> {
    let local = std::env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA missing")?;
    let local = fs::canonicalize(PathBuf::from(local)).map_err(|e| e.to_string())?;
    let root = local.join("Alcnaka/AlcMute/updates");
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    if !root.starts_with(local) {
        return Err("Update directory leaves LocalAppData".into());
    }
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let dir = root.join(format!("{}-{nonce}", std::process::id()));
    windows::create_private_directory(&dir)?;
    Ok(dir)
}
pub fn launch(setup: &Path, version: &str, options: &Options) -> Result<(), String> {
    windows::installation_directory()?;
    let helper = setup
        .parent()
        .ok_or("Missing update directory")?
        .join("AlcMute-update-helper.exe");
    fs::copy(std::env::current_exe().map_err(|e| e.to_string())?, &helper)
        .map_err(|e| e.to_string())?;
    Command::new(&helper)
        .arg("--update-helper")
        .arg(std::process::id().to_string())
        .arg(setup)
        .arg(version)
        .arg(&options.data_dir)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}
pub fn helper(args: &[String]) -> Result<(), String> {
    if !cfg!(feature = "installer") || args.len() != 5 {
        return Err("Invalid update helper invocation".into());
    }
    let pid: u32 = args[1].parse().map_err(|_| "Invalid parent PID")?;
    let setup = PathBuf::from(&args[2]);
    let version = semver_version(&args[3])?;
    let options = Options {
        data_dir: PathBuf::from(&args[4]),
        benchmark: false,
    };
    let result = (|| -> Result<(), String> {
        let install = windows::registered_directory()?;
        let local = PathBuf::from(std::env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA missing")?)
            .join("Alcnaka/AlcMute/updates");
        let root = fs::canonicalize(local).map_err(|e| e.to_string())?;
        let setup = fs::canonicalize(&setup).map_err(|e| e.to_string())?;
        let helper = fs::canonicalize(std::env::current_exe().map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        if !setup.starts_with(&root)
            || helper.parent() != setup.parent()
            || setup.file_name().and_then(|v| v.to_str())
                != Some(format!("AlcMute-{version}-Setup.exe").as_str())
        {
            return Err("Untrusted staging path".into());
        }
        updater::install_update(&setup, &version, pid, &install, &options.data_dir)?;
        Ok(())
    })();
    if let Err(error) = &result {
        options.log(error);
        windows::show_error(
            "AlcMuteの更新に失敗しました",
            &format!("{error}\n既存のインストールと設定を確認し、再試行してください。"),
        );
        if let Ok(dir) = windows::registered_directory() {
            let _ = Command::new(dir.join("alcmute.exe"))
                .arg("--background")
                .arg("--data-dir")
                .arg(&options.data_dir)
                .spawn();
        }
    }
    // Keep Setup/log/helper in the private staging directory for diagnosis;
    // no recursive deletion of user-supplied paths and no changes to old EXEs.
    result
}
fn semver_version(value: &str) -> Result<alcmute_core::updater::Version, String> {
    value
        .parse()
        .map_err(|e| format!("Invalid update version: {e}"))
}
