# AlcMute

## あるみゅーと

**VRC Background Mute**

[![Made with Slint](https://raw.githubusercontent.com/slint-ui/slint/master/logo/MadeWithSlint-logo-whitebg.png)](https://slint.dev/)

VRChatが非アクティブでも、ショートカットやWindowsの通知領域からマイクのミュートを
切り替える、Rust製のWindows常駐アプリです。VRChat公式OSCを使用します。

## 機能

- グローバルショートカットによるミュート切り替え
- 左右のCtrl・Altなどを区別した登録、修飾キーだけの組み合わせ、`K + L`などの同時押し
- VRChatから取得したミュート状態に応じたトレイアイコン
- トレイメニューと、動作を選べるダブルクリック
- トレイメニューの「AlcMuteについて」で制作者・リポジトリ・ライセンス本文を表示
- 設定画面、項目ごとのReset、確認付きの全初期化
- Windowsサインイン時の自動起動、起動時の設定画面表示の切り替え
- 異なるDPIのモニター間で移動するためのWindows描画・ウィンドウ処理の対策

対応OSはWindows 10 / 11です。Slintとソフトウェア描画を採用しています。

## 使い方

1. VRChatでOSCと「Toggle Voice」を有効にします。
2. `alcmute.exe`を起動します。初期状態では設定画面を表示します。
3. ショートカットを変更する場合は「キーを変更」を押し、組み合わせを押してから
   すべて離します。「保存」で反映します。記録中のみミュートのショートカットを停止します。
4. 「閉じる」やウィンドウの×は設定画面だけを閉じます。アプリの終了は
   トレイメニューの「終了」、または設定画面の「ツールを終了」から行います。

| 設定 | 初期値 |
| --- | --- |
| ショートカット | Ctrl + Alt + M |
| トレイのダブルクリック | ミュート切り替え |
| Windowsサインイン時の起動 | 無効 |
| 起動時に設定画面を表示 | 有効 |
| OSC送信先 | 127.0.0.1:9000 |
| OSC受信先 | 127.0.0.1:9001 |

キー入力は他のアプリにも渡されます。設定変更がある場合だけ「保存」が有効になります。
全初期化は保存済み設定を削除し、自動起動も無効にします。

## 設定ファイル

既存ユーザーの設定を引き継ぐため、保存先は従来の名前を維持しています。

```text
%LOCALAPPDATA%\VRChatMuteController\settings.toml
```

Windowsの自動起動の登録名も互換性のため`VRChatMuteController`を維持します。
自動起動はEXEの場所も判定するため、PoC版から移行した際は必要に応じて設定画面で
自動起動を再度有効にし、「保存」で新しいEXEの場所を登録してください。
別の保存先を使う場合は`alcmute.exe --data-dir <フォルダー>`を指定します。
初期値の項目は保存を省略するため、全項目が初期値なら設定ファイルは存在しません。

## ビルド

Rust 1.92以上のMSVCツールチェーンと、Visual Studio Build Toolsの
「C++によるデスクトップ開発」が必要です。リポジトリのルートで実行します。

```powershell
cargo build --release --locked
```

実行ファイルは`target/release/alcmute.exe`です。

```powershell
cargo fmt --all -- --check
cargo test --workspace --locked -- --test-threads=1
```

テストはローカルUDPとテスト専用レジストリ領域を使用し、VRChatの起動を必要としません。
実VRChatでのOSC連携や混在DPIでの表示は、別途実機で確認してください。

## 構成

```text
src/                  Slint設定画面・トレイ・キー記録
crates/alcmute-core/   OSC・ショートカット・設定・Windows連携
assets/               アイコン
vendor/               DPI対策を取り込んだ依存ソースと変更記録
.github/workflows/    Windowsのビルド・テスト
```

PoCのNWG版、計測結果、実行ログ、個人の設定ファイルは含みません。
開発時の`--test-mode`は自動起動をテスト専用レジストリへ切り替え、通常は無効な
`/alcmute/test/*` OSC操作を受け付けます。通常利用では指定しないでください。

## 配布

[GitHub Releases](https://github.com/alcnaka/AlcMute/releases)からWindows向けの
ポータブル版をダウンロードできます。ZIPを展開し、`alcmute.exe`を起動してください。
インストーラーは今後対応予定です。

変更内容は[変更履歴（CHANGELOG.md）](CHANGELOG.md)を参照してください。

## ライセンス

AlcMuteの独自コードは[MIT License](LICENSE)です。
依存ライブラリと`vendor/`のソースには、それぞれのライセンスが適用されます。
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)と各ライセンスファイルを参照してください。

実行ファイルにはMIT本文と依存ライブラリのライセンス情報を埋め込み、
トレイメニューの「AlcMuteについて」から表示できます。
依存関係を変更した場合はPowerShell 7で`./scripts/update-license-notices.ps1`を実行し、
`assets/dependency-licenses.txt`を更新してください。
