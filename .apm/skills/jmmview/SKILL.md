---
name: jmmview
description: >-
  Mermaid図をNode.js・Chromium・CDNなしでレンダリングするCLI「jmmview」の使い方ガイド。
  Mermaid記法の図(シーケンス図・フローチャート等)をSVG/PNG/JPEG/PDFに変換したいとき、
  HTMLに埋め込まれたmermaidをインラインSVG化してオフラインでも表示できるようにしたいとき、
  Markdown/HTML内の図を一括で画像ファイルに書き出したいとき、図をターミナルでプレビュー
  したいときは必ずこのスキルを使うこと。ユーザーが「mermaidをSVGに」「シーケンス図を画像化」
  「図をPNGで共有したい」「HTMLのmermaidが表示されない/CDNなしにしたい」「mermaidを
  ターミナルで確認」などと言った場合、mermaidという単語がなくてもMermaid記法の変換・
  表示・共有が目的ならこのスキルが該当する。
---

# jmmview — Mermaid図のネイティブレンダリング

jmmviewはMermaidのヘッドレスRust実装(merman)を使ったCLI。ブラウザもNodeも
起動しないため高速で、ネットワーク不要。シーケンス図を含むMermaidの全図種に対応。

## 前提確認

バイナリは通常 `~/.cargo/bin/jmmview` にある。PATHに入っていない環境も多いので、
`which`で見つからなくてもまず実体を確認する:

```bash
command -v jmmview || ls ~/.cargo/bin/jmmview
```

- 実体があるのにPATHにない → `~/.cargo/bin/jmmview` とフルパスで呼べばよい(再インストール不要)
- 本当に未インストール → `cargo install --git https://github.com/junara/jmmview --locked`
  (cargo自体も `~/.cargo/bin/cargo` か `/opt/homebrew/opt/rustup/bin/cargo` にあることが多い)

## コマンド早見表

| やりたいこと | コマンド |
|---|---|
| .mmd → SVG/PNG/JPEG/PDF | `jmmview render in.mmd -o out.png`(形式は拡張子から推定) |
| 標準入力から変換 | `echo '...' \| jmmview render > out.svg` |
| HTML内のmermaidをインラインSVG化 | `jmmview html page.html --in-place` |
| Markdown/HTMLから一括書き出し | `jmmview export doc.md -d out/ -f png --json` |
| ターミナルでプレビュー | `jmmview ascii diagram.mmd` |
| テーマ一覧 | `jmmview themes` |

## AIエージェント向けの典型ワークフロー

### 1. 生成したHTMLレポートをオフライン対応にする

mermaidをCDNの`<script>`で表示するHTMLを生成・受領したら、仕上げに実行する:

```bash
jmmview html report.html --in-place
```

mermaidブロック(`<pre class="mermaid">` / `<div class="mermaid">` /
`<pre><code class="language-mermaid">`)がインラインSVGに置換され、mermaidの
CDN・初期化スクリプトが削除される。HTMLエンティティ(`--&gt;`等)は自動デコード。
`--keep-source`を付けると元ソースが折りたたみ表示で残る。

注意: レンダリングに失敗したブロックが1つでもあると、CDNスクリプトは削除されない
(ブラウザ側でのレンダリング手段を残すため)。stderrの警告を確認すること。

### 2. 図を画像ファイルとして共有する

```bash
jmmview render diagram.mmd -o diagram.png --scale 2 --background white
```

- `--scale 2` でretina品質(既定値)
- `--background` は `white` / `black` / `transparent` / `#RRGGBB`(`#RGB`/`#RGBA`/`#RRGGBBAA`も可)のみ。
  `lightgray`のようなCSS色名は使えない(エラーになる)
- 出力拡張子が `.svg`/`.png`/`.jpg`/`.pdf` 以外だとエラー。`--format`で明示可能

### 3. ドキュメント内の全図を一括変換する

```bash
jmmview export design.md -d diagrams/ -f png --json
```

`--json` の出力(stdout)をパースして生成ファイルを把握する:

```json
{ "files": ["diagrams/design-1.png"], "rendered": 1, "failed": [] }
```

- 標準入力を使う場合は種類を明示: `... | jmmview export - --stdin-format html`(md/html/mmd)
- CIや自動化では `--strict` を付けると1件でも失敗したらexit 1になる
- PNGの既定背景は透明。チャットやスライドでの共有用途なら `--background white` を推奨

### 4. レンダリング可否を素早く確認する

ファイルを作らず内容を検証したいときは`ascii`が最速:

```bash
echo 'sequenceDiagram
    A->>B: hi' | jmmview ascii
```

成功すればUnicode罫線の図が出る。ascii対応はflowchart/sequence/class/er/xychartの
5種のみ(SVG/PNG等は全図種対応なので、ascii非対応でもrenderは通ることがある)。

## テーマ

```bash
jmmview --theme dark render in.mmd -o out.svg        # Mermaid標準テーマ
jmmview --host-theme one-dark render in.mmd -o out.png --background '#282c34'
```

`--theme`: default / base / dark / forest / neutral / neo / neo-dark / redux系。
`--host-theme`: editor-light / editor-dark / one-dark / gruvbox-light / gruvbox-dark /
ayu-light / ayu-dark(エディタ配色。`--theme`とは排他)。

## エラー処理

- exit 0 = 成功(export/htmlは一部失敗でも警告のみで0。厳格にするなら`--strict`)
- exit 1 = 失敗。エラーはstderrに`error: ...`形式
- 「not recognized as a mermaid diagram」= 入力がMermaid記法として検出できない。
  先頭行に図種宣言(`sequenceDiagram`、`flowchart TD`等)があるか確認する
- 「renderer crashed (merman bug?)」= レンダラ内部のパニックを捕捉した(既知例:
  xychartのx軸ラベルにマルチバイト文字)。該当ブロックだけ失敗扱いになり処理は
  継続する。回避策: xychartの軸ラベルは英数字にする
