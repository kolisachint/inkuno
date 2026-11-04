//! Renderer traits: DiagramRenderer, MathRenderer, Highlighter (§6.7).
//! Request contract for rich block rendering (§6.2).

/// Which kind of rich block is being rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderKind {
    Mermaid,
    Math,
}

/// The active colour theme. Each theme has its own cache entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Theme {
    Light,
    Dark,
}

/// One render request: `(kind, source, theme)` (§6.2).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenderRequest {
    pub kind: RenderKind,
    pub source: String,
    pub theme: Theme,
}

/// Why a render failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderError {
    /// The source text is not valid for the renderer.
    Invalid(String),
    /// The render host failed (crashed, timed out, unavailable).
    Host(String),
}

impl RenderError {
    /// The human-readable message carried by the error.
    pub fn message(&self) -> &str {
        match self {
            RenderError::Invalid(m) | RenderError::Host(m) => m,
        }
    }
}

/// Successful output is an SVG document.
pub type RenderResult = Result<String, RenderError>;

/// Synchronous backend called from a worker thread.
pub trait RenderBackend: Send + Sync + 'static {
    fn render(&self, req: &RenderRequest) -> RenderResult;
}

/// Renders diagram source (Mermaid in v1) to SVG.
pub trait DiagramRenderer: Send + Sync {
    fn render_diagram(&self, source: &str, theme: Theme) -> RenderResult;
}

/// Renders LaTeX (MathJax in v1) to SVG.
pub trait MathRenderer: Send + Sync {
    fn render_math(&self, tex: &str, display: bool, theme: Theme) -> RenderResult;
}
