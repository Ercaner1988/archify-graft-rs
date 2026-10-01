//! archify-egui: the native Archify studio. Pure Rust; on Windows it runs on wgpu.
//!
//! Pipeline: `archify-scene` resolves layout, routing and text fitting once; this crate paints
//! the scene with the tokens of `archify-style` and animates it with `archify-motion`.

pub mod app;
mod canvas;
mod overlay;
mod search;
mod strings;
mod view_data;

pub use app::{ArchifyApp, DiagramView, DrillHook};

pub struct TrilingualUi;

impl TrilingualUi {
    pub fn title(locale: &str) -> &'static str {
        match locale {
            "ar" => "استوديو بنية النظام (Archify & Graft)",
            "tr" => "Archify Mimari Stüdyosu & Graft Kod Analitiği",
            _ => "Archify Architecture Studio & Graft Code Intelligence",
        }
    }

    pub fn search_label(locale: &str) -> &'static str {
        match locale {
            "ar" => "ابحث في الكود أو البنية...",
            "tr" => "Kod veya mimaride ara...",
            _ => "Search code or architecture...",
        }
    }

    pub fn route_label(
        locale: &str,
        start: Option<&str>,
        target: Option<&str>,
        active: bool,
    ) -> String {
        match (locale, start, target) {
            ("ar", Some(s), Some(t)) if active => format!("المسار النشط: {s} ➔ {t}"),
            ("ar", Some(s), Some(t)) => format!("لا يوجد مسار بين {s} و {t}"),
            ("ar", Some(s), None) => format!("البداية: {s}. اختر الهدف..."),
            ("ar", None, _) => "انقر على عقدتين لتتبع المسار المباشر".to_string(),
            ("tr", Some(s), Some(t)) if active => format!("Aktif Rota: {s} ➔ {t}"),
            ("tr", Some(s), Some(t)) => format!("{s} ile {t} arasında rota bulunamadı"),
            ("tr", Some(s), None) => format!("Başlangıç: {s}. Hedef düğümü seçin..."),
            ("tr", None, _) => "Rota için iki kutuya tıklayın (R: kapat)".to_string(),
            (_, Some(s), Some(t)) if active => format!("Active Route: {s} ➔ {t}"),
            (_, Some(s), Some(t)) => format!("No route between {s} and {t}"),
            (_, Some(s), None) => format!("Start: {s}. Select target..."),
            (_, None, _) => "Click two boxes to probe the shortest route (R: off)".to_string(),
        }
    }
}

#[cfg(any(feature = "glow", feature = "wgpu"))]
pub fn run_desktop(
    diagram: Option<archify_ir::ArchitectureDiagram>,
    dataflow: Option<archify_ir::DataflowDiagram>,
    locale: &str,
) -> eframe::Result<()> {
    run_desktop_with_drill(diagram, dataflow, locale, None)
}

#[cfg(any(feature = "glow", feature = "wgpu"))]
pub fn run_desktop_with_drill(
    diagram: Option<archify_ir::ArchitectureDiagram>,
    dataflow: Option<archify_ir::DataflowDiagram>,
    locale: &str,
    on_drill: Option<DrillHook>,
) -> eframe::Result<()> {
    // Windows'ta glow/OpenGL pencereyi kara açar (Wgl sRGB swap chain) — kesin kural: wgpu.
    let native_options = eframe::NativeOptions {
        #[cfg(feature = "wgpu")]
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1160.0, 700.0])
            .with_title(TrilingualUi::title(locale))
            .with_transparent(false),
        multisampling: 0,
        depth_buffer: 0,
        stencil_buffer: 0,
        ..Default::default()
    };
    let loc = locale.to_string();
    eframe::run_native(
        TrilingualUi::title(locale),
        native_options,
        Box::new(move |_cc| {
            let mut app = ArchifyApp::new(diagram, dataflow, &loc);
            app.on_drill = on_drill;
            Ok(Box::new(app))
        }),
    )
}
