fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;
    println!("cargo:rerun-if-changed=assets/dependency-licenses.txt");
    println!("cargo:rerun-if-changed=assets/app.ico");
    let mut compressed = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
    let source = std::fs::read("assets/dependency-licenses.txt")?;
    println!("cargo:rustc-env=ALCMUTE_LICENSE_BYTES={}", source.len());
    compressed.write_all(&source)?;
    std::fs::write(
        std::path::Path::new(&std::env::var("OUT_DIR")?).join("licenses.zlib"),
        compressed.finish()?,
    )?;
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
        .set_icon("assets/app.ico")
        .set(
            "Comments",
            if std::env::var_os("CARGO_FEATURE_INSTALLER").is_some() {
                "Installer"
            } else {
                "Portable"
            },
        )
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
