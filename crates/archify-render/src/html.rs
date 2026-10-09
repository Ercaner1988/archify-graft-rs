//! Single-file HTML page around a rendered SVG:
//! - `wrap_html`: Pure HTML and CSS, no script (passes strict script-free contract).
//! - `wrap_interactive_html`: Full standalone interactive viewer with pan, zoom, search
//!   and interactive legend filtering (for rich HTML export).

use crate::xml::esc;

/// Wraps a rendered SVG into a self-contained page. The SVG carries its own theme (dark,
/// light via `prefers-color-scheme`), hover dimming and intro motion in CSS; the page only
/// sizes it. Zoom with the browser (Ctrl +/-, pinch); the page scrolls when the picture is wider
/// than the window. Pure HTML/CSS, zero script tags.
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

/// Rich standalone interactive viewer: pan, zoom, search filter, and clickable legend.
pub fn wrap_interactive_html(svg: &str, title: &str) -> String {
    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<style>
  :root {{
    color-scheme: dark light;
    --bg-page: #0d1117;
    --text-page: #e6edf3;
    --panel-bg: rgba(22, 27, 34, 0.85);
    --border-color: #30363d;
    --accent: #22d3ee;
  }}
  @media (prefers-color-scheme: light) {{
    :root {{
      --bg-page: #f6f8fa;
      --text-page: #1f2328;
      --panel-bg: rgba(255, 255, 255, 0.85);
      --border-color: #d0d7de;
      --accent: #0284c7;
    }}
  }}
  * {{ box-sizing: border-box; }}
  html, body {{
    margin: 0;
    padding: 0;
    width: 100vw;
    height: 100vh;
    overflow: hidden;
    background: var(--bg-page);
    color: var(--text-page);
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
  }}
  #viewer-container {{
    width: 100%;
    height: 100%;
    position: relative;
    cursor: grab;
    overflow: hidden;
  }}
  #viewer-container:active {{ cursor: grabbing; }}
  #viewport {{
    transform-origin: 0 0;
    position: absolute;
    top: 0;
    left: 0;
    will-change: transform;
  }}
  #viewport > svg {{
    display: block;
    user-select: none;
  }}
  .hud-panel {{
    position: absolute;
    top: 16px;
    left: 16px;
    z-index: 100;
    display: flex;
    gap: 8px;
    align-items: center;
    background: var(--panel-bg);
    backdrop-filter: blur(8px);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 6px 12px;
    box-shadow: 0 4px 12px rgba(0,0,0,0.25);
  }}
  .hud-btn {{
    background: transparent;
    border: 1px solid var(--border-color);
    color: var(--text-page);
    border-radius: 6px;
    padding: 6px 10px;
    font-size: 13px;
    cursor: pointer;
    transition: all 0.15s ease;
  }}
  .hud-btn:hover {{
    background: rgba(255, 255, 255, 0.1);
    border-color: var(--accent);
  }}
  .search-box {{
    background: rgba(0, 0, 0, 0.2);
    border: 1px solid var(--border-color);
    color: var(--text-page);
    border-radius: 6px;
    padding: 6px 10px;
    font-size: 13px;
    outline: none;
    width: 180px;
  }}
  .search-box:focus {{
    border-color: var(--accent);
  }}
  .dimmed {{
    opacity: 0.15 !important;
    filter: grayscale(0.8);
    transition: opacity 0.2s ease, filter 0.2s ease;
  }}
  .highlighted {{
    filter: drop-shadow(0 0 10px var(--accent)) !important;
    opacity: 1 !important;
  }}
</style>
</head>
<body>
<div id="viewer-container">
  <div class="hud-panel">
    <input type="text" id="search-input" class="search-box" placeholder="Ara / Filter..." />
    <button class="hud-btn" id="btn-zoom-in" title="Yakınlaştır">+</button>
    <button class="hud-btn" id="btn-zoom-out" title="Uzaklaştır">-</button>
    <button class="hud-btn" id="btn-fit" title="Sığdır (Reset)">Sığdır</button>
  </div>
  <div id="viewport">
    {svg}
  </div>
</div>

<script>
(function() {{
  const container = document.getElementById('viewer-container');
  const viewport = document.getElementById('viewport');
  const svg = viewport.querySelector('svg');
  const searchInput = document.getElementById('search-input');
  
  if (!svg) return;

  let scale = 1;
  let pointX = 0;
  let pointY = 0;
  let isPanning = false;
  let startX = 0;
  let startY = 0;

  function updateTransform() {{
    viewport.style.transform = `translate(${{pointX}}px, ${{pointY}}px) scale(${{scale}})`;
  }}

  function fit() {{
    const rect = svg.getBoundingClientRect();
    const cWidth = container.clientWidth;
    const cHeight = container.clientHeight;
    const sWidth = svg.viewBox ? svg.viewBox.baseVal.width : rect.width;
    const sHeight = svg.viewBox ? svg.viewBox.baseVal.height : rect.height;

    scale = Math.min((cWidth - 64) / sWidth, (cHeight - 64) / sHeight, 1.2);
    pointX = (cWidth - sWidth * scale) / 2;
    pointY = (cHeight - sHeight * scale) / 2;
    updateTransform();
  }}

  window.addEventListener('resize', fit);
  fit();

  container.addEventListener('mousedown', (e) => {{
    if (e.target.closest('.hud-panel')) return;
    isPanning = true;
    startX = e.clientX - pointX;
    startY = e.clientY - pointY;
  }});

  window.addEventListener('mousemove', (e) => {{
    if (!isPanning) return;
    pointX = e.clientX - startX;
    pointY = e.clientY - startY;
    updateTransform();
  }});

  window.addEventListener('mouseup', () => {{
    isPanning = false;
  }});

  container.addEventListener('wheel', (e) => {{
    e.preventDefault();
    const xs = (e.clientX - pointX) / scale;
    const ys = (e.clientY - pointY) / scale;
    const delta = -e.deltaY;
    if (delta > 0) {{
      scale = Math.min(scale * 1.15, 6.0);
    }} else {{
      scale = Math.max(scale / 1.15, 0.2);
    }}
    pointX = e.clientX - xs * scale;
    pointY = e.clientY - ys * scale;
    updateTransform();
  }}, {{ passive: false }});

  document.getElementById('btn-zoom-in').addEventListener('click', () => {{
    scale = Math.min(scale * 1.25, 6.0);
    updateTransform();
  }});

  document.getElementById('btn-zoom-out').addEventListener('click', () => {{
    scale = Math.max(scale / 1.25, 0.2);
    updateTransform();
  }});

  document.getElementById('btn-fit').addEventListener('click', fit);

  searchInput.addEventListener('input', (e) => {{
    const term = e.target.value.trim().toLowerCase();
    const nodes = svg.querySelectorAll('.node');
    if (!term) {{
      nodes.forEach(n => n.classList.remove('dimmed', 'highlighted'));
      return;
    }}
    nodes.forEach(n => {{
      const label = (n.getAttribute('data-node-label') || n.textContent || '').toLowerCase();
      if (label.includes(term)) {{
        n.classList.remove('dimmed');
        n.classList.add('highlighted');
      }} else {{
        n.classList.add('dimmed');
        n.classList.remove('highlighted');
      }}
    }});
  }});

  let activeLegend = null;
  const legendItems = svg.querySelectorAll('.legend-item');
  legendItems.forEach(item => {{
    item.addEventListener('click', (e) => {{
      e.stopPropagation();
      const kind = item.getAttribute('data-legend-kind');
      const nodes = svg.querySelectorAll('.node');
      
      if (activeLegend === kind) {{
        activeLegend = null;
        nodes.forEach(n => n.classList.remove('dimmed', 'highlighted'));
      }} else {{
        activeLegend = kind;
        nodes.forEach(n => {{
          const nodeRole = n.getAttribute('data-node-kind') || '';
          const nodeStatus = n.getAttribute('data-node-status') || '';
          if (nodeRole.includes(kind) || nodeStatus.includes(kind) || n.classList.contains(`c-${{kind}}`)) {{
            n.classList.remove('dimmed');
            n.classList.add('highlighted');
          }} else {{
            n.classList.add('dimmed');
            n.classList.remove('highlighted');
          }}
        }});
      }}
    }});
  }});
}})();
</script>
</body>
</html>
"##,
        title = esc(title),
        svg = svg
    )
}
