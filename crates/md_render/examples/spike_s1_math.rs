//! S1 spike (DESIGN §6.1, §6.4, §6.5, §6.8): run MathJax v3 in embedded QuickJS to
//! turn TeX into SVG, then rasterize the result with resvg.
//!
//! Throwaway evidence for `docs/spikes/S1-render-host.md`. It is not used by the
//! render queue. The MathJax bundle is built outside the repository (see the note)
//! and passed in by path, so no bundle is committed.
//!
//! ```text
//! MathJax bundle: <mathjax-bundle.js>   (IIFE that defines globalThis.texToSvg)
//! Output dir:     <out-dir>
//! Optional:       [mermaid.svg]         (rasterized as a resvg check)
//! cargo run -p md_render --example spike_s1_math \
//!     --features "spike-quickjs spike-resvg" --release -- <bundle> <out-dir> [mermaid.svg]
//! ```

/// Formulas to convert: (name, TeX, display mode).
#[cfg(feature = "spike-quickjs")]
const FORMULAS: &[(&str, &str, bool)] = &[
    ("frac", r"\frac{a}{b}", true),
    ("sum", r"\sum_{i=1}^{n} i^2 = \frac{n(n+1)(2n+1)}{6}", true),
    ("inline", r"x^2 + y^2 = r^2", false),
    ("error", r"\frac{a}{", true),
];

/// Repeat count for the warm-latency measurement.
#[cfg(feature = "spike-quickjs")]
const WARM_RUNS: usize = 200;

#[cfg(feature = "spike-quickjs")]
struct Rendered {
    name: &'static str,
    markup: String,
    micros: u128,
}

/// Loads the bundle into a fresh QuickJS context and converts every formula.
#[cfg(feature = "spike-quickjs")]
fn render_all(bundle: &str) -> Result<(f64, Vec<Rendered>, f64), Box<dyn std::error::Error>> {
    use rquickjs::{Context, Function, Runtime};
    use std::time::Instant;

    let t0 = Instant::now();
    let rt = Runtime::new()?;
    let ctx = Context::full(&rt)?;
    let result = ctx.with(|ctx| -> rquickjs::Result<(f64, Vec<Rendered>, f64)> {
        ctx.eval::<(), _>(bundle)?;
        let init_ms = t0.elapsed().as_secs_f64() * 1e3;
        let tex_to_svg: Function = ctx.globals().get("texToSvg")?;

        let mut rendered = Vec::new();
        for &(name, tex, display) in FORMULAS {
            let t = Instant::now();
            let markup: String = tex_to_svg.call((tex, display))?;
            rendered.push(Rendered {
                name,
                markup,
                micros: t.elapsed().as_micros(),
            });
        }

        let t = Instant::now();
        for _ in 0..WARM_RUNS {
            let _: String = tex_to_svg.call((FORMULAS[1].1, true))?;
        }
        let warm_mean_us = t.elapsed().as_secs_f64() * 1e6 / WARM_RUNS as f64;
        Ok((init_ms, rendered, warm_mean_us))
    })?;
    Ok(result)
}

/// The `<svg>...</svg>` element inside the `<mjx-container>` wrapper.
#[cfg(feature = "spike-quickjs")]
fn extract_svg(markup: &str) -> Option<&str> {
    let start = markup.find("<svg")?;
    let end = markup.rfind("</svg>")? + "</svg>".len();
    Some(&markup[start..end])
}

/// Rasterizes SVG at `scale` with system fonts loaded. Returns a one-line summary.
#[cfg(feature = "spike-resvg")]
fn rasterize(svg: &str, scale: f32, png: Option<&std::path::Path>) -> String {
    use resvg::{tiny_skia, usvg};

    let mut opt = usvg::Options::default();
    opt.fontdb_mut().load_system_fonts();
    let tree = match usvg::Tree::from_str(svg, &opt) {
        Ok(tree) => tree,
        Err(e) => return format!("usvg parse error: {e}"),
    };
    let size = tree.size();
    let w = (size.width() * scale).ceil() as u32;
    let h = (size.height() * scale).ceil() as u32;
    let Some(mut pixmap) = tiny_skia::Pixmap::new(w, h) else {
        return "zero-size pixmap".to_string();
    };
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    let painted = pixmap.pixels().iter().filter(|p| p.alpha() > 0).count();
    if let Some(path) = png {
        if let Err(e) = pixmap.save_png(path) {
            return format!("png write error: {e}");
        }
    }
    format!("resvg {w}x{h}px painted_px={painted}")
}

#[cfg(feature = "spike-resvg")]
fn raster_summary(svg: &str, png: &std::path::Path) -> String {
    rasterize(svg, 2.0, Some(png))
}

#[cfg(all(feature = "spike-quickjs", not(feature = "spike-resvg")))]
fn raster_summary(_svg: &str, _png: &std::path::Path) -> String {
    "resvg not built (feature spike-resvg off)".to_string()
}

#[cfg(feature = "spike-quickjs")]
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        return Err("usage: spike_s1_math <mathjax-bundle.js> <out-dir> [mermaid.svg]".into());
    }
    let bundle = std::fs::read_to_string(&args[1])?;
    let out = std::path::PathBuf::from(&args[2]);
    std::fs::create_dir_all(&out)?;

    let (init_ms, rendered, warm_mean_us) = render_all(&bundle)?;
    println!("quickjs init (runtime + bundle eval): {init_ms:.1} ms");
    println!("warm mean per convert ({WARM_RUNS} runs): {warm_mean_us:.0} us");

    // Theme colours for the fill (DESIGN §6.4): MathJax emits currentColor.
    const LIGHT: &str = "#1d1d1f";
    const DARK: &str = "#f5f5f7";

    for r in &rendered {
        let svg = extract_svg(&r.markup).ok_or("no <svg> in MathJax output")?;
        println!(
            "[{}] convert={} us svg_bytes={} foreignObject={} currentColor={} data-mjx-error={}",
            r.name,
            r.micros,
            svg.len(),
            svg.matches("<foreignObject").count(),
            svg.matches("currentColor").count(),
            svg.contains("data-mjx-error"),
        );
        for (theme, colour) in [("light", LIGHT), ("dark", DARK)] {
            let themed = svg.replace("currentColor", colour);
            let path = out.join(format!("math-{}-{theme}.svg", r.name));
            std::fs::write(&path, &themed)?;
            if theme == "light" {
                let png = out.join(format!("math-{}-{theme}.png", r.name));
                println!("    resvg: {}", raster_summary(&themed, &png));
            }
        }
    }

    if let Some(mermaid) = args.get(3) {
        let svg = std::fs::read_to_string(mermaid)?;
        let png = out.join("mermaid-check.png");
        println!("[mermaid] {} bytes from {mermaid}", svg.len());
        println!("    resvg: {}", raster_summary(&svg, &png));
    }
    Ok(())
}

#[cfg(not(feature = "spike-quickjs"))]
fn run() -> Result<(), Box<dyn std::error::Error>> {
    Err("build with --features spike-quickjs (and spike-resvg for rasterization)".into())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
