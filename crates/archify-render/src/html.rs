//! Single-file HTML page around a rendered SVG: pure HTML and CSS, no script.

use crate::xml::esc;

/// Wraps a rendered SVG into a self-contained page. The SVG carries its own theme (dark,
/// light via `prefers-color-scheme`), hover dimming and intro motion in CSS; the page only
/// sizes it. Zoom with the browser (Ctrl +/-, pinch); the page scrolls when the picture is wider
/// than the window.
pub fn wrap_html(svg: &str, title: &str) -> String {
    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<style>
  :root {{ color-scheme: dark light; }}
  html, body {{ margin: 0; min-height: 100%; background: #020617; }}
  @media (prefers-color-scheme: light) {{ html, body {{ background: #f8fafc; }} }}
  main {{ overflow: auto; }}
  main > svg {{ display: block; width: 100%; height: auto; min-width: 900px; min-height: 100vh; }}
</style>
</head>
<body>
<main>
{svg}
</main>
</body>
</html>
"##,
        title = esc(title),
        svg = svg
    )
}
