# 開発

## ビルド

Rust 1.92以上のMSVCツールチェーンと、Visual Studio Build Toolsの
「C++によるデスクトップ開発」が必要です。リポジトリのルートで実行します。

```powershell
cargo build --release --locked
```

実行ファイルは`target/release/alcmute.exe`です。
既定はPortable版です。Installer版は`cargo build --release --features installer --locked`でビルドします。

WindowsのVERSIONINFOは`build.rs`で`winresource`を使って生成します。
文字列バージョンはCargoのversion、数値バージョンはmajor.minor.patch.0です。
Windows SDKのリソースコンパイラーが必要です。ビルド後に次のコマンドで検証できます。

```powershell
./scripts/verify-version-info.ps1
```

CIでも会社名・製品名・説明・内部名・元ファイル名・著作権と、文字列／数値の両バージョンを検証します。

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked -- --test-threads=1
cargo test --workspace --features dev-tools --locked -- --test-threads=1
cargo test --workspace --features installer --locked -- --test-threads=1
cargo clippy --workspace --all-targets --features installer --locked -- -D warnings
```

テストはローカルUDPとテスト専用レジストリ領域を使用し、VRChatの起動を必要としません。
実VRChatでのOSC連携や混在DPIでの表示は、別途実機で確認してください。
CIでも書式・通常／InstallerのClippy（警告をエラー扱い）・通常／dev-tools／Installerのテスト・両形式のReleaseビルドを実行します。

回帰テストでは、通信を維持した設定保存、ポート競合、保存失敗、ホットキーの登録・復旧失敗、
通信の生成・復旧失敗、キュー制限・終了、二重起動Mutex、破損設定の保護を確認します。
レジストリ更新・復旧の異常系は障害注入でも検証し、実際の自動起動設定は変更しません。
Release前には実VRChatでの操作、トレイ・ショートカット、二重起動の案内、
ポート競合・破損設定のダイアログ、100%／125%など異DPIでの画面移動を実機で確認してください。

## 構成

```text
src/                  Slint設定画面・トレイ・キー記録
crates/alcmute-core/   OSC・ショートカット・設定・Windows連携
assets/               アイコン
vendor/               DPI対策を取り込んだ依存ソースと変更記録
.github/workflows/    Windowsのビルド・テスト
```

PoCのNWG版、計測結果、実行ログ、個人の設定ファイルは含みません。
開発用操作は通常ビルドでは受け付けません。必要な場合だけ
`cargo build --features dev-tools`でビルドし、`--test-mode --data-dir <テスト専用フォルダー>`
で起動します。`/alcmute/test/*` OSC操作を有効にしますが、Portable版の自動起動禁止は維持します。
自動起動の書込み試験は単体テスト専用のレジストリ領域で行います。配布では`dev-tools`を有効にしないでください。

## ライセンス情報の更新

依存関係を変更した場合はPowerShell 7で`./scripts/update-license-notices.ps1`を実行し、
`assets/dependency-licenses.txt`を更新してください。
依存ライセンスはビルド時にzlib圧縮して埋め込みます。テストで展開後の全文一致を確認し、
CIではMIT本文、製品アイコンの各サイズと元ICOの画像データ一致、VERSIONINFO、配布種別と`--verify-licenses`の終了コードも確認します。

UIは`src/`、Windows・OSC・設定処理は`crates/alcmute-core/`へ分離しています。
ショートカットは左右共通・左右指定ともWindowsの低レベルキーボードフックで検出し、設定・Aboutが前面の場合はwinitの入力でも補完します。Windowsの元の入力時刻・キー・押下／解放を64件以内の履歴で照合し、遅れて届く重複入力を除外します。記録中だけミュート操作を停止し、フック処理では軽量な判定とキュー通知だけを行います。キー状態は入力イベント時に同期し、現在の入力をリピートと誤認しないよう扱います。常時ポーリングは追加しません。
入力時刻の対応は[KBDLLHOOKSTRUCT.timeの仕様](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-kbdllhookstruct)に基づきます。実ウィンドウの前面化と実キー操作は別途手動確認が必要です。
`global-hotkey`は既存設定のキー名・別名を解析するためだけに使用し、製品コードで`RegisterHotKey`や同ライブラリのイベントハンドラーは使用しません。予約済みホットキーとの共存テストだけがWindows標準登録を使用します。
設定項目の変更では、旧設定の読み込み、保存、Reset、全初期化、未保存判定を合わせて確認します。

依存ソースの変更は`vendor/README.md`に理由と上流の参照先を記載し、元のライセンスを保持します。
Gitには`target/`、計測ログ、個人の`settings.toml`を含めないでください。

## 変更履歴

利用者に関わる変更は`CHANGELOG.md`の`Unreleased`に追記します。
コミット履歴のコピーではなく、利用者から見て何が変わったかを記載してください。
`Added`（追加）、`Changed`（変更）、`Deprecated`（非推奨）、`Removed`（削除）、
`Fixed`（修正）、`Security`（セキュリティ）のうち、該当する見出しのみ使用します。

リリース時に対象の変更をバージョンと日本時間の公開日付きの節へ移し、
`Unreleased`を次の開発用に残します。末尾の比較リンクも新しいタグに更新し、
公開するGitHub Releaseの説明に対応する変更内容を記載します。

## インストーラーと配布物

PowerShell 7とInno Setup 6.3以降（検証済み6.7.3）が必要です。公式コンパイラを導入し、ルートで実行します。

```powershell
./scripts/package.ps1 -Iscc 'C:/Program Files (x86)/Inno Setup 6/ISCC.exe'
```

`dist/AlcMute-<version>-Setup.exe`、`dist/AlcMute-<version>-Portable.zip`、
`dist/SHA256SUMS.txt`を生成します。ビルドキャッシュは`-TargetRoot`で指定できます。
同じキャッシュで両形式を順番にビルドし、検証したEXEを形式別に保存します。
配布物に`dev-tools`は有効化しません。インストーラーのAppIdを変更しないでください。
ソースや素材を変更した後は両形式を再生成してください。以前のZIPや展開済みEXEは自動更新されません。
`dist/build/portable`・`dist/build/installer`は形式別ビルド、`dist/portable`はZIP作成用の一時配置です。
利用者向けには生成済みのSetupまたはZIPを渡し、古い`target/release`や過去の展開先と混同しないでください。

個別のEXEは次のコマンドで検証できます。

```powershell
./scripts/verify-artifact.ps1 -Executable dist/build/portable/alcmute.exe -Distribution portable
./scripts/verify-artifact.ps1 -Executable dist/build/installer/alcmute.exe -Distribution installer
./scripts/verify-install-preflight.ps1 -Executable dist/build/installer/alcmute.exe
```

事前確認スクリプトはAlcMuteを終了した状態で実行し、未起動・起動中相当のMutex・解放後を確認します。
実アプリを起動したりRunキーを変更したりしません。
インストーラーはファイル配置前に起動中の旧版を検出します。配置後は自動起動の移行が成功してから本体を起動します。
`[Run]`は`ssPostInstall`より前に処理されるため、本体の先行起動を追加しないでください。
サイレント更新後の検証・再起動は更新ヘルパーが担当します。

新規インストール先はLocalAppData配下の空フォルダーに制限します。
更新時はInnoの既存AppId・InstallDir・Uninstall登録を照合し、旧アプリを終了してから実行します。
実ユーザーのRunを書き換えるE2Eは、テスト用ユーザー／VMで行ってください。
開発用`--test-mode`も、Portableの自動起動禁止を回避しません。レジストリの書込みテストは`cfg(test)`だけが専用キーで行います。

アイコンの素材と再生成方法は`assets/APP-ICON.md`を参照してください。
設計、安全上の制限と手動テスト一覧は[検証記録](docs/v0.3.0-validation.md)にあります。

## Release作成

1. 本体／coreのCargoバージョンとCargo.lockを合わせ、依存ライセンスを再収集します。
2. CHANGELOGのUnreleasedを公開日付きのバージョン節へ移し、比較リンク・READMEを更新します。
3. fmt・両形式のClippy／テスト・Releaseビルド・配布物検証を完了します。
4. 変更をコミットし、明示的に許可されたときだけpushと`v<version>`タグの作成・pushを行います。
5. CIはタグとCargoバージョンを照合し、成果物を添付したRelease下書きを生成します。
6. ハッシュと手動E2E結果を確認し、説明とSlint帰属表示を確認してから、公開の承認を得て発行します。

通常のpush／PRでは正式Releaseも下書きも作成しません。GitHubの書込み権限はタグ用Releaseジョブだけに付与します。
