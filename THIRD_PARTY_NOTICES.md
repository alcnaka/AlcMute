# Third-party notices

AlcMute's own source is MIT-licensed. This does not relicense dependencies or
vendored third-party code. Preserve their copyright and license notices when
redistributing them.

- **Slint 1.18.1**: `GPL-3.0-only OR LicenseRef-Slint-Royalty-free-2.0 OR
  LicenseRef-Slint-Software-3.0`. The vendored backend retains its original
  `LICENSES/` directory. See https://slint.dev/license/ and the exact license
  texts in `vendor/i-slint-backend-winit/LICENSES/` for terms.
- **winit 0.30.13**: see `vendor/winit/LICENSE` and its Cargo package metadata.
- Other Rust dependencies retain the licenses declared in their Cargo packages.
  `Cargo.lock` records the versions used by this repository.

Local changes to the vendored sources are described in `vendor/README.md`.
The README includes the official Made with Slint attribution badge. Preserve
this attribution on the public project/download page when distributing under
Slint's royalty-free application license; consult the bundled license text.
The application embeds the collected dependency notices and license texts in
`assets/dependency-licenses.txt`. Open "AlcMuteについて" from the tray menu to
read them without external license files. Regenerate this asset with
`scripts/update-license-notices.ps1` whenever Cargo.lock changes.
For packages which omit license files from their crate archive, the generator
uses the upstream license texts in `assets/license-overrides/`; these retain
their official source URLs. Review these texts when updating those packages.
