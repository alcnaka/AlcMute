# AlcMute（あるみゅーと）

**日本語** | [English](README.en.md)

**VRC Background Mute — VRChatが非アクティブでも、ショートカットでマイクのミュートを切り替え。**

Windows 10 / 11向けの常駐アプリです。VRChat公式OSCを使い、通知領域（システムトレイ）からの操作にも対応しています。

## ダウンロード

**[最新版をダウンロードする](https://github.com/alcnaka/AlcMute/releases/latest)**

迷ったらInstaller版を選んでください。

**ダウンロードや起動で警告が出たら**：[警告ごとの操作手順](docs/user-guide.md#セキュリティ機能でブロックされる)を確認してください。Edgeでの保存方法や、起動をブロックされた場合の確認先を案内しています。

| | Installer版 | Portable版 |
| --- | --- | --- |
| 配布ファイル | `AlcMute-…-Setup.exe` | `AlcMute-…-Portable.zip` |
| 使い始め方 | Setupを実行 | ZIPを展開して`alcmute.exe`を実行 |
| Windowsサインイン時の自動起動 | 対応 | 非対応 |
| 更新 | アプリ内更新に対応 | 手動で差し替え |

## 使い始める

1. VRChatで**OSC**と**Toggle Voice**を有効にします。
2. AlcMuteを起動します。Installer版はスタートメニューから開けます。
3. **Ctrl + Alt + M**でミュートを切り替えます。キーは設定画面で変更できます。

設定画面の「閉じる」や×を押しても、アプリはトレイに常駐します。終了するにはトレイメニューの「終了」を選びます。

表示言語は、設定画面の「言語」で自動・日本語・Englishから選び、「保存」で変更できます。

## 詳しい使い方

- [インストール・更新](docs/user-guide.md#インストールと更新)
- [ショートカット・設定・トレイ操作](docs/user-guide.md#基本操作)
- [設定の変更](docs/user-guide.md#設定を変更する)
- [困ったとき](docs/user-guide.md#困ったとき)
- [変更履歴](CHANGELOG.md)

## ライセンス

制作：**Alcnaka** / [MIT License](LICENSE)。ライセンス情報はアプリの「AlcMuteについて」でも確認できます。

<a href="https://slint.dev/">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="./assets/img/MadeWithSlint-logo-dark.svg">
    <source media="(prefers-color-scheme: light)" srcset="./assets/img/MadeWithSlint-logo-light.svg">
    <img alt="#MadeWithSlint" src="./assets/img/MadeWithSlint-logo-light.svg" width="106" height="49">
  </picture>
</a>
