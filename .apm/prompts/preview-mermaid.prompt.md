---
description: Validate and preview a Mermaid diagram in the terminal with jmmview before committing it or exporting images.
author: junara
input:
  - mermaid_source
---

# Mermaid 図をターミナルで検証・プレビュー

書いた Mermaid 記法が実際に図として成立するかを、ブラウザを開かずに確認します。
ドキュメントにコミットする前や、画像を書き出す前の事前チェックに向いています。

## 前提

```bash
command -v jmmview || ls ~/.cargo/bin/jmmview
```

未インストールなら `cargo install --git https://github.com/junara/jmmview --locked`。

## 手順

1. Mermaid ソース `{{mermaid_source}}` をターミナルにプレビューします。
   ファイルでも標準入力でも渡せます:

   ```bash
   jmmview ascii {{mermaid_source}}
   ```

   ```bash
   echo 'sequenceDiagram
       A->>B: リクエスト
       B-->>A: レスポンス' | jmmview ascii
   ```

2. Unicode 罫線の図が表示されれば、記法として有効です。

3. エラーが出た場合の読み方:

   - `not recognized as a mermaid diagram` — 図種の宣言が無いか誤っています。
     先頭行が `sequenceDiagram` / `flowchart TD` / `classDiagram` などになっているか確認します。
   - `renderer crashed (merman bug?)` — レンダラ内部の既知の不具合を捕捉した状態です
     (例: xychart の x 軸ラベルにマルチバイト文字)。軸ラベルを英数字にすると回避できます。

## 注意

`ascii` が対応するのは flowchart / sequence / class / er / xychart の 5 種類です。
それ以外の図種は `ascii` では出せませんが、SVG や PNG への変換(`jmmview render`)は
全図種で使えるので、検証は `jmmview render <file> -o /dev/null` でも代用できます。
