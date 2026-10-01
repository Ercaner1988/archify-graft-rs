//! Interactive Archify & Graft studio (egui): toolbar, canvas, overlays.

use crate::canvas::{self, Cx};
use crate::strings::strings;
use crate::view_data::ViewData;
use crate::TrilingualUi;
use archify_egui_paint::theme::c;
use archify_ir::{ArchitectureDiagram, DataflowDiagram, VisualPreset};
use archify_motion::MotionSettings;
use archify_scene::{dataflow_scene, Scene};
use archify_style::{Theme, Tokens};
use egui::{Frame, Key, Margin, RichText, Stroke};

/// Which diagram the central canvas currently draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiagramView {
    #[default]
    Architecture,
    Dataflow,
}

/// Called on double click of a box when a deeper level exists. Receives the box id and may
/// return the diagram of that box's inside (for example the modules of a crate).
pub type DrillHook = Box<dyn FnMut(&str) -> Option<ArchitectureDiagram>>;

pub struct ArchifyApp {
    pub diagram: Option<ArchitectureDiagram>,
    pub dataflow: Option<DataflowDiagram>,
    pub view: DiagramView,
    pub locale: String,
    pub preset: VisualPreset,
    pub theme: Theme,
    pub search_text: String,
    pub motion: MotionSettings,
    pub on_drill: Option<DrillHook>,
    tokens: Tokens,
    arch: Option<ViewData>,
    flow: Option<ViewData>,
    /// Levels above the current one when drilled in: `(diagram, view)`.
    history: Vec<(Option<ArchitectureDiagram>, Option<ViewData>)>,
    clock: f32,
    route_mode: bool,
    status: String,
}

impl Default for ArchifyApp {
    fn default() -> Self {
        Self::new(None, None, "tr")
    }
}

impl ArchifyApp {
    pub fn new(
        diagram: Option<ArchitectureDiagram>,
        dataflow: Option<DataflowDiagram>,
        locale: &str,
    ) -> Self {
        let motion = MotionSettings {
            ambient: true,
            ..MotionSettings::default()
        };
        let arch = diagram
            .as_ref()
            .map(|d| ViewData::new(Scene::from_architecture(d), motion.reduced_motion));
        let flow = dataflow
            .as_ref()
            .map(|d| ViewData::new(dataflow_scene(d), motion.reduced_motion));
        let preset = VisualPreset::Editorial;
        let theme = Theme::Dark;
        Self {
            diagram,
            dataflow,
            view: DiagramView::default(),
            locale: locale.to_string(),
            preset,
            theme,
            search_text: String::new(),
            motion,
            on_drill: None,
            tokens: Tokens::new(preset, theme),
            arch,
            flow,
            history: Vec::new(),
            clock: 0.0,
            route_mode: false,
            status: String::new(),
        }
    }

    /// Replaces the architecture diagram (resets the camera and the tour).
    pub fn set_diagram(&mut self, diagram: ArchitectureDiagram) {
        self.arch = Some(ViewData::new(
            Scene::from_architecture(&diagram),
            self.motion.reduced_motion,
        ));
        self.diagram = Some(diagram);
        self.view = DiagramView::Architecture;
    }

    fn current(&mut self) -> Option<&mut ViewData> {
        match self.view {
            DiagramView::Architecture => self.arch.as_mut(),
            DiagramView::Dataflow => self.flow.as_mut(),
        }
    }

    fn sync_tokens(&mut self) {
        if self.tokens.preset != self.preset || self.tokens.theme != self.theme {
            self.tokens = Tokens::new(self.preset, self.theme);
        }
    }

    fn drill(&mut self, id: &str) {
        let Some(hook) = self.on_drill.as_mut() else {
            return;
        };
        if let Some(inner) = hook(id) {
            let scene = Scene::from_architecture(&inner);
            let view = ViewData::new(scene, self.motion.reduced_motion);
            let old_view = self.arch.replace(view);
            let old_diagram = self.diagram.replace(inner);
            self.history.push((old_diagram, old_view));
            self.view = DiagramView::Architecture;
        }
    }

    fn global_keys(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let (theme, style, route) = ctx.input(|i| {
            (
                i.key_pressed(Key::T),
                i.key_pressed(Key::S),
                i.key_pressed(Key::R),
            )
        });
        if theme {
            self.theme = self.theme.toggled();
        }
        if style {
            self.preset = next_preset(self.preset);
        }
        if route {
            self.route_mode = !self.route_mode;
        }
    }

    /// Primary UI function, called every frame by the host (eframe or an embedding app).
    pub fn render_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let dt = ctx.input(|i| i.stable_dt).clamp(0.0, 0.1);
        self.clock += dt;
        self.global_keys(&ctx);
        self.sync_tokens();
        let t = self.tokens.clone();
        let s = strings(&self.locale);

        egui::Panel::bottom("archify_statusbar")
            .frame(
                Frame::new()
                    .fill(c(t.panel))
                    .stroke(Stroke::new(1.0, c(t.panel_border)))
                    .inner_margin(Margin::symmetric(10, 4)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    archify_egui_paint::theme::style_ui(ui, &t);
                    ui.style_mut().visuals.override_text_color = Some(c(t.muted));
                    let text = if self.route_mode {
                        let (start, target, active) = self.route_state();
                        TrilingualUi::route_label(
                            &self.locale,
                            start.as_deref(),
                            target.as_deref(),
                            active,
                        )
                    } else if self.status.is_empty() {
                        s.hint.to_string()
                    } else {
                        self.status.clone()
                    };
                    ui.label(RichText::new(text).font(archify_egui_paint::paint::mono(11.0)));
                });
            });

        self.toolbar(ui, &t);

        let locale_now = self.locale.clone();
        if let Some(vd) = self.current() {
            if vd.tour.is_active() {
                crate::overlay::tour_bar(ui, &t, vd, &locale_now);
            }
        }
        let mut drill = None;
        let mut status = String::new();
        egui::CentralPanel::default()
            .frame(Frame::new().fill(c(t.bg)))
            .show(ui, |ui| {
                let (search, locale, motion, clock, route_mode) = (
                    self.search_text.clone(),
                    self.locale.clone(),
                    self.motion,
                    self.clock,
                    self.route_mode,
                );
                let can_drill = self.on_drill.is_some();
                if let Some(vd) = self.current() {
                    let cx = Cx {
                        tokens: &t,
                        motion,
                        clock,
                        dt,
                        search: &search,
                        locale: &locale,
                        route_mode,
                        can_drill,
                    };
                    let out = canvas::show(ui, vd, &cx);
                    drill = out.drill;
                    status = out.status;
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            RichText::new(TrilingualUi::search_label(&locale)).color(c(t.muted)),
                        );
                    });
                }
            });
        self.status = status;
        if let Some(id) = drill {
            self.drill(&id);
        }
    }

    fn route_state(&mut self) -> (Option<String>, Option<String>, bool) {
        let Some(vd) = self.current() else {
            return (None, None, false);
        };
        let name = |i: usize| vd.scene.nodes[i].label.text.clone();
        let start = vd.route_start.map(name);
        let (target, active) = match (vd.route.first(), vd.route.last()) {
            (Some(_), Some(&l)) if vd.route.len() > 1 => (Some(name(l)), true),
            _ => (None, false),
        };
        let start = start.or_else(|| vd.route.first().map(|&i| name(i)));
        (start, target, active)
    }

    fn toolbar(&mut self, ui: &mut egui::Ui, t: &Tokens) {
        let s = strings(&self.locale);
        egui::Panel::top("archify_toolbar")
            .frame(
                Frame::new()
                    .fill(c(t.panel))
                    .stroke(Stroke::new(1.0, c(t.panel_border)))
                    .inner_margin(Margin::symmetric(10, 6)),
            )
            .show(ui, |ui| {
                archify_egui_paint::theme::style_ui(ui, t);
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(s.title).strong());
                    ui.separator();
                    egui::ComboBox::from_id_salt("archify_preset")
                        .selected_text(Tokens::preset_name(self.preset))
                        .show_ui(ui, |ui| {
                            for p in [
                                VisualPreset::Editorial,
                                VisualPreset::Classic,
                                VisualPreset::SignalFlow,
                                VisualPreset::Blueprint,
                            ] {
                                ui.selectable_value(&mut self.preset, p, Tokens::preset_name(p));
                            }
                        });
                    let theme_label = if self.theme == Theme::Dark {
                        s.dark
                    } else {
                        s.light
                    };
                    if ui.button(theme_label).on_hover_text("T").clicked() {
                        self.theme = self.theme.toggled();
                    }
                    ui.separator();
                    if self.dataflow.is_some() && self.diagram.is_some() {
                        ui.selectable_value(
                            &mut self.view,
                            DiagramView::Architecture,
                            s.architecture,
                        );
                        ui.selectable_value(&mut self.view, DiagramView::Dataflow, s.dataflow);
                        ui.separator();
                    }
                    if !self.history.is_empty() && ui.button(format!("◀ {}", s.back)).clicked() {
                        if let Some((d, v)) = self.history.pop() {
                            self.diagram = d;
                            self.arch = v;
                        }
                    }
                    if ui.button(s.fit).on_hover_text("F").clicked() {
                        if let Some(vd) = self.current() {
                            let vp = vd.vs.viewport;
                            vd.vs.fly_to_rect(
                                vd.content_rect(),
                                vp,
                                56.0,
                                1.25,
                                archify_motion::CAMERA_SECS,
                            );
                        }
                    }
                    ui.separator();
                    ui.add(
                        egui::TextEdit::singleline(&mut self.search_text)
                            .hint_text(s.search_hint)
                            .desired_width(150.0),
                    );
                    ui.separator();
                    ui.checkbox(&mut self.motion.ambient, s.ambient);
                    if ui
                        .checkbox(&mut self.motion.reduced_motion, s.reduced)
                        .changed()
                    {
                        let r = self.motion.reduced_motion;
                        for vd in [self.arch.as_mut(), self.flow.as_mut()]
                            .into_iter()
                            .flatten()
                        {
                            vd.set_reduced_motion(r);
                        }
                    }
                    ui.separator();
                    for (code, name) in [("tr", "TR"), ("en", "EN"), ("ar", "AR")] {
                        if ui.selectable_label(self.locale == code, name).clicked() {
                            self.locale = code.to_string();
                        }
                    }
                });
            });
    }
}

fn next_preset(p: VisualPreset) -> VisualPreset {
    match p {
        VisualPreset::Editorial => VisualPreset::Classic,
        VisualPreset::Classic => VisualPreset::SignalFlow,
        VisualPreset::SignalFlow => VisualPreset::Blueprint,
        VisualPreset::Blueprint => VisualPreset::Editorial,
    }
}

#[cfg(any(feature = "glow", feature = "wgpu"))]
impl eframe::App for ArchifyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.render_ui(ui);
    }
}
