---
description: Inline every mermaid block of an HTML report as SVG with jmmview so the page renders offline, and drop its mermaid CDN scripts.
author: junara
input:
  - html_file
---

# HTML レポートをオフライン対応にする

`jmmview` を使って、HTML に埋め込まれた Mermaid 図をインライン SVG に置き換え、
mermaid の CDN 依存を取り除きます。ネットワークのない環境や、社内配布・アーカイブ
用途で「開いても図が真っ白」になるのを防ぐのが目的です。

## 前提

`jmmview` が使えることを確認します。PATH にない環境も多いので、実体も確認します:

```bash
command -v jmmview || ls ~/.cargo/bin/jmmview
```

未インストールなら `cargo install --git https://github.com/junara/jmmview --locked`。

## 手順

1. 対象ファイル `{{html_file}}` を書き換えます。元ファイルを残したい場合は
   `--in-place` の代わりに `-o` で別名出力にします。

   ```bash
   jmmview html {{html_file}} --in-place
   ```

   元の mermaid ソースも残したい場合は `--keep-source` を付けます(折りたたみ表示)。

2. stderr の結果行を確認します。`N diagram(s) inlined, M mermaid script tag(s) removed`
   と出れば成功です。

3. 警告が出た場合は、レンダリングに失敗したブロックがあります。この場合 jmmview は
   **CDN スクリプトをあえて残します**(ブラウザ側で描画できる手段を奪わないため)。
   警告メッセージのブロック番号を手掛かりに Mermaid ソースを修正し、再実行してください。

4. 検証します。CDN 参照が消え、SVG が入ったことを確認します:

   ```bash
   grep -c '<svg' {{html_file}}
   grep -ciE 'cdn|mermaid\.min\.js|mermaid\.initialize' {{html_file}}
   ```

   前者が図の数、後者が 0 になっていれば完了です。

## 注意

- CI で確実に失敗させたい場合は `--strict` を付けます(1 件でも失敗したら exit 1)。
- `<script>` 内や HTML コメント内の mermaid 風テキストは変更されません(意図的な仕様)。
