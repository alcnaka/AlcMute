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
