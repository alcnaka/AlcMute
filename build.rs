fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    if std::env::var("CARGO_CFG_TARGET_OS")? != "windows" {
        return Ok(());
    }

    let version = std::env::var("CARGO_PKG_VERSION")?;
    let major: u16 = std::env::var("CARGO_PKG_VERSION_MAJOR")?.parse()?;
    let minor: u16 = std::env::var("CARGO_PKG_VERSION_MINOR")?.parse()?;
    let patch: u16 = std::env::var("CARGO_PKG_VERSION_PATCH")?.parse()?;
    let numeric = (u64::from(major) << 48) | (u64::from(minor) << 32) | (u64::from(patch) << 16);
    let mut resource = winresource::WindowsResource::new();
    resource
        .set("CompanyName", "Alcnaka")
        .set("ProductName", "AlcMute")
        .set("FileDescription", "AlcMute - VRC Background Mute")
        .set("InternalName", "alcmute")
        .set("OriginalFilename", "alcmute.exe")
        .set("LegalCopyright", "Copyright (c) 2026 Alcnaka")
        .set("FileVersion", &version)
        .set("ProductVersion", &version)
        .set_version_info(winresource::VersionInfo::FILEVERSION, numeric)
        .set_version_info(winresource::VersionInfo::PRODUCTVERSION, numeric);
    resource.compile()?;
    Ok(())
}
