# 開発

Windows上でルートの`Cargo.toml`を使用します。

```powershell
cargo fmt --all -- --check
cargo test --workspace --locked -- --test-threads=1
cargo build --release --locked
```

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
