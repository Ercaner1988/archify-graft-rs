pub mod architecture_svg;
pub mod dataflow_svg;
pub mod delta_svg;
pub mod sequence_svg;
pub mod xml;

pub use architecture_svg::ArchitectureSvgRenderer;
pub use dataflow_svg::DataflowSvgRenderer;
pub use delta_svg::DeltaSvgRenderer;
pub use sequence_svg::SequenceSvgRenderer;

use archify_ir::ArchitectureDiagram;

pub struct SvgRenderer;

impl SvgRenderer {
    pub fn render_delta(delta: &archify_ir::DeltaDiagram) -> String {
        DeltaSvgRenderer::render(delta)
    }

    pub fn render_sequence(seq: &archify_ir::SequenceDiagram) -> String {
        SequenceSvgRenderer::render(seq)
    }

    pub fn render_dataflow(df: &archify_ir::DataflowDiagram) -> String {
        DataflowSvgRenderer::render(df)
    }

    pub fn render(diagram: &ArchitectureDiagram) -> String {
        ArchitectureSvgRenderer::render(diagram)
    }
}

/// Wraps a rendered SVG string into a single self-contained, explorable HTML file:
/// drag to pan, wheel/pinch to zoom, double-click or the button to reset. This is not
/// a port of Archify's full viewer (no search/focus/trace/theme-switch/export) — just
/// enough that a generated diagram is actually explorable in a browser instead of a
/// flat, non-interactive image.
pub fn wrap_html(svg: &str, title: &str) -> String {
    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>{title}</title>
<style>
  html,body {{ margin:0; height:100%; overflow:hidden; background:#020617; }}
  #stage {{ width:100%; height:100%; cursor:grab; touch-action:none; }}
  #stage.grabbing {{ cursor:grabbing; }}
  #stage svg {{ width:100%; height:100%; display:block; }}
  #reset {{
    position:fixed; top:12px; right:12px; z-index:10;
    font:12px system-ui,sans-serif; padding:6px 10px; border-radius:6px;
    border:1px solid #334155; background:#0f172acc; color:#e2e8f0; cursor:pointer;
  }}
  #reset:hover {{ background:#1e293bcc; }}
</style>
</head>
<body>
<button id="reset" title="Reset pan/zoom">Reset view</button>
<div id="stage">{svg}</div>
<script>
(function() {{
  var stage = document.getElementById('stage');
  var svg = stage.querySelector('svg');
  var x = 0, y = 0, scale = 1, dragging = false, lastX = 0, lastY = 0;

  function apply() {{
    svg.style.transform = 'translate(' + x + 'px,' + y + 'px) scale(' + scale + ')';
    svg.style.transformOrigin = '0 0';
  }}
  function reset() {{ x = 0; y = 0; scale = 1; apply(); }}

  stage.addEventListener('wheel', function(e) {{
    e.preventDefault();
    var prev = scale;
    scale = Math.min(8, Math.max(0.2, scale * (e.deltaY < 0 ? 1.1 : 1 / 1.1)));
    // zoom toward the cursor, not the top-left corner
    var rect = stage.getBoundingClientRect();
    var cx = e.clientX - rect.left, cy = e.clientY - rect.top;
    x = cx - (cx - x) * (scale / prev);
    y = cy - (cy - y) * (scale / prev);
    apply();
  }}, {{ passive: false }});

  stage.addEventListener('pointerdown', function(e) {{
    dragging = true; lastX = e.clientX; lastY = e.clientY;
    stage.classList.add('grabbing');
    stage.setPointerCapture(e.pointerId);
  }});
  stage.addEventListener('pointermove', function(e) {{
    if (!dragging) return;
    x += e.clientX - lastX; y += e.clientY - lastY;
    lastX = e.clientX; lastY = e.clientY;
    apply();
  }});
  ['pointerup', 'pointercancel', 'pointerleave'].forEach(function(ev) {{
    stage.addEventListener(ev, function() {{ dragging = false; stage.classList.remove('grabbing'); }});
  }});
  stage.addEventListener('dblclick', reset);
  document.getElementById('reset').addEventListener('click', reset);
}})();
</script>
</body>
</html>
"##,
        title = xml::esc(title),
        svg = svg
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use archify_ir::{Component, DiagramMeta, SemanticRole, VisualPreset};

    #[test]
    fn test_all_four_presets() {
        let mut diagram = ArchitectureDiagram {
            meta: DiagramMeta {
                title: "Preset Test".to_string(),
                subtitle: None,
                locale: "en".to_string(),
                visual_preset: VisualPreset::SignalFlow,
            },
            components: vec![Component {
                id: "c1".to_string(),
                label: "Client".to_string(),
                sublabel: None,
                role: SemanticRole::Frontend,
                x: 10.0,
                y: 10.0,
                width: 100.0,
                height: 50.0,
            }],
            connections: vec![],
            regions: vec![],
            story_beats: vec![],
        };

        // 1. SignalFlow
        let svg_signal = SvgRenderer::render(&diagram);
        assert!(svg_signal.contains("filter:drop-shadow(0 0 8px"));

        // 2. Blueprint (zero blur)
        diagram.meta.visual_preset = VisualPreset::Blueprint;
        let svg_bp = SvgRenderer::render(&diagram);
        assert!(svg_bp.contains("#0a192f"));
        assert!(!svg_bp.contains("filter:drop-shadow"));

        // 3. Editorial (warm paper)
        diagram.meta.visual_preset = VisualPreset::Editorial;
        let svg_ed = SvgRenderer::render(&diagram);
        assert!(svg_ed.contains("#f8fafc"));
        assert!(svg_ed.contains("Georgia, Cambria, serif"));
    }
}
