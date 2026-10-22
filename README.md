# AlcMute

## あるみゅーと

**VRC Background Mute**

<a href="https://slint.dev/">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="./assets/img/MadeWithSlint-logo-dark.svg">
    <source media="(prefers-color-scheme: light)" srcset="./assets/img/MadeWithSlint-logo-light.svg">
    <img alt="#MadeWithSlint" src="./assets/img/MadeWithSlint-logo-light.svg" width="212" height="97">
  </picture>
</a>

VRChatが非アクティブでも、ショートカットやWindowsの通知領域からマイクのミュートを
切り替える、Rust製のWindows常駐アプリです。VRChat公式OSCを使用します。

## 機能

- グローバルショートカットによるミュート切り替え
- 左右のCtrl・Altなどを区別した登録、修飾キーだけの組み合わせ、`K + L`などの同時押し
- VRChatから取得したミュート状態に応じたトレイアイコン
- トレイメニューと、動作を選べるダブルクリック
- トレイメニューまたは設定画面の「AlcMuteについて」で制作者・リポジトリ・ライセンス本文を表示
- 設定画面、項目ごとのReset、確認付きの全初期化
- Windowsサインイン時の自動起動、起動時の設定画面表示の切り替え
- 表示倍率が異なるモニター間でのウィンドウ移動に対応

対応OSはWindows 10 / 11です。

Windows実行ファイルの会社名は`Alcnaka`、製品名は`AlcMute`です。
ファイルのバージョン情報からも確認できます。

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

設定の変更に失敗した場合は画面で案内します。必要に応じて設定を再保存するか、
AlcMuteを再起動してください。

同じユーザー・Windowsセッションで二重起動すると通知を表示します。
起動済みのアプリは通知領域から操作してください。起動に失敗した場合は、
エラー概要と`error.log`の保存先をダイアログに表示します。ログは設定と同じフォルダーです。
設定ファイルが破損している場合は自動で削除せず停止します。ファイルをバックアップしてから
内容を修正するか、名前を変更して初期設定で起動してください。

ミュート状態を取得できない場合も、トレイ・設定画面・ショートカットから切り替えを操作できます。
状態が不明なときはミュート済みとは表示しません。VRChat未起動時も操作できますが、
実際に反映するにはVRChat側のOSCと「Toggle Voice」が必要です。
短時間に操作が集中した場合は、少し待ってから操作するよう状態欄に表示します。

## 設定ファイル

既定の保存先は次のとおりです。

```text
%LOCALAPPDATA%\Alcnaka\AlcMute\settings.toml
```

旧保存先`%LOCALAPPDATA%\VRChatMuteController`の設定は、初回起動時に引き継ぎます。
新保存先に設定がある場合はそちらを優先し、旧ファイルはバックアップとして残します。
自動起動の設定も引き継ぎます。旧版を終了してから新しいEXEを起動してください。
別の保存先を使う場合は`alcmute.exe --data-dir <フォルダー>`を指定します。
初期値の項目は保存を省略するため、全項目が初期値なら設定ファイルは存在しません。

## 配布

[GitHub Releases](https://github.com/alcnaka/AlcMute/releases)からWindows向けの
ポータブル版をダウンロードできます。ZIPを展開し、`alcmute.exe`を起動してください。
インストーラーは今後対応予定です。

変更内容は[変更履歴（CHANGELOG.md）](CHANGELOG.md)を参照してください。

## ライセンス

AlcMuteの独自コードは[MIT License](LICENSE)です。
依存ライブラリには、それぞれのライセンスが適用されます。
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)と各ライセンスファイルを参照してください。

実行ファイルにはMIT本文と依存ライブラリのライセンス情報を埋め込み、
トレイメニューまたは設定画面の「AlcMuteについて」から表示できます。
