//! テーマ処理を含む、mermanのHeadlessRendererの薄いラッパー。

use anyhow::{Context, Result, bail};
use merman::render::raster::RasterOptions;
use merman::render::{HeadlessRenderer, HostThemePreset, HostThemeProfile};

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Format {
    Svg,
    Png,
    Jpeg,
    Pdf,
}

impl Format {
    pub fn from_extension(path: &std::path::Path) -> Option<Self> {
        match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
            "svg" => Some(Self::Svg),
            "png" => Some(Self::Png),
            "jpg" | "jpeg" => Some(Self::Jpeg),
            "pdf" => Some(Self::Pdf),
            _ => None,
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Svg => "svg",
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Pdf => "pdf",
        }
    }
}

#[derive(Debug, Clone, Default, clap::Args)]
pub struct ThemeArgs {
    /// Mermaidテーマ: default, base, dark, forest, neutral, neo, neo-dark,
    /// redux, redux-dark, redux-color, redux-dark-color
    #[arg(long, global = true)]
    pub theme: Option<String>,

    /// エディタ風ホストテーマ: editor-light, editor-dark, one-dark,
    /// gruvbox-light, gruvbox-dark, ayu-light, ayu-dark
    #[arg(long, global = true, conflicts_with = "theme")]
    pub host_theme: Option<String>,
}

pub fn build_renderer(theme: &ThemeArgs) -> Result<HeadlessRenderer> {
    let mut renderer = HeadlessRenderer::new()
        .with_lenient_parsing()
        .with_vendored_text_measurer();

    if let Some(name) = &theme.host_theme {
        let preset = HostThemePreset::ALL
            .into_iter()
            .find(|p| p.as_str() == name)
            .with_context(|| {
                format!(
                    "unknown host theme '{name}' (available: {})",
                    merman::render::supported_host_theme_presets().join(", ")
                )
            })?;
        renderer = renderer.with_host_theme(&HostThemeProfile::from_preset(preset));
    } else if let Some(name) = &theme.theme {
        if !merman::supported_themes().contains(&name.as_str()) {
            bail!(
                "unknown theme '{name}' (available: {})",
                merman::supported_themes().join(", ")
            );
        }
        renderer = renderer.with_site_config(merman::MermaidConfig::from_value(
            serde_json::json!({ "theme": name }),
        ));
    }

    Ok(renderer)
}

pub struct RenderOptions {
    pub scale: f32,
    pub background: Option<String>,
}

/// merman 0.7は一部の入力(マルチバイトのxychart軸ラベルなど)でパニックする
/// ため、レンダリング呼び出しを隔離し、1つの不正なブロックがhtml/export全体を
/// 中断させないようにする。
pub fn catch_panic<T>(f: impl FnOnce() -> Result<T>) -> Result<T> {
    // 標準の"thread panicked"というstderr出力を抑止し、パニック内容は
    // 戻り値のエラーとして報告する。シングルスレッドのCLIなので、呼び出しの
    // 前後でグローバルフックを差し替えても安全。
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    std::panic::set_hook(hook);
    match result {
        Ok(result) => result,
        Err(payload) => {
            let msg = payload
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "unknown panic".to_string());
            bail!("renderer crashed (merman bug?): {msg}")
        }
    }
}

/// mermanのラスタ用色パーサが解釈できる色だけを受け付ける。
fn validate_background(bg: &str) -> Result<()> {
    let s = bg.trim().to_ascii_lowercase();
    if matches!(s.as_str(), "transparent" | "white" | "black") {
        return Ok(());
    }
    if let Some(hex) = s.strip_prefix('#')
        && matches!(hex.len(), 3 | 4 | 6 | 8)
        && hex.chars().all(|c| c.is_ascii_hexdigit())
    {
        return Ok(());
    }
    bail!(
        "unsupported background color '{bg}' (accepted: transparent, white, black, #RGB, #RGBA, #RRGGBB, #RRGGBBAA)"
    )
}

/// 1つのmermaidソースを指定形式にレンダリングする。ソースがmermaidの図として
/// 認識できない場合はNoneを返す。
pub fn render(
    renderer: &HeadlessRenderer,
    source: &str,
    format: Format,
    diagram_id: &str,
    opts: &RenderOptions,
) -> Result<Option<Vec<u8>>> {
    let renderer = renderer.clone().with_diagram_id(diagram_id);
    catch_panic(|| match format {
        Format::Svg => {
            let svg = renderer
                .render_svg_sync(source)
                .context("failed to render SVG")?;
            Ok(svg.map(String::into_bytes))
        }
        Format::Png | Format::Jpeg => {
            let mut raster = RasterOptions::default().with_scale(opts.scale);
            if let Some(bg) = &opts.background {
                // mermanはPNG経路では解釈できない色を黙って無視するため、
                // 透明な画像を書き出してしまう前にここで検証する。
                validate_background(bg)?;
                raster = raster.with_background(bg.clone());
            } else if format == Format::Jpeg {
                // JPEGにはアルファチャンネルがないため、mermanは不透明な背景色を要求する。
                raster = raster.with_background("#ffffff");
            }
            let bytes = if format == Format::Png {
                renderer.render_png_sync(source, &raster)
            } else {
                renderer.render_jpeg_sync(source, &raster)
            }
            .context("failed to rasterize diagram")?;
            Ok(bytes)
        }
        Format::Pdf => renderer
            .render_pdf_sync(source)
            .context("failed to render PDF"),
    })
}

/// HTMLへのインライン埋め込み用に、mermaidソースをSVG文字列へレンダリングする。
pub fn render_inline_svg(
    renderer: &HeadlessRenderer,
    source: &str,
    diagram_id: &str,
) -> Result<Option<String>> {
    let renderer = renderer.clone().with_diagram_id(diagram_id);
    catch_panic(|| {
        renderer
            .render_svg_sync(source)
            .context("failed to render SVG")
    })
}
