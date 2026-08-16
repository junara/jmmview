# jmmview

Claude Code が出力する Mermaid 図を、**Node.js・Chromium・CDN なし**で綺麗にレンダリングする Rust 製 CLI。

[merman](https://github.com/Latias94/merman)(Mermaid のヘッドレス Rust 実装、Zed でも採用)をレンダリングエンジンに使用しています。シークエンス図を含む Mermaid の全図種に対応します。

## できること

- **`render`** — `.mmd` / 標準入力の Mermaid ソースを SVG / PNG / JPEG / PDF に変換
- **`html`** — HTML に埋め込まれた Mermaid ブロックをインライン SVG に置換し、mermaid の CDN `<script>` タグを除去(オフラインで表示可能に)
- **`export`** — Markdown / HTML 内の全 Mermaid ブロックを一括で SVG / PNG ファイルに書き出し
- **`ascii`** — ターミナルに Unicode 罫線でプレビュー
- **`themes`** — 利用可能なテーマの一覧

## merman-cli との違い

jmmview はレンダリングエンジンに [merman](https://github.com/Latias94/merman) を使っています。
merman には公式の CLI である **[merman-cli](https://crates.io/crates/merman-cli)** があり、
「Node・Chromium なしで Mermaid を SVG/PNG/JPEG/PDF に変換する」という基本機能は重なります。
**単体の図を変換したいだけなら merman-cli で十分です。**

jmmview は「HTML と AI エージェント」に用途を絞った別ツールです。主な違い:

| | jmmview | merman-cli |
|---|---|---|
| HTML 内の mermaid をインライン SVG 化 | **できる**(`html`) | できない |
| mermaid の CDN `<script>` を除去 | **できる** | できない |
| Markdown から図を一括書き出し | できる(`export`) | できる(`batch`。書き換え済み Markdown とマニフェストも生成) |
| HTML から図を一括書き出し | **できる** | できない(Markdown のみ) |
| 機械可読な実行結果 | `export --json` | `lint --format json` ほか |
| mermaid-cli(mmdc)互換 | なし | **あり**(`mmdc` サブコマンド) |
| Lint / 自動修正 | なし | **あり**(`lint` / `fix`) |
| パース結果・レイアウトの調査 | なし | **あり**(`detect` / `parse` / `layout`) |
| Rustdoc 連携 | なし | **あり**(`rustdoc`) |
| AI エージェント用スキル同梱 | **あり** | なし |
| バイナリサイズ | 約 21MB | 約 42MB |

要するに:

- **merman-cli を使うべき場合** — mermaid-cli(mmdc)からの移行、Mermaid ソースの lint や自動修正、
  パース結果の調査、Rustdoc への埋め込み、Markdown 中心のワークフロー。機能の幅は merman-cli の方が
  はるかに広く、上流の公式ツールです。
- **jmmview を使うべき場合** — **既存の HTML に埋め込まれた Mermaid をインライン SVG 化して
  オフラインで開けるようにしたい**とき(merman-cli は HTML を扱いません)。AI エージェントに
  図の変換を任せたいとき。

※ 比較は jmmview 0.1.0 と merman-cli 0.8.0-alpha.5 を実際に動かして確認したものです。

## インストール

### 前提

Rust ツールチェーン(**Rust 1.88 以降**、edition 2024)が必要です。未導入の場合:

```bash
# rustup(公式)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

```bash
# または Homebrew(macOS)
brew install rustup && rustup default stable
```

Node.js・Chromium・ネットワーク上の CDN は一切不要です。ビルドに必要なのは Rust だけです。

### GitHub からインストール

```bash
cargo install --git https://github.com/junara/jmmview --locked
```

### ソースからインストール

```bash
git clone https://github.com/junara/jmmview.git
cd jmmview
cargo install --path . --locked
```

`--locked` を推奨します。`Cargo.lock` に記録された依存バージョン(ビルド互換性のための `roughr-merman = 0.12.0` ピンを含む)をそのまま使ってビルドします。

バイナリは `~/.cargo/bin/jmmview` に入ります(rustup 導入時に PATH へ自動追加されます)。

### 動作確認

```bash
jmmview --help
```

```bash
echo 'sequenceDiagram
    A->>B: hello
    B-->>A: world' | jmmview ascii
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

## AI エージェントとの連携

- 標準入力・標準出力対応なのでパイプで直接つなげます
- `export --json` は生成ファイル一覧を JSON で返すため、機械的に扱えます(ブロックが見つからない場合も JSON を出力して exit 1)
- 失敗はデフォルトで警告のみ(元ブロックを保持)、`html --strict` / `export --strict` で exit code に反映

エージェントに使い方を教えるスキルを同梱しています。導入方法は次の [Agent Skill](#agent-skill) を参照してください。

## Agent Skill

AI エージェント(Claude Code など)が jmmview を正しく使えるようにするスキルを同梱しています
([`skills/jmmview/SKILL.md`](skills/jmmview/SKILL.md))。コマンド早見表、典型ワークフロー、
テーマ、エラー処理と既知の制約が書かれており、エージェントは「この図を PNG にして」程度の
指示から適切なコマンドを組み立てられます。

導入方法は 3 通りあります。

### Claude Code プラグイン(マーケットプレイス)

Claude Code 内でそのまま実行してください:

```
/plugin marketplace add junara/jmmview
```

```
/plugin install jmmview@jmmview
```

スキルが導入され(`/jmmview:jmmview` で明示的に呼び出せます)、リポジトリのデフォルト
ブランチに追従します。

### GitHub CLI(`gh skill`)

Claude Code 以外のエージェント(GitHub Copilot、Cursor、Codex、Gemini CLI など)にも
導入できます:

```bash
gh skill install junara/jmmview jmmview --agent claude-code
```

```bash
gh skill install junara/jmmview jmmview --agent github-copilot --scope user
```

```bash
gh skill install junara/jmmview jmmview@v0.1.0 --agent claude-code
```

エージェントごとのディレクトリ(Claude Code なら `.claude/skills/jmmview/`)に配置されます。

### APM パッケージマネージャ

```bash
apm install junara/jmmview
```

スキルに加えて、再利用可能なプロンプト(`inline-mermaid-html`、`export-diagrams`、
`preview-mermaid`)も入ります。

### 手動での導入

上記が使えない場合は、ファイルをコピーするだけでも動きます:

```bash
mkdir -p ~/.claude/skills/jmmview && cp skills/jmmview/SKILL.md ~/.claude/skills/jmmview/
```

### CLAUDE.md への記載

プロジェクトの `CLAUDE.md` に次の一文を入れておくと、Claude Code が HTML 出力のたびに
自動で図をインライン化します:

```markdown
- HTML レポートを生成したら `jmmview html <file> --in-place` を実行して
  Mermaid 図をインライン SVG 化すること(CDN 非依存にするため)
```

### スキルを編集する場合

`skills/jmmview/SKILL.md` が正本です。編集したら APM 配布用の複製を同期してください:

```bash
./scripts/sync-skill.sh
```

## 開発

```bash
cargo test
cargo build --release
```

注意: 依存クレート `roughr-merman` は 0.7 系 merman との互換性のため `0.12.0` に固定しています(`Cargo.lock`)。`cargo update` で `roughr-merman` が 0.12.2 以降に上がるとビルドが壊れます。
