# ドキュメントの生成

原稿は`docs/user-guide.md`です。WebとPDFを個別に編集せず、このMarkdownを更新します。画像も原稿から相対パスで参照します。

## Windowsで生成

Python 3.11以上を用意し、リポジトリのルートで実行します。

```powershell
python -m venv .venv-docs
./.venv-docs/Scripts/python.exe -m pip install -r scripts/docs/requirements.txt
./.venv-docs/Scripts/python.exe scripts/docs/build.py
./.venv-docs/Scripts/python.exe scripts/docs/verify.py
```

- Web：`output/site/index.html`（直接開いて閲覧可能）
- PDF：`output/pdf/AlcMute-Manual.pdf`

Webにも同じPDFがコピーされます。検索・目次・ライト／ダーク表示は外部サービスなしで動作します。

PDFにはWindowsのMSゴシックを埋め込みます。フォントがない環境では`--font <日本語対応TTF/TTC>`を指定してください。フォントを変更・再配布する場合は、その利用条件を確認してください。リポジトリにはフォントファイルを含めません。

## 確認と公開

GitHub Actionsの`Documentation`はWebとPDFを生成し、`alcmute-documentation`成果物として保存します。公開・Release添付は自動で行いません。

検証スクリプトはローカルリンク・目次の参照先、日本語フォントの埋め込み、PDFとMarkdown本文のテキスト一致、Webに置いたPDFとの一致を確認します。

Web公開時は`output/site/`全体を静的ホスティングへ配置できます。GitHub Pagesを使う場合も同じフォルダーが対象です。公開先の設定は別途行います。

生成物はGit管理対象外です。配布前にはPDF全ページの改行・表・画像と、Webの目次・リンク・狭い画面での表示を確認してください。ソースのCargoバージョンを表示しますが、原稿が開発中の変更を含む場合もあるため、公開版との差を原稿に明記します。

## 対応するMarkdown

見出し、段落、番号付き／箇条書きリスト、表、コードブロック、強調、リンク、ローカル画像に対応します。生HTMLは実行しません。PDFで未対応の記法はエラーにし、本文を黙って落とさないようにしています。
