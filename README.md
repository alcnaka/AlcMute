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

OSC送受信先以外の変更や同じ設定の再保存では、通信と取得済みのミュート状態を維持します。
OSC設定を変更する場合だけ通信を再構成し、失敗時は従来の通信とショートカットの復旧を試みます。
復旧できなかった場合は画面で案内します。設定を再保存するか、AlcMuteを再起動してください。

同じユーザー・Windowsセッションで二重起動すると通知を表示します。
起動済みのアプリは通知領域から操作してください。起動に失敗した場合は、
エラー概要と`error.log`の保存先をダイアログに表示します。ログは設定と同じフォルダーです。
設定ファイルが破損している場合は自動で削除せず停止します。ファイルをバックアップしてから
内容を修正するか、名前を変更して初期設定で起動してください。

状態取得とミュート操作は独立しています。OSCQueryで状態を取得できなくても、
通信ワーカーが動作していれば、トレイ・設定画面・ショートカットから切り替えを送信できます。
未確認の状態をミュート済みとは表示しません。VRChat未起動時も送信は可能ですが、
実際に反映するにはVRChat側のOSCと「Toggle Voice」が必要です。
切り替え要求は最大32件を順番に処理し、1回ごとの押下40ms・解放40msを維持します。
上限を超えた要求は蓄積せず、操作が多すぎる旨を状態欄に表示します。

## 設定ファイル

既定の保存先は次のとおりです。

```text
%LOCALAPPDATA%\Alcnaka\AlcMute\settings.toml
```

Windowsの自動起動の登録名も`AlcMute`です。
旧保存先`%LOCALAPPDATA%\VRChatMuteController`の設定は、初回起動時に引き継ぎます。
新保存先に設定がある場合はそちらを優先します。旧ファイルはバックアップとして残し、
移行済みの印を保存することで、全初期化後に旧設定が復活するのを防ぎます。
旧名の自動起動エントリがある場合は、新名で現在のEXE・設定保存先を登録してから削除します。
新名のエントリがすでにある場合はそちらを優先します。
旧版の自動起動が指定する旧既定保存先も、新しい保存先へ切り替えます。
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
CIでは配布するEXEにMIT本文と収集済み依存ライセンス本文が埋め込まれていることも確認します。
