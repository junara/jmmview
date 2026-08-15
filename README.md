# jmmview

Claude Code が出力する Mermaid 図を、**Node.js・Chromium・CDN なし**で綺麗にレンダリングする Rust 製 CLI。

[merman](https://github.com/Latias94/merman)(Mermaid のヘッドレス Rust 実装、Zed でも採用)をレンダリングエンジンに使用しています。シークエンス図を含む Mermaid の全図種に対応します。

## できること

- **`render`** — `.mmd` / 標準入力の Mermaid ソースを SVG / PNG / JPEG / PDF に変換
- **`html`** — HTML に埋め込まれた Mermaid ブロックをインライン SVG に置換し、mermaid の CDN `<script>` タグを除去(オフラインで表示可能に)
- **`export`** — Markdown / HTML 内の全 Mermaid ブロックを一括で SVG / PNG ファイルに書き出し
- **`ascii`** — ターミナルに Unicode 罫線でプレビュー
- **`themes`** — 利用可能なテーマの一覧

## インストール

```bash
cargo install --path .
```

## 使い方

### 単体レンダリング(SVG / PNG / JPEG / PDF)

```bash
# 拡張子から形式を自動判別
jmmview render diagram.mmd -o diagram.svg
jmmview render diagram.mmd -o diagram.png --scale 2          # retina 品質
jmmview render diagram.mmd -o diagram.png --background white # white/black/transparent/#RRGGBB

# 標準入力 → 標準出力(パイプ利用)
echo 'sequenceDiagram
    A->>B: hello' | jmmview render > out.svg
```

### HTML の Mermaid をインライン SVG 化(CDN 依存を除去)

```bash
jmmview html report.html -o report-offline.html
jmmview html report.html --in-place           # 上書き
jmmview html report.html --keep-source        # 元ソースを折りたたみで残す
jmmview html report.html --strict             # レンダリング失敗時に exit 1
```

対応パターン: `<pre class="mermaid">`、`<div class="mermaid">`、`<pre><code class="language-mermaid">`(class はトークン一致。`mermaid-wrapper` などは誤検出しません)。`<script>` 内や HTML コメント内の mermaid 風テキストは変更されません。
HTML エンティティ(`--&gt;` など)は自動でデコードされます。jsdelivr / unpkg などの mermaid CDN スクリプトと `mermaid.initialize` / `mermaid.run` / ESM import の初期化スクリプトは削除されます。ただしレンダリングに失敗したブロックがある場合は、ブラウザで表示できるよう mermaid スクリプトを残します。

### Markdown / HTML から一括書き出し

```bash
jmmview export design-doc.md -d diagrams/ -f png --scale 2
jmmview export report.html -d out/ -f svg
jmmview export design-doc.md --json          # Claude Code などから使いやすい JSON 出力
jmmview export design-doc.md --strict        # 1件でも失敗したら exit 1(CI 向け)
```

### ターミナルプレビュー

```bash
jmmview ascii diagram.mmd
cat diagram.mmd | jmmview ascii
```

### テーマ

```bash
jmmview themes                                        # 一覧
jmmview --theme dark render seq.mmd -o seq.svg        # Mermaid 標準テーマ
jmmview --host-theme one-dark render seq.mmd -o s.png # エディタ風テーマ
```

- `--theme`: default / base / dark / forest / neutral / neo / neo-dark / redux 系
- `--host-theme`: editor-light / editor-dark / one-dark / gruvbox-light / gruvbox-dark / ayu-light / ayu-dark

## Claude Code との連携

- 標準入力・標準出力対応なのでパイプで直接つなげます
- `export --json` は生成ファイル一覧を JSON で返すため、機械的に扱えます(ブロックが見つからない場合も JSON を出力して exit 1)
- 失敗はデフォルトで警告のみ(元ブロックを保持)、`html --strict` / `export --strict` で exit code に反映

`CLAUDE.md` に次のように書いておくと、Claude Code が HTML 出力後に自動で図をインライン化できます:

```markdown
- HTML レポートを生成したら `jmmview html <file> --in-place` を実行して
  Mermaid 図をインライン SVG 化すること(CDN 非依存にするため)
```

## 開発

```bash
cargo test
cargo build --release
```

注意: 依存クレート `roughr-merman` は 0.7 系 merman との互換性のため `0.12.0` に固定しています(`Cargo.lock`)。`cargo update` で `roughr-merman` が 0.12.2 以降に上がるとビルドが壊れます。
