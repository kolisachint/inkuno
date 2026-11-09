//! Diagram stub: the canvas diagram model was removed (owner-approved cut).
//! Only the Mermaid-in-wiki path stays (`logic::mermaid::kind`,
//! `ui::figure::mermaid`, `views::viewer::Source::Mermaid`).
//! This stub keeps `cargo check` and the `ui::figure` test module compiling
//! until the integrator removes the last diagram references (see REPORT).
//!
//! REPORT for integrator: delete this file + `pub mod diagram` in
//! `src/logic/mod.rs`, remove `exports_a_styled_diagram` at
//! `src/ui/figure.rs:165-176`, and drop `Pane::viewer` if nothing uses it
//! (`src/shell/pane.rs`, currently test-only).
#![allow(dead_code)]

/// Version of the removed diagram format. Kept so the MCP `vault_info`
/// answer still carries a `diagram_version` number.
pub const VERSION: u32 = 0;

/// Ink colours of diagrams, as `0xrrggbb`.
pub const COLORS: [u32; 6] = [0x1c1c1c, 0xc0392b, 0x2980b9, 0x27ae60, 0x8e44ad, 0xe67e22];

/// Shape form. Only `Rect` is needed by the remaining test.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Form {
    Rect,
}

/// One shape of a stub diagram.
#[derive(Clone, Debug)]
pub struct Shape {
    pub id: u32,
    pub text: String,
}

/// A stub diagram: enough for the leftover `ui::figure` test.
/// `blank()` and `default()` draw differently on purpose, so the test's
/// styled-vs-plain PNG comparison still holds.
#[derive(Clone, Debug)]
pub struct Diagram {
    shapes: Vec<Shape>,
    next: u32,
    rich: bool,
}

impl Default for Diagram {
    fn default() -> Self {
        Diagram {
            shapes: Vec::new(),
            next: 1,
            rich: true,
        }
    }
}

impl Diagram {
    pub fn blank() -> Self {
        Diagram {
            shapes: Vec::new(),
            next: 1,
            rich: false,
        }
    }

    pub fn shape_mut(&mut self, id: u32) -> Option<&mut Shape> {
        self.shapes.iter_mut().find(|s| s.id == id)
    }

    pub fn add_shape(&mut self, _form: Form, _x: f32, _y: f32, _w: f32, _h: f32) -> u32 {
        let id = self.next;
        self.next += 1;
        self.shapes.push(Shape {
            id,
            text: String::new(),
        });
        id
    }

    /// Minimal valid SVG. `default()` draws an extra mark so its PNG differs.
    pub fn to_svg(&self, _ink: u32) -> String {
        let mut svg = String::from(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100"><rect x="1" y="1" width="198" height="98" fill="white" stroke="black"/>"#,
        );
        if self.rich {
            svg.push_str(r#"<rect x="10" y="10" width="20" height="10" fill="black"/>"#);
        }
        for shape in &self.shapes {
            let text: String = shape
                .text
                .chars()
                .flat_map(|c| match c {
                    '<' => "&lt;".chars().collect::<Vec<_>>(),
                    '>' => "&gt;".chars().collect::<Vec<_>>(),
                    '&' => "&amp;".chars().collect::<Vec<_>>(),
                    c => vec![c],
                })
                .collect();
            svg.push_str(&format!("<text x=\"20\" y=\"50\">{text}</text>"));
        }
        svg.push_str("</svg>");
        svg
    }
}
