# AlcMute（あるみゅーと）

**VRC Background Mute — VRChatが非アクティブでも、ショートカットでマイクのミュートを切り替え。**

Windows 10 / 11向けの常駐アプリです。VRChat公式OSCを使い、通知領域（システムトレイ）からの操作にも対応しています。

## ダウンロード

**[最新版をダウンロードする](https://github.com/alcnaka/AlcMute/releases/latest)**

迷ったらInstaller版を選んでください。

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

ショートカットは左右の修飾キーの指定や、`K + L`などの同時押しにも対応しています。設定画面ではトレイのダブルクリック動作や、ライト／ダークテーマも変更できます。

## 詳しい使い方

- [インストール・更新](docs/user-guide.md#インストールと更新)
- [ショートカット・設定・トレイ操作](docs/user-guide.md#基本操作)
- [設定ファイルの場所・バックアップ](docs/user-guide.md#設定ファイル)
- [困ったとき](docs/user-guide.md#困ったとき)
- [変更履歴](CHANGELOG.md)

## ライセンス

制作：**Alcnaka**。AlcMuteの独自コードは[MIT License](LICENSE)です。
依存ライブラリのライセンスは[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)と、アプリの「AlcMuteについて」から確認できます。

<a href="https://slint.dev/">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="./assets/img/MadeWithSlint-logo-dark.svg">
    <source media="(prefers-color-scheme: light)" srcset="./assets/img/MadeWithSlint-logo-light.svg">
    <img alt="#MadeWithSlint" src="./assets/img/MadeWithSlint-logo-light.svg" width="106" height="49">
  </picture>
</a>
