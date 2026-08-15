mod extract;
mod renderer;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use renderer::{Format, RenderOptions, ThemeArgs};

/// jmmview — MermaidをネイティブにレンダリングするCLI(Node・CDN不要)。
///
/// MermaidのヘッドレスRust実装であるmermanを使用。シーケンス図をはじめ
/// 30種類以上の図をSVG/PNG/JPEG/PDFに変換し、HTML内の図をインラインSVG化
/// してオフラインでも表示できるようにし、ターミナルでのプレビューにも対応。
#[derive(Parser)]
#[command(name = "jmmview", version, about, max_term_width = 100)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    #[command(flatten)]
    theme: ThemeArgs,
}

#[derive(Subcommand)]
enum Command {
    /// 1つのmermaidソース(.mmdファイルまたは標準入力)をSVG/PNG/JPEG/PDFに変換
    Render {
        /// 入力.mmdファイル。'-'または省略で標準入力
        input: Option<PathBuf>,

        /// 出力ファイル。形式は拡張子から推定。省略時はSVGを標準出力へ
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// 出力形式(拡張子からの推定を上書き)
        #[arg(short, long, value_enum)]
        format: Option<Format>,

        /// PNG/JPEGの拡大率(2 = retina品質)
        #[arg(long, default_value_t = 2.0)]
        scale: f32,

        /// PNG/JPEGの背景色(例: white, '#0d1117')
        #[arg(long)]
        background: Option<String>,
    },

    /// HTML内のmermaidブロックをインラインSVG化し、mermaidのCDNスクリプトを除去
    Html {
        /// 入力HTMLファイル。'-'で標準入力
        input: PathBuf,

        /// 出力ファイル。省略時は標準出力へ
        #[arg(short, long, conflicts_with = "in_place")]
        output: Option<PathBuf>,

        /// 入力ファイルを直接上書き
        #[arg(short = 'i', long)]
        in_place: bool,

        /// 各図の下に元のmermaidソースを折りたたみ表示で残す
        #[arg(long)]
        keep_source: bool,

        /// レンダリングできないブロックがあれば、そのまま残さずexit 1で失敗させる
        #[arg(long)]
        strict: bool,
    },

    /// Markdown/HTML/.mmd内の全mermaidブロックを抽出し、それぞれファイルに出力
    Export {
        /// 入力.md/.html/.mmdファイル。'-'で標準入力(--stdin-formatが適用される)
        input: PathBuf,

        /// 出力ディレクトリ
        #[arg(short = 'd', long, default_value = ".")]
        out_dir: PathBuf,

        /// 出力形式
        #[arg(short, long, value_enum, default_value = "svg")]
        format: Format,

        /// 出力ファイルのベース名(NAME.svg, NAME-2.svg, ...)。省略時は入力ファイル名から
        #[arg(long)]
        stem: Option<String>,

        /// PNG/JPEGの拡大率(2 = retina品質)
        #[arg(long, default_value_t = 2.0)]
        scale: f32,

        /// PNG/JPEGの背景色(例: white, '#0d1117')
        #[arg(long)]
        background: Option<String>,

        /// 標準入力をこの種類のドキュメントとして扱う
        #[arg(long, value_enum, default_value = "md")]
        stdin_format: InputKind,

        /// パスの列挙の代わりにJSONレポート({"files": [...]})を出力
        #[arg(long)]
        json: bool,

        /// レンダリングに失敗したブロックがあればexit 1で失敗させる
        #[arg(long)]
        strict: bool,
    },

    /// mermaidの図をUnicode罫線のテキストとしてターミナルにプレビュー
    Ascii {
        /// 入力.mmdファイル。'-'または省略で標準入力
        input: Option<PathBuf>,
    },

    /// 利用可能なテーマを一覧表示
    Themes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum InputKind {
    Md,
    Html,
    Mmd,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Render {
            input,
            output,
            format,
            scale,
            background,
        } => cmd_render(&cli.theme, input, output, format, scale, background),
        Command::Html {
            input,
            output,
            in_place,
            keep_source,
            strict,
        } => cmd_html(&cli.theme, input, output, in_place, keep_source, strict),
        Command::Export {
            input,
            out_dir,
            format,
            stem,
            scale,
            background,
            stdin_format,
            json,
            strict,
        } => cmd_export(
            &cli.theme,
            input,
            out_dir,
            format,
            stem,
            scale,
            background,
            stdin_format,
            json,
            strict,
        ),
        Command::Ascii { input } => cmd_ascii(input),
        Command::Themes => cmd_themes(),
    }
}

fn read_input(path: Option<&Path>) -> Result<String> {
    match path {
        Some(p) if p.as_os_str() != "-" => std::fs::read_to_string(p)
            .with_context(|| format!("failed to read {}", p.display())),
        _ => {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .context("failed to read stdin")?;
            Ok(buf)
        }
    }
}

fn cmd_render(
    theme: &ThemeArgs,
    input: Option<PathBuf>,
    output: Option<PathBuf>,
    format: Option<Format>,
    scale: f32,
    background: Option<String>,
) -> Result<()> {
    let source = read_input(input.as_deref())?;
    let format = match (format, output.as_deref()) {
        (Some(f), _) => f,
        (None, None) => Format::Svg,
        (None, Some(path)) => Format::from_extension(path).with_context(|| {
            format!(
                "cannot infer output format from '{}': use a .svg/.png/.jpg/.pdf extension or pass --format",
                path.display()
            )
        })?,
    };

    let renderer = renderer::build_renderer(theme)?;
    let opts = RenderOptions { scale, background };
    let id = output
        .as_deref()
        .and_then(|p| p.file_stem())
        .and_then(|s| s.to_str())
        .unwrap_or("jmmview");

    let bytes = renderer::render(&renderer, &source, format, id, &opts)?
        .context("input is not recognized as a mermaid diagram")?;

    match output {
        Some(path) => {
            std::fs::write(&path, &bytes)
                .with_context(|| format!("failed to write {}", path.display()))?;
            eprintln!("wrote {} ({} bytes)", path.display(), bytes.len());
        }
        None => std::io::stdout()
            .write_all(&bytes)
            .context("failed to write to stdout")?,
    }
    Ok(())
}

fn cmd_html(
    theme: &ThemeArgs,
    input: PathBuf,
    output: Option<PathBuf>,
    in_place: bool,
    keep_source: bool,
    strict: bool,
) -> Result<()> {
    if in_place && input.as_os_str() == "-" {
        bail!("--in-place requires a file input, not stdin");
    }
    let html = read_input(Some(&input))?;
    let renderer = renderer::build_renderer(theme)?;

    let report = extract::rewrite_html(&html, keep_source, |index, source| {
        renderer::render_inline_svg(&renderer, source, &format!("jmm-{index}"))
    });

    for warning in &report.failed {
        eprintln!("warning: {warning}");
    }
    if strict && !report.failed.is_empty() {
        bail!("{} mermaid block(s) failed to render", report.failed.len());
    }

    let target = if in_place { Some(input) } else { output };
    match target {
        Some(path) => {
            std::fs::write(&path, report.html.as_bytes())
                .with_context(|| format!("failed to write {}", path.display()))?;
            eprintln!(
                "wrote {} ({} diagram(s) inlined, {} mermaid script tag(s) removed)",
                path.display(),
                report.rendered,
                report.scripts_removed
            );
        }
        None => {
            std::io::stdout()
                .write_all(report.html.as_bytes())
                .context("failed to write to stdout")?;
            eprintln!(
                "{} diagram(s) inlined, {} mermaid script tag(s) removed",
                report.rendered, report.scripts_removed
            );
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn cmd_export(
    theme: &ThemeArgs,
    input: PathBuf,
    out_dir: PathBuf,
    format: Format,
    stem: Option<String>,
    scale: f32,
    background: Option<String>,
    stdin_format: InputKind,
    json: bool,
    strict: bool,
) -> Result<()> {
    let text = read_input(Some(&input))?;
    let kind = if input.as_os_str() == "-" {
        stdin_format
    } else {
        match input
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("md")
            .to_ascii_lowercase()
            .as_str()
        {
            "html" | "htm" => InputKind::Html,
            "mmd" | "mermaid" => InputKind::Mmd,
            _ => InputKind::Md,
        }
    };

    let blocks: Vec<extract::Block> = match kind {
        InputKind::Html => extract::extract_from_html(&text),
        InputKind::Mmd => vec![extract::Block { source: text }],
        InputKind::Md => extract::extract_from_markdown(&text),
    };
    if blocks.is_empty() {
        let message = format!("no mermaid blocks found in {}", input.display());
        if json {
            let report = serde_json::json!({
                "files": [],
                "rendered": 0,
                "failed": [message],
            });
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        bail!("{message}");
    }

    std::fs::create_dir_all(&out_dir)
        .with_context(|| format!("failed to create {}", out_dir.display()))?;

    let stem = stem.unwrap_or_else(|| {
        input
            .file_stem()
            .and_then(|s| s.to_str())
            .filter(|s| *s != "-")
            .unwrap_or("diagram")
            .to_string()
    });

    let renderer = renderer::build_renderer(theme)?;
    let opts = RenderOptions { scale, background };
    let mut written = Vec::new();
    let mut failures = Vec::new();

    for (i, block) in blocks.iter().enumerate() {
        let name = if blocks.len() == 1 {
            format!("{stem}.{}", format.extension())
        } else {
            format!("{stem}-{}.{}", i + 1, format.extension())
        };
        let path = out_dir.join(&name);
        let id = format!("{stem}-{}", i + 1);
        match renderer::render(&renderer, &block.source, format, &id, &opts) {
            Ok(Some(bytes)) => {
                std::fs::write(&path, &bytes)
                    .with_context(|| format!("failed to write {}", path.display()))?;
                written.push(path);
            }
            Ok(None) => failures.push(format!("block {}: not recognized as mermaid", i + 1)),
            Err(err) => failures.push(format!("block {}: {err:#}", i + 1)),
        }
    }

    for failure in &failures {
        eprintln!("warning: {failure}");
    }

    if json {
        let report = serde_json::json!({
            "files": written.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
            "rendered": written.len(),
            "failed": failures,
        });
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        for path in &written {
            println!("{}", path.display());
        }
    }

    if written.is_empty() {
        bail!("all {} mermaid block(s) failed to render", blocks.len());
    }
    if strict && !failures.is_empty() {
        bail!("{} mermaid block(s) failed to render", failures.len());
    }
    Ok(())
}

fn cmd_ascii(input: Option<PathBuf>) -> Result<()> {
    use merman::ascii::{AsciiRenderOptions, HeadlessAsciiRenderer};
    let source = read_input(input.as_deref())?;
    let text = renderer::catch_panic(|| {
        HeadlessAsciiRenderer::new()
            .with_lenient_parsing()
            .with_ascii_options(AsciiRenderOptions::unicode())
            .render_ascii_sync(&source)
            .context("failed to render text preview")
    })?
    .context("input is not recognized as a mermaid diagram")?;
    println!("{text}");
    Ok(())
}

fn cmd_themes() -> Result<()> {
    println!("mermaid themes (--theme):");
    for name in merman::supported_themes() {
        println!("  {name}");
    }
    println!("\nhost themes (--host-theme):");
    for name in merman::supported_host_theme_presets() {
        println!("  {name}");
    }
    Ok(())
}
