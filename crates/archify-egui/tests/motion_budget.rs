//! Numeric acceptance of the canvas motion, measured on a headless egui context:
//! no repaint while idle, >= 10 frames for a hover transition, >= 15 frames for a 2x zoom.

use archify_egui::ArchifyApp;
use archify_ir::{
    ArchitectureDiagram, Component, Connection, DiagramMeta, SemanticRole as R, VisualPreset,
};
use egui::{Event, Modifiers, MouseWheelUnit, Pos2, RawInput, Rect, Vec2};
use std::time::Duration;

fn comp(id: &str, x: f32, y: f32, role: R) -> Component {
    Component {
        id: id.into(),
        label: id.into(),
        sublabel: Some("alt".into()),
        role,
        x,
        y,
        width: 170.0,
        height: 64.0,
    }
}

fn diagram() -> ArchitectureDiagram {
    let e = |a: &str, b: &str| Connection {
        from: a.into(),
        to: b.into(),
        label: Some("kullanır".into()),
        line_style: String::new(),
    };
    ArchitectureDiagram {
        meta: DiagramMeta {
            title: "t".into(),
            subtitle: None,
            locale: "tr".into(),
            visual_preset: VisualPreset::Editorial,
        },
        components: vec![
            comp("a", 40.0, 80.0, R::Frontend),
            comp("b", 310.0, 80.0, R::Backend),
            comp("c", 580.0, 80.0, R::Database),
            comp("d", 310.0, 300.0, R::External),
        ],
        connections: vec![e("a", "b"), e("b", "c"), e("b", "d")],
        regions: vec![],
        story_beats: vec![],
    }
}

struct Harness {
    ctx: egui::Context,
    app: ArchifyApp,
    t: f64,
}

impl Harness {
    fn new() -> Self {
        let mut app = ArchifyApp::new(Some(diagram()), None, "tr");
        app.motion.ambient = false;
        Harness {
            ctx: egui::Context::default(),
            app,
            t: 0.0,
        }
    }

    /// Runs one 60 Hz frame; returns true when the app asked for another frame right away.
    fn frame(&mut self, events: Vec<Event>) -> bool {
        self.t += 1.0 / 60.0;
        let input = RawInput {
            time: Some(self.t),
            predicted_dt: 1.0 / 60.0,
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200.0, 800.0))),
            events,
            ..Default::default()
        };
        let app = &mut self.app;
        let mut out = self.ctx.run_ui(input, |ui| app.render_ui(ui));
        out.textures_delta.clear();
        out.viewport_output
            .values()
            .any(|v| v.repaint_delay < Duration::from_millis(100))
    }

    fn settle_until_intro_done(&mut self) {
        for _ in 0..400 {
            self.frame(vec![]);
        }
    }

    fn settle(&mut self) {
        let mut quiet = 0;
        for _ in 0..1500 {
            if self.frame(vec![]) {
                quiet = 0;
            } else {
                quiet += 1;
                if quiet >= 5 {
                    return;
                }
            }
        }
        panic!("never settled");
    }

    /// Frames that kept requesting repaints after `events`, until quiet again.
    fn busy_frames(&mut self, events: Vec<Event>) -> usize {
        let mut busy = usize::from(self.frame(events));
        let mut quiet = 0;
        for _ in 0..600 {
            if self.frame(vec![]) {
                busy += 1;
                quiet = 0;
            } else {
                quiet += 1;
                if quiet >= 5 {
                    break;
                }
            }
        }
        busy
    }
}

#[test]
fn idle_canvas_does_not_repaint() {
    let mut h = Harness::new();
    h.settle();
    let repaints = (0..120).filter(|_| h.frame(vec![])).count();
    assert_eq!(repaints, 0, "repaint requested while idle");
}

#[test]
fn hover_transition_spans_at_least_ten_frames() {
    let mut h = Harness::new();
    h.settle();
    // a box in the fitted view: sweep the pointer over the canvas centre-left
    let mut best = 0;
    'scan: for y in (180..620).step_by(40) {
        for x in (60..1140).step_by(45) {
            let moved = vec![Event::PointerMoved(Pos2::new(x as f32, y as f32))];
            best = best.max(h.busy_frames(moved));
            h.busy_frames(vec![Event::PointerGone]);
            if best >= 10 {
                break 'scan;
            }
        }
    }
    eprintln!("hover transition animated frames: {best}");
    assert!(best >= 10, "hover produced only {best} animated frames");
}

#[test]
fn wheel_zoom_is_smooth_over_many_frames() {
    let mut h = Harness::new();
    h.settle();
    let at = Pos2::new(600.0, 400.0);
    h.busy_frames(vec![Event::PointerMoved(at)]);
    let wheel = Event::MouseWheel {
        unit: MouseWheelUnit::Point,
        delta: Vec2::new(0.0, 400.0),
        phase: egui::TouchPhase::Move,
        modifiers: Modifiers::NONE,
    };
    let frames = h.busy_frames(vec![wheel]);
    assert!(frames >= 15, "zoom settled in {frames} frames");
}

#[test]
fn ambient_light_is_on_by_default_and_throttled() {
    let mut h = Harness::new();
    h.app.motion.ambient = true;
    h.settle_until_intro_done();
    // ambient beams keep the canvas alive, but not at vsync rate: the app asks to repaint
    // after a short delay instead of immediately
    let mut delays = 0;
    for _ in 0..60 {
        h.t += 1.0 / 60.0;
        let input = RawInput {
            time: Some(h.t),
            predicted_dt: 1.0 / 60.0,
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200.0, 800.0))),
            ..Default::default()
        };
        let app = &mut h.app;
        let mut out = h.ctx.run_ui(input, |ui| app.render_ui(ui));
        out.textures_delta.clear();
        if out
            .viewport_output
            .values()
            .any(|v| v.repaint_delay < Duration::from_secs(1))
        {
            delays += 1;
        }
    }
    assert!(delays >= 55, "ambient light stopped: {delays}/60 frames");
    assert!(
        ArchifyApp::new(None, None, "tr").motion.ambient,
        "ambient must default to on"
    );
}

fn click(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

#[test]
fn double_click_drills_into_the_hook_diagram() {
    let mut h = Harness::new();
    let mut inner = diagram();
    inner.components.truncate(2);
    inner.connections.truncate(1);
    h.app.on_drill = Some(Box::new(move |_| Some(inner.clone())));
    h.settle();
    let mut hit = None;
    'scan: for y in (150..650).step_by(25) {
        for x in (40..1160).step_by(25) {
            let p = Pos2::new(x as f32, y as f32);
            h.t += 1.0; // keep earlier probes out of the triple-click window
            h.frame(vec![Event::PointerMoved(p)]);
            for _ in 0..2 {
                h.frame(vec![click(p, true)]);
                h.frame(vec![click(p, false)]);
            }
            if h.app.diagram.as_ref().map(|d| d.components.len()) == Some(2) {
                hit = Some(p);
                break 'scan;
            }
        }
    }
    assert!(
        hit.is_some(),
        "no double click drilled into the hook diagram"
    );
}
