//! 16x16 role sigils, drawn with `currentColor` strokes (colour comes from `.i-<role>`).

use archify_ir::SemanticRole;

/// CSS suffix of a role (`c-backend`, `i-backend`, `--backend-stroke`).
pub fn role_css(role: SemanticRole) -> &'static str {
    match role {
        SemanticRole::Frontend => "frontend",
        SemanticRole::Backend => "backend",
        SemanticRole::Database => "database",
        SemanticRole::Cloud => "cloud",
        SemanticRole::Security => "security",
        SemanticRole::Messagebus => "messagebus",
        SemanticRole::External => "external",
    }
}

/// Inner SVG of the sigil.
pub fn sigil(role: SemanticRole) -> &'static str {
    match role {
        SemanticRole::Frontend => {
            "<rect x=\"1.5\" y=\"2.5\" width=\"13\" height=\"11\" rx=\"1.5\"/><path d=\"M1.5 6h13\"/>"
        }
        SemanticRole::Backend => "<path d=\"M5.5 4.5L2 8l3.5 3.5M10.5 4.5L14 8l-3.5 3.5\"/>",
        SemanticRole::Database => {
            "<ellipse cx=\"8\" cy=\"4\" rx=\"5\" ry=\"2\"/><path d=\"M3 4v8c0 1.1 2.2 2 5 2s5-.9 5-2V4M3 8c0 1.1 2.2 2 5 2s5-.9 5-2\"/>"
        }
        SemanticRole::Cloud => {
            "<path d=\"M4.5 12.5a3 3 0 0 1-.4-5.97 4 4 0 0 1 7.6-.5A3.2 3.2 0 0 1 11.5 12.5z\"/>"
        }
        SemanticRole::Security => {
            "<path d=\"M8 1.5l5 2v4c0 3.2-2.1 5.6-5 7-2.9-1.4-5-3.8-5-7v-4z\"/>"
        }
        SemanticRole::Messagebus => {
            "<path d=\"M2 4h9M2 8h12M2 12h9\"/><circle cx=\"13\" cy=\"4\" r=\"1\"/>"
        }
        SemanticRole::External => {
            "<path d=\"M9 2h5v5M14 2L7.5 8.5M12 10v3.5H2.5V4H6\"/>"
        }
    }
}
