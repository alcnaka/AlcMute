# Web・PDFの生成（開発者向け）

共通原稿は`docs/user-guide.md`です。ユーザー向けの操作説明だけを置き、内部仕様・検証記録は`docs/developer-documentation-notes.md`などへ分離します。WebとPDFを別々に編集しません。

## 必要な環境

- Python 3.11以上
- Node.js 24 LTS、pnpm 11.25.0
- Windowsの游ゴシックMedium／Bold（PDF用）

## ビルド

リポジトリのルートで実行します。

```powershell
python -m venv .venv-docs
./.venv-docs/Scripts/python.exe -m pip install -r scripts/docs/requirements.txt
$env:ASTRO_TELEMETRY_DISABLED = '1'
$env:RAYON_NUM_THREADS = '1'
./.venv-docs/Scripts/python.exe scripts/docs/build.py
pnpm --dir website install --frozen-lockfile
pnpm --dir website build
./.venv-docs/Scripts/python.exe scripts/docs/verify.py
```

PythonでPDFとStarlight用Markdownを生成し、Astroで静的サイトをビルドします。

- Web：`output/site/`
- PDF：`output/pdf/AlcMute-Manual.pdf`
- 生成した中間原稿：`website/src/content/docs/`（編集しない）

画像は原稿から相対パスで参照します。本文・見出しの游ゴシックはPDFに埋め込みます。別環境では`--font <本文用TTF/TTC> --bold-font <見出し用TTF/TTC>`を指定できます。フォント本体はリポジトリへ含めません。

## プレビュー

```powershell
pnpm --dir website preview
```

表示されたlocalhostのURLを開きます。Astro版の検索やページ移動を確認する際は、HTMLファイルを直接開かずHTTPで配信してください。開発サーバーは`pnpm --dir website dev`です。原稿を編集したら先にPythonの生成コマンドを再実行します。

Starlight標準の複数ページ、Pagefind検索、目次、ページ送り、モバイルメニュー、ライト／ダーク／自動を利用します。本文は日本語です。

## GitHub Pagesなどのサブパス

プロジェクトサイトで公開する場合は、生成とビルドの両方に同じベースパスを渡します。

```powershell
$env:DOCS_BASE_PATH = '/AlcMute/'
./.venv-docs/Scripts/python.exe scripts/docs/build.py
pnpm --dir website build
./.venv-docs/Scripts/python.exe scripts/docs/verify.py
```

検証では複数ページの内部リンク・目次・画像、PDF本文の収録、フォント埋め込み、配布PDFの一致を確認します。配布前にはPDF全ページのレイアウトも確認します。

## CIと公開

`Documentation`ワークフローで生成・検証し、`alcmute-documentation`成果物を保存します。`output/site/`全体を静的ホスティングに配置できます。サイト公開やRelease添付は自動で行いません。

原稿は標準Markdown（見出し・段落・リスト・表・コード・強調・リンク・画像）に限定します。PDF未対応の記法はエラーにします。WebとPDFの内容を一致させるため、原稿へMDX専用コンポーネントを直接書かないでください。
