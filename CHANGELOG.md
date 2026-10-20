# Changelog

AlcMuteの利用者に関わる主な変更を、このファイルに記録します。

形式は[Keep a Changelog](https://keepachangelog.com/ja/1.1.0/)に基づき、
バージョン番号は[Semantic Versioning](https://semver.org/lang/ja/)に従います。
日付は日本時間の公開日を`YYYY-MM-DD`で記載します。

## [Unreleased]

### Added（追加）

- トレイメニューに「AlcMuteについて」を追加。制作者Alcnaka、リポジトリへのリンク、MITライセンス本文を表示。
- 依存ライブラリのライセンス情報を実行ファイルに埋め込み、アプリ内で表示。
- この変更履歴と、READMEからのリンクを追加。

### Changed（変更）

- 設定の既定保存先を`%LOCALAPPDATA%\VRChatMuteController`から`%LOCALAPPDATA%\Alcnaka\AlcMute`に変更。旧設定は初回起動時に引き継ぎ、旧ファイルはバックアップとして保持。新しい設定を優先し、全初期化後の再取り込みを防止。
- Windows自動起動の登録名を`VRChatMuteController`から`AlcMute`に変更。旧名のエントリがある場合は新名へ移行してから削除。旧版の自動起動に含まれる旧既定保存先の指定も、新保存先へ切り替え。

### Fixed（修正）

- AlcMuteのMITライセンスの著作権者表記を`Copyright (c) 2026 Alcnaka`に変更。
- 長い依存ライセンスの表示で発生したクラッシュへの対策として、ライブラリ選択とページ分割を導入。一度に表示する本文を最大2,048文字・48改行に制限し、本文を省略せず閲覧できるように変更。

## [0.1.0] - 2026-10-21

AlcMute（あるみゅーと / VRC Background Mute）の初回リリース。

### Added（追加）

- VRChat公式OSCを使ったマイクのミュート切り替えと状態取得。
- VRChatが非アクティブでも使えるグローバルショートカット。
- 左右の修飾キーの区別、修飾キーのみの組み合わせ、`K + L`などの複数キーによるショートカット登録。
- ミュート状態に応じたトレイアイコンと、ミュート切り替え・設定・終了のトレイメニュー。
- トレイアイコンのダブルクリック動作を、ミュート切り替えと設定画面表示から選択する設定。
- キーを押してショートカットを記録できる設定画面と、未保存の変更の表示。
- 項目ごとのリセットと、確認付きの全初期化。全初期化ではWindowsの自動起動も解除。
- Windowsサインイン時の自動起動と、起動時に設定画面を表示するかどうかの設定。
- 異なるDPIのモニター間でのウィンドウ移動・描画への対策。
- Windows 10 / 11のx64向けポータブル版ZIPによる配布。

[Unreleased]: https://github.com/alcnaka/AlcMute/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/alcnaka/AlcMute/releases/tag/v0.1.0
