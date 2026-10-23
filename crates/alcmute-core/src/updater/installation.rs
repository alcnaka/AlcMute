//! Shared install transaction. Test substitution stays private to this module.
use super::{Client, Version};
use crate::{windows, Result};
use std::{fs, io::Read, os::windows::fs::OpenOptionsExt, path::Path, process::Command};

pub fn install_update(
    setup: &Path,
    version: &Version,
    pid: u32,
    install: &Path,
    data: &Path,
) -> Result<()> {
    if !cfg!(feature = "installer") {
        return Err("Installer distribution required".into());
    }
    if windows::registered_directory()? != fs::canonicalize(install).map_err(|e| e.to_string())? {
        return Err("Unmanaged update installation".into());
    }
    perform(
        &Client::default(),
        setup,
        version,
        pid,
        install,
        data,
        &windows::installed_version,
    )
}

fn perform(
    client: &Client,
    setup: &Path,
    version: &Version,
    pid: u32,
    install: &Path,
    data: &Path,
    installed_version: &dyn Fn() -> Result<String>,
) -> Result<()> {
    // Re-fetch and verify independently of the download/UI process.
    let release = client.fetch_version(version)?;
    let hash = client.release_hash(&release)?;
    let mut locked_setup = fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(setup)
        .map_err(|e| e.to_string())?;
    let size = locked_setup.metadata().map_err(|e| e.to_string())?.len();
    if size > super::MAX_SETUP_SIZE {
        return Err("Oversized installer".into());
    }
    let mut bytes = Vec::new();
    locked_setup
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    super::verify(&bytes, release.setup()?.size, &hash)?;
    drop(bytes);
    if *version
        <= installed_version()?
            .parse::<Version>()
            .map_err(|e| e.to_string())?
    {
        return Err("更新先は現在のバージョンより新しくありません".into());
    }
    windows::wait_for_update_parent(pid, &install.join("alcmute.exe"))
        .map_err(|e| format!("更新前のAlcMuteの終了確認に失敗しました: {e}"))?;
    let previous = setup
        .parent()
        .ok_or("Missing staging directory")?
        .join("previous-alcmute.exe");
    let mut backup = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(previous)
        .map_err(|e| e.to_string())?;
    std::io::copy(
        &mut fs::File::open(install.join("alcmute.exe")).map_err(|e| e.to_string())?,
        &mut backup,
    )
    .and_then(|_| backup.sync_all())
    .map_err(|e| e.to_string())?;
    drop(backup);
    let status = Command::new(setup)
        .args(["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART"])
        .arg(format!("/DIR={}", windows::normal_path(install)?))
        .arg(format!("/LOG={}", setup.with_extension("log").display()))
        .status()
        .map_err(|e| format!("更新インストーラーを起動できませんでした: {e}"))?;
    if !status.success() {
        return Err(format!("インストールに失敗しました: {status}"));
    }
    if installed_version()? != version.to_string()
        || windows::executable_version(&install.join("alcmute.exe"))? != version.to_string()
    {
        return Err("インストール後のバージョンを確認できませんでした".into());
    }
    Command::new(install.join("alcmute.exe"))
        .args(["--background", "--data-dir"])
        .arg(data)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
#[path = "local_tests.rs"]
mod local_tests;
