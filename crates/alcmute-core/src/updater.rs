//! External HTTPS update traffic is separate from loopback OSCQuery.
//! Release asset validation is independent of UI/install mechanics, so a future
//! signature verifier can strengthen this trust boundary without changing UI.
use crate::Result;
pub use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{fs, io::Write, path::Path, time::Duration};

pub const RELEASES: &str = "https://github.com/alcnaka/AlcMute/releases";
const API: &str = "https://api.github.com/repos/alcnaka/AlcMute/releases";
pub const MAX_SETUP_SIZE: u64 = 128 * 1024 * 1024;
#[derive(Clone, Debug, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
    pub state: String,
    #[serde(default)]
    pub digest: Option<String>,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub html_url: String,
    pub draft: bool,
    pub prerelease: bool,
    pub assets: Vec<Asset>,
}
impl Release {
    pub fn version(&self) -> Result<Version> {
        let version = Version::parse(
            self.tag_name
                .strip_prefix('v')
                .ok_or("Release tag must start with v")?,
        )
        .map_err(|e| e.to_string())?;
        if self.draft
            || self.prerelease
            || !version.pre.is_empty()
            || !version.build.is_empty()
            || self.tag_name != format!("v{version}")
        {
            return Err("通常の安定版Releaseではありません".into());
        }
        if self.html_url != format!("{RELEASES}/tag/v{version}") {
            return Err("Unexpected release URL".into());
        }
        Ok(version)
    }
    pub fn newer_than(&self, current: &str) -> Result<bool> {
        Ok(self.version()? > Version::parse(current).map_err(|e| e.to_string())?)
    }
    pub fn asset(&self, name: &str, max: u64) -> Result<&Asset> {
        self.version()?;
        let found: Vec<_> = self
            .assets
            .iter()
            .filter(|asset| asset.name == name)
            .collect();
        if found.len() != 1 {
            return Err(format!("Release asset missing or duplicated: {name}"));
        }
        let asset = found[0];
        let expected = format!("{RELEASES}/download/{}/{name}", self.tag_name);
        if asset.browser_download_url != expected
            || asset.size == 0
            || asset.size > max
            || asset.state != "uploaded"
        {
            return Err("Invalid release asset URL/size/state".into());
        }
        Ok(asset)
    }
    pub fn setup(&self) -> Result<&Asset> {
        self.asset(
            &format!("AlcMute-{}-Setup.exe", self.version()?),
            MAX_SETUP_SIZE,
        )
    }
}
mod installation;
pub use installation::install_update;

#[derive(Default)]
struct Client {
    #[cfg(test)]
    local: Option<String>,
}
impl Client {
    fn https_only(&self) -> bool {
        #[cfg(test)]
        if self.local.is_some() {
            return false;
        }
        true
    }
    fn endpoint(&self, official: &str) -> Result<String> {
        #[cfg(test)]
        if let Some(base) = &self.local {
            // Only unit-test code can construct a local client. Metadata still
            // contains canonical GitHub URLs and uses the production validators.
            let path = if let Some(path) = official.strip_prefix(API) {
                format!("/api{path}")
            } else if let Some(path) = official.strip_prefix(RELEASES) {
                format!("/releases{path}")
            } else {
                return Err("Test transport rejected an external URL".into());
            };
            return Ok(format!("{base}{path}"));
        }
        Ok(official.to_string())
    }
    fn agent(&self) -> ureq::Agent {
        ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(30)))
            .timeout_connect(Some(Duration::from_secs(5)))
            .max_redirects(0)
            .https_only(self.https_only())
            .build()
            .into()
    }
    fn fetch_version(&self, version: &Version) -> Result<Release> {
        let release = self
            .fetch(&format!("{API}/tags/v{version}"))?
            .ok_or("Release not found")?;
        if release.version()? != *version {
            return Err("Release version mismatch".into());
        }
        Ok(release)
    }
    fn fetch(&self, endpoint: &str) -> Result<Option<Release>> {
        let mut response = match self
            .agent()
            .get(&self.endpoint(endpoint)?)
            .header("User-Agent", concat!("AlcMute/", env!("CARGO_PKG_VERSION")))
            .header("Accept", "application/vnd.github+json")
            .call()
        {
            Ok(value) => value,
            Err(ureq::Error::StatusCode(status)) => return parse_response(status, ""),
            Err(e) => return Err(format!("更新情報を取得できません: {e}")),
        };
        let text = response
            .body_mut()
            .with_config()
            .limit(2 * 1024 * 1024)
            .read_to_string()
            .map_err(|e| e.to_string())?;
        parse_response(response.status().as_u16(), &text)
    }
}
pub fn fetch_latest() -> Result<Option<Release>> {
    Client::default().fetch(&format!("{API}/latest"))
}
pub fn fetch_version(version: &Version) -> Result<Release> {
    Client::default().fetch_version(version)
}
fn parse_response(status: u16, text: &str) -> Result<Option<Release>> {
    if status == 404 {
        return Ok(None);
    }
    if status != 200 {
        return Err(format!("GitHub API HTTP {status}"));
    }
    let release: Release = serde_json::from_str(text).map_err(|e| e.to_string())?;
    release.version()?;
    Ok(Some(release))
}
fn allowed_redirect(url: &url::Url) -> bool {
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none_or(|port| port == 443)
        && url.fragment().is_none()
        && matches!(
            url.host_str(),
            Some(
                "github.com"
                    | "release-assets.githubusercontent.com"
                    | "objects.githubusercontent.com"
            )
        )
}
impl Client {
    fn download(&self, asset: &Asset, limit: u64) -> Result<Vec<u8>> {
        let mut current =
            url::Url::parse(&asset.browser_download_url).map_err(|e| e.to_string())?;
        for _ in 0..5 {
            if !allowed_redirect(&current) {
                return Err("Untrusted download redirect".into());
            }
            let mut response = self
                .agent()
                .get(&self.endpoint(current.as_str())?)
                .header("User-Agent", "AlcMute")
                .call()
                .map_err(|e| e.to_string())?;
            if response.status().is_redirection() {
                let location = response
                    .headers()
                    .get("location")
                    .and_then(|v| v.to_str().ok())
                    .ok_or("Missing redirect location")?;
                current = current.join(location).map_err(|e| e.to_string())?;
                continue;
            }
            if !response.status().is_success() {
                return Err("Asset download failed".into());
            }
            let bytes = response
                .body_mut()
                .with_config()
                .limit(limit)
                .read_to_vec()
                .map_err(|e| e.to_string())?;
            if bytes.len() as u64 != asset.size {
                return Err("Asset size mismatch".into());
            }
            return Ok(bytes);
        }
        Err("Too many download redirects".into())
    }
}
pub fn checksum(manifest: &str, name: &str) -> Result<[u8; 32]> {
    let matches: Vec<_> = manifest
        .lines()
        .filter_map(|line| line.split_once("  "))
        .filter(|(_, file)| *file == name)
        .collect();
    if matches.len() != 1
        || matches[0].0.len() != 64
        || !matches[0].0.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err("SHA256SUMS entry missing/invalid/duplicated".into());
    }
    let hash = matches[0].0;
    let mut bytes = [0u8; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte =
            u8::from_str_radix(&hash[index * 2..index * 2 + 2], 16).map_err(|e| e.to_string())?;
    }
    Ok(bytes)
}
pub fn verify(bytes: &[u8], size: u64, hash: &[u8; 32]) -> Result<()> {
    if bytes.len() as u64 != size
        || size > MAX_SETUP_SIZE
        || Sha256::digest(bytes).as_slice() != hash
    {
        return Err("ダウンロードのサイズまたはSHA-256が一致しません。実行しませんでした。".into());
    }
    if bytes.len() < 64 || &bytes[..2] != b"MZ" {
        return Err("Invalid Windows installer".into());
    }
    let offset = u32::from_le_bytes(bytes[60..64].try_into().unwrap()) as usize;
    if bytes.get(offset..offset.saturating_add(4)) != Some(b"PE\0\0") {
        return Err("Invalid PE header".into());
    }
    Ok(())
}
impl Client {
    fn release_hash(&self, release: &Release) -> Result<[u8; 32]> {
        let setup = release.setup()?;
        let manifest = release.asset("SHA256SUMS.txt", 64 * 1024)?;
        let bytes = self.download(manifest, 64 * 1024)?;
        let hash = checksum(
            std::str::from_utf8(&bytes).map_err(|e| e.to_string())?,
            &setup.name,
        )?;
        if let Some(digest) = &setup.digest {
            let expected = format!(
                "sha256:{}",
                hash.iter().map(|b| format!("{b:02x}")).collect::<String>()
            );
            if digest != &expected {
                return Err("GitHub digest and SHA256SUMS disagree".into());
            }
        }
        Ok(hash)
    }
    fn stage(&self, release: &Release, dir: &Path) -> Result<std::path::PathBuf> {
        let setup = release.setup()?;
        let hash = self.release_hash(release)?;
        let bytes = self.download(setup, MAX_SETUP_SIZE)?;
        verify(&bytes, setup.size, &hash)?;
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let path = dir.join(&setup.name);
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| e.to_string())?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        Ok(path)
    }
}
pub fn release_hash(release: &Release) -> Result<[u8; 32]> {
    Client::default().release_hash(release)
}
pub fn stage(release: &Release, dir: &Path) -> Result<std::path::PathBuf> {
    Client::default().stage(release, dir)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "Opt-in external GitHub HTTPS smoke test"]
    fn github_https_smoke() {
        let release = fetch_latest().unwrap().unwrap();
        assert!(release.version().is_ok());
        assert!(!release.draft && !release.prerelease);
    }
    fn release() -> Release {
        Release {
            tag_name: "v0.3.1".into(),
            html_url: format!("{RELEASES}/tag/v0.3.1"),
            draft: false,
            prerelease: false,
            assets: vec![Asset {
                name: "AlcMute-0.3.1-Setup.exe".into(),
                browser_download_url: format!("{RELEASES}/download/v0.3.1/AlcMute-0.3.1-Setup.exe"),
                size: 100,
                state: "uploaded".into(),
                digest: None,
            }],
        }
    }
    #[test]
    fn validates_versions_and_assets_and_never_downgrades() {
        let mut release = release();
        assert!(release.newer_than("0.3.0").unwrap());
        assert!(!release.newer_than("0.4.0").unwrap());
        assert!(!release.newer_than("0.3.1").unwrap());
        assert!(release.setup().is_ok());
        release.assets[0].browser_download_url = "https://evil.example/setup.exe".into();
        assert!(release.setup().is_err());
        release.draft = true;
        assert!(release.version().is_err());
        release.draft = false;
        release.prerelease = true;
        assert!(release.version().is_err());
        for tag in ["v0.4.0-beta.1", "v0.4.0+build", "v../evil", "0.4.0"] {
            release.tag_name = tag.into();
            assert!(release.version().is_err());
        }
    }
    #[test]
    fn rejects_untrusted_redirects_and_corrupt_downloads() {
        for url in [
            "http://github.com/file",
            "https://github.com.evil.test/file",
            "https://evil.test/file",
            "https://user@github.com/file",
            "https://github.com:444/file",
        ] {
            assert!(!allowed_redirect(&url.parse().unwrap()));
        }
        assert!(allowed_redirect(
            &"https://release-assets.githubusercontent.com/a?token=secret"
                .parse()
                .unwrap()
        ));
        assert!(verify(b"bad", 3, &[0; 32]).is_err());
        assert!(checksum("wrong  Setup.exe", "Setup.exe").is_err());
        let mut pe = vec![0u8; 128];
        pe[..2].copy_from_slice(b"MZ");
        pe[60..64].copy_from_slice(&64u32.to_le_bytes());
        pe[64..68].copy_from_slice(b"PE\0\0");
        let hash: [u8; 32] = Sha256::digest(&pe).into();
        assert!(verify(&pe, 128, &hash).is_ok());
        pe[80] = 1;
        assert!(verify(&pe, 128, &hash).is_err());
        assert!(verify(&pe, 129, &hash).is_err());
    }
    #[test]
    fn checksum_requires_one_exact_asset() {
        let line = format!("{}  Setup.exe", "a".repeat(64));
        assert_eq!(checksum(&line, "Setup.exe").unwrap(), [0xaa; 32]);
        assert!(checksum(&format!("{line}\n{line}"), "Setup.exe").is_err());
        assert!(checksum(&line, "evil.exe").is_err());
        assert!(checksum(
            &format!("{}  Setup.exe", "あ".repeat(21) + "x"),
            "Setup.exe"
        )
        .is_err());
    }
    #[test]
    fn handles_missing_release_api_errors_and_malformed_response() {
        assert!(parse_response(404, "").unwrap().is_none());
        for status in [301, 403, 429, 500] {
            assert!(parse_response(status, "").is_err());
        }
        assert!(parse_response(200, "not JSON").is_err());
        assert!(parse_response(200,r#"{"tag_name":"v0.3.1","html_url":"https://evil.test/","draft":false,"prerelease":false,"assets":[]}"#).is_err());
    }
}
