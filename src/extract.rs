//! MarkdownとHTMLからのmermaidブロック抽出、およびHTML書き換え。

use regex::Regex;
use std::ops::Range;
use std::sync::LazyLock;

/// ドキュメント中で見つかった1つのmermaidブロック。
#[derive(Debug, Clone)]
pub struct Block {
    pub source: String,
}

/// HTMLエンティティ(`&gt;`、`&mdash;`、数値参照など)をデコードする。
/// merman-core経由で既に同梱されているhtmlizeの完全なエンティティ表を使う。
pub fn decode_html_entities(input: &str) -> String {
    htmlize::unescape(input).into_owned()
}

pub fn escape_html(s: &str) -> String {
    htmlize::escape_text(s).into_owned()
}

/// Markdownから```mermaidフェンスブロックを抽出する(`~~~`フェンスも受け付ける)。
pub fn extract_from_markdown(md: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut lines = md.lines();
    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        let fence_char = match trimmed.chars().next() {
            Some(c @ ('`' | '~')) => c,
            _ => continue,
        };
        let fence_len = trimmed.chars().take_while(|&c| c == fence_char).count();
        if fence_len < 3 {
            continue;
        }
        let closes_fence = |l: &str| {
            let t = l.trim_start();
            let n = t.chars().take_while(|&c| c == fence_char).count();
            n >= fence_len && t[n..].trim().is_empty()
        };
        let info = trimmed[fence_len..].trim();
        if !info
            .split_whitespace()
            .next()
            .is_some_and(|w| w.eq_ignore_ascii_case("mermaid"))
        {
            // mermaid以外のコードブロックは末尾まで読み飛ばし、他ブロック内の
            // ネストしたフェンスやmermaidの例を拾わないようにする。
            for inner in lines.by_ref() {
                if closes_fence(inner) {
                    break;
                }
            }
            continue;
        }
        let mut source = String::new();
        for inner in lines.by_ref() {
            if closes_fence(inner) {
                break;
            }
            // 開始フェンスのインデント分だけ取り除く。
            let strip = inner
                .char_indices()
                .take_while(|&(i, c)| i < indent && (c == ' ' || c == '\t'))
                .count();
            source.push_str(&inner[strip..]);
            source.push('\n');
        }
        if !source.trim().is_empty() {
            blocks.push(Block { source });
        }
    }
    blocks
}

// <pre class="mermaid">、<div class="mermaid">、および
// <pre><code class="language-mermaid">(Markdownコンバータの出力)に対応。
// class値はトークン完全一致("mermaid-wrapper"にマッチしてはならない)、
// 属性名の直前には空白を要求("data-class"にマッチしてはならない)。
// regexクレートは後方参照を持たないため、preとdivは別の選択肢に分ける。
static BLOCK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r#"(?is)<pre\b[^>]*\sclass\s*=\s*["'](?:[^"']*\s)?mermaid(?:\s[^"']*)?["'][^>]*>(?P<pre_body>.*?)</pre\s*>"#,
        r#"|<div\b[^>]*\sclass\s*=\s*["'](?:[^"']*\s)?mermaid(?:\s[^"']*)?["'][^>]*>(?P<div_body>.*?)</div\s*>"#,
        r#"|<pre\b[^>]*>\s*<code\b[^>]*\sclass\s*=\s*["'](?:[^"']*\s)?(?:language-|lang-)mermaid(?:\s[^"']*)?["'][^>]*>(?P<code>.*?)</code\s*>\s*</pre\s*>"#,
    ))
    .expect("valid mermaid block regex")
});

static SCRIPT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<script\b[^>]*>.*?</script\s*>").expect("valid regex"));

static COMMENT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<!--.*?-->").expect("valid regex"));

static INNER_TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)</?(?:code|span)\b[^>]*>|<br\s*/?>").expect("valid regex"));

// mermaidを読み込むscript要素とは: src属性のURLにmermaidを含むもの、
// mermaid.initialize()/mermaid.run()を呼ぶもの、またはmermaidモジュールを
// ESM importするもの。単語の共起だけ("important" + コメント中の"mermaid"
// など)でマッチしてはならない。
static MERMAID_SRC_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)^<script\b[^>]*\ssrc\s*=\s*["'][^"']*mermaid[^"']*["']"#)
        .expect("valid regex")
});

static MERMAID_CALL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bmermaid\s*\.\s*(?:initialize|run|init)\s*\(").expect("valid regex")
});

static MERMAID_IMPORT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r#"(?is)\bimport\b[^;()]{0,300}?["'][^"']*mermaid[^"']*["']"#,
        r#"|\bimport\s*\(\s*["'][^"']*mermaid[^"']*["']"#,
    ))
    .expect("valid regex")
});

fn strip_inner_markup(body: &str) -> String {
    let no_tags = INNER_TAG_RE.replace_all(body, |caps: &regex::Captures| {
        if caps[0].len() >= 3 && caps[0][..3].eq_ignore_ascii_case("<br") {
            "\n"
        } else {
            ""
        }
    });
    decode_html_entities(&no_tags)
}

fn block_source(caps: &regex::Captures) -> String {
    let body = caps
        .name("pre_body")
        .or_else(|| caps.name("div_body"))
        .or_else(|| caps.name("code"))
        .map(|m| m.as_str())
        .unwrap_or_default();
    strip_inner_markup(body)
}

/// ブラウザがマークアップとして解釈しない領域(`<script>`本文とHTMLコメント)の
/// バイト範囲。その中のmermaid風テキスト(JSテンプレートやコメントアウトされた
/// 例)は変更してはならない。
fn protected_spans(html: &str) -> Vec<Range<usize>> {
    let mut spans: Vec<Range<usize>> = SCRIPT_RE
        .find_iter(html)
        .chain(COMMENT_RE.find_iter(html))
        .map(|m| m.range())
        .collect();
    spans.sort_by_key(|r| r.start);
    spans
}

fn is_protected(spans: &[Range<usize>], pos: usize) -> bool {
    spans.iter().any(|r| r.contains(&pos))
}

/// HTMLを変更せずにmermaidソースだけを抽出する。
pub fn extract_from_html(html: &str) -> Vec<Block> {
    let protected = protected_spans(html);
    BLOCK_RE
        .captures_iter(html)
        .filter(|caps| !is_protected(&protected, caps.get(0).unwrap().start()))
        .map(|caps| Block {
            source: block_source(&caps),
        })
        .filter(|b| !b.source.trim().is_empty())
        .collect()
}

/// `<script>`要素がmermaidの読み込み・初期化を行う場合にtrueを返す。
fn is_mermaid_script(script: &str) -> bool {
    MERMAID_SRC_RE.is_match(script)
        || MERMAID_CALL_RE.is_match(script)
        || MERMAID_IMPORT_RE.is_match(script)
}

/// HTMLドキュメント書き換えの結果。
pub struct RewriteReport {
    pub html: String,
    pub rendered: usize,
    pub failed: Vec<String>,
    pub scripts_removed: usize,
}

/// `html`中の全mermaidブロックを`render`が生成したSVGに置き換え、ページが
/// ネットワークに依存しないようmermaidのCDN/初期化`<script>`要素を削除する。
///
/// スクリプトの削除は全ブロックのレンダリングに成功した場合のみ行う。失敗して
/// 生のmermaidソースのまま残したブロックがある場合は、ブラウザ側でレンダリング
/// できるようmermaidスクリプトを残す。
///
/// `render`は(連番, mermaidソース)を受け取り、SVGマークアップを返す。
/// ソースが有効な図でない場合はNoneを返す。
pub fn rewrite_html(
    html: &str,
    keep_source: bool,
    mut render: impl FnMut(usize, &str) -> anyhow::Result<Option<String>>,
) -> RewriteReport {
    let mut rendered = 0usize;
    let mut failed = Vec::new();
    let mut index = 0usize;

    let protected = protected_spans(html);
    let replaced = BLOCK_RE.replace_all(html, |caps: &regex::Captures| {
        let whole = caps.get(0).unwrap();
        if is_protected(&protected, whole.start()) {
            return whole.as_str().to_string();
        }
        let source = block_source(caps);
        if source.trim().is_empty() {
            return whole.as_str().to_string();
        }
        index += 1;
        match render(index, &source) {
            Ok(Some(svg)) => {
                rendered += 1;
                let mut out = format!("<figure class=\"jmmview-diagram\">{svg}");
                if keep_source {
                    out.push_str(&format!(
                        "<details><summary>mermaid source</summary><pre>{}</pre></details>",
                        escape_html(&source)
                    ));
                }
                out.push_str("</figure>");
                out
            }
            Ok(None) => {
                failed.push(format!("block {index}: not recognized as a mermaid diagram"));
                whole.as_str().to_string()
            }
            Err(err) => {
                failed.push(format!("block {index}: {err:#}"));
                whole.as_str().to_string()
            }
        }
    });

    let mut scripts_removed = 0usize;
    let mut html = if failed.is_empty() {
        let comments: Vec<Range<usize>> =
            COMMENT_RE.find_iter(&replaced).map(|m| m.range()).collect();
        SCRIPT_RE
            .replace_all(&replaced, |caps: &regex::Captures| {
                let whole = caps.get(0).unwrap();
                if !is_protected(&comments, whole.start()) && is_mermaid_script(whole.as_str()) {
                    scripts_removed += 1;
                    String::new()
                } else {
                    whole.as_str().to_string()
                }
            })
            .into_owned()
    } else {
        replaced.into_owned()
    };

    if rendered > 0 {
        html = inject_style(&html);
    }

    RewriteReport {
        html,
        rendered,
        failed,
        scripts_removed,
    }
}

const STYLE: &str = "<style id=\"jmmview-style\">\
.jmmview-diagram{margin:1.5em auto;text-align:center;overflow-x:auto;}\
.jmmview-diagram>svg{max-width:100%;height:auto;}\
.jmmview-diagram details{text-align:left;font-size:0.85em;margin-top:0.5em;}\
.jmmview-diagram details pre{background:rgba(127,127,127,0.1);padding:0.75em;border-radius:6px;overflow-x:auto;}\
</style>";

static HEAD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)</head\s*>").expect("valid regex"));

fn inject_style(html: &str) -> String {
    if html.contains("id=\"jmmview-style\"") {
        return html.to_string();
    }
    if let Some(m) = HEAD_RE.find(html) {
        let mut out = String::with_capacity(html.len() + STYLE.len());
        out.push_str(&html[..m.start()]);
        out.push_str(STYLE);
        out.push_str(&html[m.start()..]);
        out
    } else {
        format!("{STYLE}{html}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_ok(_: usize, _: &str) -> anyhow::Result<Option<String>> {
        Ok(Some("<svg>ok</svg>".into()))
    }

    #[test]
    fn decodes_entities_including_named_table() {
        assert_eq!(
            decode_html_entities("A--&gt;B &amp; C &#65; &#x42; &mdash;"),
            "A-->B & C A B \u{2014}"
        );
    }

    #[test]
    fn extracts_markdown_fences() {
        let md = "# t\n```mermaid\nsequenceDiagram\nA->>B: hi\n```\n```rust\nfn x(){}\n```\n";
        let blocks = extract_from_markdown(md);
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].source.contains("sequenceDiagram"));
    }

    #[test]
    fn skips_mermaid_examples_inside_other_fences() {
        let md = "````md\n```mermaid\nflowchart TD\n```\n````\n";
        assert!(extract_from_markdown(md).is_empty());
    }

    #[test]
    fn extracts_pre_mermaid_from_html() {
        let html = r#"<pre class="mermaid">graph TD; A--&gt;B;</pre>"#;
        let blocks = extract_from_html(html);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].source, "graph TD; A-->B;");
    }

    #[test]
    fn extracts_language_mermaid_code_from_html() {
        let html = r#"<pre><code class="language-mermaid">graph TD
A --&gt; B</code></pre>"#;
        let blocks = extract_from_html(html);
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].source.contains("A --> B"));
    }

    #[test]
    fn class_matching_is_token_exact() {
        let html = concat!(
            r#"<div class="mermaid-wrapper"><div class="mermaid">graph TD; A;</div></div>"#,
            r#"<div class="mermaid-fallback">not a diagram</div>"#,
            r#"<div data-class="mermaid">not a diagram either</div>"#,
        );
        let blocks = extract_from_html(html);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].source, "graph TD; A;");
    }

    #[test]
    fn ignores_blocks_inside_scripts_and_comments() {
        let html = concat!(
            r#"<script>var t='<div class="mermaid">graph TD; A</div>';</script>"#,
            r#"<!-- <div class="mermaid">graph TD; B</div> -->"#,
            r#"<div class="mermaid">graph TD; C</div>"#,
        );
        let blocks = extract_from_html(html);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].source, "graph TD; C");

        let report = rewrite_html(html, false, render_ok);
        assert_eq!(report.rendered, 1);
        assert!(report.html.contains(r#"var t='<div class="mermaid">graph TD; A</div>';"#));
        assert!(report.html.contains(r#"<!-- <div class="mermaid">graph TD; B</div> -->"#));
    }

    #[test]
    fn rewrite_replaces_blocks_and_strips_cdn() {
        let html = concat!(
            r#"<html><head><script src="https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.min.js"></script>"#,
            r#"<script>mermaid.initialize({startOnLoad:true});</script></head>"#,
            r#"<body><div class="mermaid">graph TD; A--&gt;B;</div></body></html>"#,
        );
        let report = rewrite_html(html, false, render_ok);
        assert_eq!(report.rendered, 1);
        assert_eq!(report.scripts_removed, 2);
        assert!(report.html.contains("<svg>ok</svg>"));
        assert!(!report.html.contains("jsdelivr"));
        assert!(report.html.contains("jmmview-style"));
    }

    #[test]
    fn rewrite_keeps_non_mermaid_scripts() {
        let html = concat!(
            r#"<script>console.log("hi")</script>"#,
            r#"<script>/* important: mermaid docs */ initNav();</script>"#,
            r#"<div class="mermaid">graph TD; A;</div>"#,
        );
        let report = rewrite_html(html, false, render_ok);
        assert_eq!(report.scripts_removed, 0);
        assert!(report.html.contains("console.log"));
        assert!(report.html.contains("initNav"));
    }

    #[test]
    fn rewrite_strips_esm_import_scripts() {
        let html = concat!(
            r#"<script type="module">import mermaid from "https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs";</script>"#,
            r#"<div class="mermaid">graph TD; A;</div>"#,
        );
        let report = rewrite_html(html, false, render_ok);
        assert_eq!(report.scripts_removed, 1);
    }

    #[test]
    fn rewrite_keeps_scripts_when_a_block_fails() {
        let html = concat!(
            r#"<script src="https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.min.js"></script>"#,
            r#"<div class="mermaid">good graph</div>"#,
            r#"<div class="mermaid">bad graph</div>"#,
        );
        let report = rewrite_html(html, false, |_, src| {
            if src.contains("bad") {
                Ok(None)
            } else {
                Ok(Some("<svg/>".into()))
            }
        });
        assert_eq!(report.rendered, 1);
        assert_eq!(report.failed.len(), 1);
        assert_eq!(report.scripts_removed, 0);
        assert!(report.html.contains("jsdelivr"));
    }
}
