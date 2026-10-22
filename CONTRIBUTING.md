# 開発

## ビルド

Rust 1.92以上のMSVCツールチェーンと、Visual Studio Build Toolsの
「C++によるデスクトップ開発」が必要です。リポジトリのルートで実行します。

```powershell
cargo build --release --locked
```

実行ファイルは`target/release/alcmute.exe`です。

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked -- --test-threads=1
cargo test --workspace --features dev-tools --locked -- --test-threads=1
```

テストはローカルUDPとテスト専用レジストリ領域を使用し、VRChatの起動を必要としません。
実VRChatでのOSC連携や混在DPIでの表示は、別途実機で確認してください。
CIでも書式・Clippy（警告をエラー扱い）・通常／開発用featureのテスト・Releaseビルドを実行します。

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
で起動します。このモードは自動起動をテスト専用レジストリへ切り替え、
`/alcmute/test/*` OSC操作を有効にします。通常利用・配布では有効にしないでください。

## ライセンス情報の更新

依存関係を変更した場合はPowerShell 7で`./scripts/update-license-notices.ps1`を実行し、
`assets/dependency-licenses.txt`を更新してください。
CIでは配布するEXEにMIT本文と収集済み依存ライセンス本文が埋め込まれていることも確認します。

UIは`src/`、Windows・OSC・設定処理は`crates/alcmute-core/`へ分離しています。
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
