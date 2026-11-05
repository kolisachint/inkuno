//! SVG to pixels with resvg/usvg (DESIGN §6.5, task 6.4).
//!
//! The output is a plain straight-alpha RGBA buffer. It holds no GPUI types,
//! so the caller converts it into a GPUI image at its own boundary.

use std::fmt;
use std::sync::Arc;

use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{self, fontdb};

/// Largest accepted width or height in pixels, to bound allocations.
pub const MAX_SIDE_PX: u32 = 8192;

/// A rasterized SVG.
///
/// `rgba` is row-major, 4 bytes per pixel, straight (non-premultiplied) alpha,
/// and has length `width * height * 4`. The background is transparent unless
/// the SVG paints one.
#[derive(Debug, Clone, PartialEq)]
pub struct RasterImage {
    pub width: u32,
    pub height: u32,
    /// The scale factor the image was rasterized at (1.0, 2.0 on Retina).
    pub scale: f32,
    pub rgba: Vec<u8>,
}

/// Why rasterization failed.
#[derive(Debug, Clone, PartialEq)]
pub enum RasterError {
    /// The scale factor is not a finite number greater than zero.
    InvalidScale(f32),
    /// The SVG could not be parsed.
    Parse(String),
    /// The SVG has no drawable area, or the scaled size is unusable.
    EmptySize,
    /// The scaled size exceeds [`MAX_SIDE_PX`] on one side.
    TooLarge { width: u32, height: u32 },
}

impl fmt::Display for RasterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RasterError::InvalidScale(s) => write!(f, "invalid scale factor {s}"),
            RasterError::Parse(m) => write!(f, "invalid SVG: {m}"),
            RasterError::EmptySize => write!(f, "SVG has an empty size"),
            RasterError::TooLarge { width, height } => {
                write!(
                    f,
                    "rasterized size {width}x{height} exceeds {MAX_SIDE_PX}px"
                )
            }
        }
    }
}

impl std::error::Error for RasterError {}

/// Loads the fonts installed on this machine. Text in an SVG is drawn only
/// when a matching face is in the database.
pub fn system_fonts() -> Arc<fontdb::Database> {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();
    Arc::new(db)
}

/// Rasterizes `svg` at `scale` times its intrinsic size.
///
/// The pixel size is `ceil(width * scale) x ceil(height * scale)`. `fonts`
/// supplies the faces for `<text>` elements.
pub fn rasterize_svg(
    svg: &str,
    scale: f32,
    fonts: Arc<fontdb::Database>,
) -> Result<RasterImage, RasterError> {
    if !(scale.is_finite() && scale > 0.0) {
        return Err(RasterError::InvalidScale(scale));
    }

    let options = usvg::Options {
        fontdb: fonts,
        ..usvg::Options::default()
    };
    let tree =
        usvg::Tree::from_str(svg, &options).map_err(|e| RasterError::Parse(e.to_string()))?;

    let size = tree.size();
    let width = (size.width() * scale).ceil();
    let height = (size.height() * scale).ceil();
    if !(width >= 1.0 && height >= 1.0) {
        return Err(RasterError::EmptySize);
    }
    if width > MAX_SIDE_PX as f32 || height > MAX_SIDE_PX as f32 {
        return Err(RasterError::TooLarge {
            width: width as u32,
            height: height as u32,
        });
    }
    let (width, height) = (width as u32, height as u32);

    let mut pixmap = Pixmap::new(width, height).ok_or(RasterError::EmptySize)?;
    resvg::render(
        &tree,
        Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );

    // tiny-skia stores premultiplied alpha; the output is straight alpha.
    let mut rgba = Vec::with_capacity(pixmap.pixels().len() * 4);
    for px in pixmap.pixels() {
        let c = px.demultiply();
        rgba.extend_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
    }

    Ok(RasterImage {
        width,
        height,
        scale,
        rgba,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Stands in for a MathJax `tex2svgPromise` output: a 45x13 pt box
    /// holding two bars.
    const FORMULA: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="45" height="13" viewBox="0 0 45 13">
  <rect x="5" y="3" width="30" height="2" fill="#000"/>
  <rect x="10" y="7" width="20" height="2" fill="#000"/>
</svg>"##;

    fn empty_fonts() -> Arc<fontdb::Database> {
        Arc::new(fontdb::Database::new())
    }

    fn pixel(img: &RasterImage, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * img.width + x) * 4) as usize;
        [
            img.rgba[i],
            img.rgba[i + 1],
            img.rgba[i + 2],
            img.rgba[i + 3],
        ]
    }

    #[test]
    fn scale_one_gives_intrinsic_dimensions() {
        let img = rasterize_svg(FORMULA, 1.0, empty_fonts()).unwrap();
        assert_eq!((img.width, img.height), (45, 13));
        assert_eq!(img.scale, 1.0);
        assert_eq!(img.rgba.len(), 45 * 13 * 4);
    }

    #[test]
    fn scale_two_doubles_dimensions() {
        let img = rasterize_svg(FORMULA, 2.0, empty_fonts()).unwrap();
        assert_eq!((img.width, img.height), (90, 26));
        assert_eq!(img.scale, 2.0);
        assert_eq!(img.rgba.len(), 90 * 26 * 4);
    }

    #[test]
    fn fractional_scale_rounds_up() {
        // 45 * 1.5 = 67.5 -> 68, 13 * 1.5 = 19.5 -> 20
        let img = rasterize_svg(FORMULA, 1.5, empty_fonts()).unwrap();
        assert_eq!((img.width, img.height), (68, 20));
    }

    #[test]
    fn scaled_output_still_draws_the_formula() {
        let img = rasterize_svg(FORMULA, 2.0, empty_fonts()).unwrap();
        // The top bar spans x 5..35, y 3..5 in user units, so x 10..70, y 6..10 at 2x.
        assert_eq!(pixel(&img, 20, 8), [0, 0, 0, 255]);
    }

    #[test]
    fn background_is_transparent() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20">
  <rect x="5" y="5" width="10" height="10" fill="#ff0000"/>
</svg>"##;
        let img = rasterize_svg(svg, 1.0, empty_fonts()).unwrap();
        assert_eq!(pixel(&img, 0, 0), [0, 0, 0, 0]);
        assert_eq!(pixel(&img, 19, 19), [0, 0, 0, 0]);
        assert_eq!(pixel(&img, 10, 10), [255, 0, 0, 255]);
    }

    #[test]
    fn invalid_svg_is_an_error() {
        let err = rasterize_svg("this is not svg", 1.0, empty_fonts()).unwrap_err();
        assert!(matches!(err, RasterError::Parse(_)), "got {err:?}");

        let unclosed = r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10">"#;
        assert!(matches!(
            rasterize_svg(unclosed, 1.0, empty_fonts()),
            Err(RasterError::Parse(_))
        ));
    }

    #[test]
    fn zero_size_svg_is_an_error() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="0" height="0"/>"#;
        assert!(rasterize_svg(svg, 1.0, empty_fonts()).is_err());
    }

    #[test]
    fn invalid_scale_is_an_error() {
        for s in [0.0_f32, -1.0, f32::NAN, f32::INFINITY] {
            assert!(
                matches!(
                    rasterize_svg(FORMULA, s, empty_fonts()),
                    Err(RasterError::InvalidScale(_))
                ),
                "scale {s}"
            );
        }
    }

    #[test]
    fn oversized_output_is_rejected() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="5000" height="10"/>"#;
        let err = rasterize_svg(svg, 2.0, empty_fonts()).unwrap_err();
        assert_eq!(
            err,
            RasterError::TooLarge {
                width: 10000,
                height: 20
            }
        );
    }

    #[test]
    fn text_is_drawn_with_system_fonts() {
        let fonts = system_fonts();
        let Some(face) = fonts.faces().next() else {
            eprintln!("note: no system fonts installed; skipping text rendering test");
            return;
        };
        let family = face.families[0].0.clone();
        let svg = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="120" height="40">
  <text x="4" y="30" font-family="{family}" font-size="28" fill="#000">Ax</text>
</svg>"##
        );
        let img = rasterize_svg(&svg, 1.0, fonts).unwrap();
        let painted = img
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[3] > 0)
            .count();
        assert!(painted > 0, "text produced no pixels");
    }
}
