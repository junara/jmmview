---
description: Batch-export every mermaid diagram in a Markdown or HTML document to shareable image files with jmmview.
author: junara
input:
  - source_file
  - output_dir
---

# ドキュメント内の図を画像ファイルに一括書き出し

Markdown や HTML に書かれた Mermaid 図を、Slack・wiki・スライドに貼れる画像
ファイルとして書き出します。

## 前提

```bash
command -v jmmview || ls ~/.cargo/bin/jmmview
```

未インストールなら `cargo install --git https://github.com/junara/jmmview --locked`。

## 手順

1. `{{source_file}}` の全 Mermaid ブロックを `{{output_dir}}` に書き出します。
   共有用途なら背景を白にしておくと、ダークテーマのチャットでも図が潰れません
   (PNG の既定は透過背景です)。

   ```bash
   jmmview export {{source_file}} -d {{output_dir}} -f png --scale 2 --background white --json
   ```

2. `--json` の出力(stdout)から生成ファイルを確認します:

   ```json
   { "files": ["..."], "rendered": 2, "failed": [] }
   ```

   `failed` が空でなければ、その図のソースを修正して再実行します。

3. 生成された画像を目視で確認し、ラベルの欠けや文字化けがないかを見ます。

## 形式の選び方

| 用途 | 指定 |
|---|---|
| チャット・wiki・スライド | `-f png --background white`(`--scale 2` で retina 品質) |
| ドキュメントへの埋め込み・拡大表示 | `-f svg`(ベクタなので劣化しない) |
| 印刷・配布資料 | `-f pdf` |

## 注意

- ファイル名は `<入力ファイル名>-1.png`, `-2.png` … になります。`--stem NAME` で変更可能。
- CI で使う場合は `--strict` を付けると、1 件でも失敗したときに exit 1 になります。
- 標準入力から渡す場合は種類の明示が必要です: `... | jmmview export - --stdin-format md`
