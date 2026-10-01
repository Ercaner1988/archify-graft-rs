//! Zoom/pan with cursor-centred, frame-rate independent smoothing plus a
//! cubic-ease-out camera tween.
//!
//! Convention: `screen = pan + world * zoom`, both relative to the viewport's
//! top-left corner (`pan` is where the world origin sits on screen).

use crate::ease::{cubic_out, exp_follow};

/// Default camera tween length.
pub const CAMERA_SECS: f32 = 0.42;
/// Camera tween length while following a tour step.
pub const TOUR_CAMERA_SECS: f32 = 0.32;
const ZOOM_RATE: f32 = 14.0;

#[derive(Debug, Clone, Copy)]
struct Camera {
    from_zoom: f32,
    to_zoom: f32,
    from_center: [f32; 2],
    to_center: [f32; 2],
    elapsed: f32,
    secs: f32,
}

#[derive(Debug, Clone)]
pub struct ViewState {
    pub zoom: f32,
    pub pan: [f32; 2],
    pub min_zoom: f32,
    pub max_zoom: f32,
    /// Last known viewport size; used to interpolate camera tweens through the
    /// world-space centre. Set by `set_viewport` / `fly_to_rect`.
    pub viewport: [f32; 2],
    target_zoom: f32,
    anchor_screen: [f32; 2],
    anchor_world: [f32; 2],
    camera: Option<Camera>,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: [0.0, 0.0],
            min_zoom: 0.05,
            max_zoom: 6.0,
            viewport: [0.0, 0.0],
            target_zoom: 1.0,
            anchor_screen: [0.0, 0.0],
            anchor_world: [0.0, 0.0],
            camera: None,
        }
    }
}

/// Multiplier for a scroll amount in points (event based, so frame-rate independent).
pub fn wheel_factor(scroll_y_points: f32) -> f32 {
    (scroll_y_points * 0.0015).exp()
}

impl ViewState {
    pub fn new(zoom: f32, pan: [f32; 2]) -> Self {
        Self {
            zoom,
            pan,
            target_zoom: zoom,
            ..Self::default()
        }
    }

    pub fn set_viewport(&mut self, viewport: [f32; 2]) {
        self.viewport = viewport;
    }

    pub fn screen_to_world(&self, p: [f32; 2]) -> [f32; 2] {
        [
            (p[0] - self.pan[0]) / self.zoom,
            (p[1] - self.pan[1]) / self.zoom,
        ]
    }

    pub fn world_to_screen(&self, p: [f32; 2]) -> [f32; 2] {
        [
            self.pan[0] + p[0] * self.zoom,
            self.pan[1] + p[1] * self.zoom,
        ]
    }

    /// Multiply the zoom target by `factor`, keeping the world point under `cursor` fixed.
    pub fn zoom_at(&mut self, cursor: [f32; 2], factor: f32) {
        self.camera = None;
        self.target_zoom = (self.target_zoom * factor).clamp(self.min_zoom, self.max_zoom);
        self.anchor_screen = cursor;
        self.anchor_world = self.screen_to_world(cursor);
    }

    /// Drag-pan. Cancels the camera tween and any pending zoom target.
    pub fn pan_by(&mut self, delta: [f32; 2]) {
        self.camera = None;
        self.target_zoom = self.zoom;
        self.pan[0] += delta[0];
        self.pan[1] += delta[1];
    }

    pub fn interrupt_camera(&mut self) {
        self.camera = None;
        self.target_zoom = self.zoom;
    }

    /// Instantly set zoom and pan.
    pub fn set(&mut self, zoom: f32, pan: [f32; 2]) {
        self.camera = None;
        self.zoom = zoom;
        self.target_zoom = zoom;
        self.pan = pan;
    }

    pub fn is_animating(&self) -> bool {
        self.camera.is_some() || (self.zoom - self.target_zoom).abs() > 0.0
    }

    fn center(&self, zoom: f32, pan: [f32; 2]) -> [f32; 2] {
        [
            (self.viewport[0] / 2.0 - pan[0]) / zoom,
            (self.viewport[1] / 2.0 - pan[1]) / zoom,
        ]
    }

    /// Zoom + pan that fit `rect = [x, y, w, h]` (world) into `viewport` with `padding` px.
    pub fn fit_rect(
        rect: [f32; 4],
        viewport: [f32; 2],
        padding: f32,
        max_zoom: f32,
    ) -> (f32, [f32; 2]) {
        let w = rect[2].max(1e-3);
        let h = rect[3].max(1e-3);
        let zx = (viewport[0] - 2.0 * padding).max(1.0) / w;
        let zy = (viewport[1] - 2.0 * padding).max(1.0) / h;
        let zoom = zx.min(zy).min(max_zoom);
        let (cx, cy) = (rect[0] + w / 2.0, rect[1] + h / 2.0);
        (
            zoom,
            [viewport[0] / 2.0 - cx * zoom, viewport[1] / 2.0 - cy * zoom],
        )
    }

    /// Animate to `target_zoom` / `target_pan` (cubic ease-out; zoom in log space,
    /// pan through the world-space viewport centre).
    pub fn fly_to(&mut self, target_zoom: f32, target_pan: [f32; 2], secs: f32) {
        let to_zoom = target_zoom.clamp(self.min_zoom, self.max_zoom);
        if secs <= 0.0 {
            self.set(to_zoom, target_pan);
            return;
        }
        self.camera = Some(Camera {
            from_zoom: self.zoom,
            to_zoom,
            from_center: self.center(self.zoom, self.pan),
            to_center: self.center(to_zoom, target_pan),
            elapsed: 0.0,
            secs,
        });
        self.target_zoom = to_zoom;
    }

    pub fn fly_to_rect(
        &mut self,
        rect: [f32; 4],
        viewport: [f32; 2],
        padding: f32,
        max_zoom: f32,
        secs: f32,
    ) {
        self.viewport = viewport;
        let (z, pan) = Self::fit_rect(rect, viewport, padding, max_zoom.min(self.max_zoom));
        self.fly_to(z.max(self.min_zoom), pan, secs);
    }

    /// Advance by `dt`; returns `true` while animating.
    pub fn update(&mut self, dt: f32) -> bool {
        if let Some(mut cam) = self.camera {
            cam.elapsed += dt.max(0.0);
            let p = (cam.elapsed / cam.secs).min(1.0);
            let e = cubic_out(p);
            let zoom = (cam.from_zoom.ln() + (cam.to_zoom.ln() - cam.from_zoom.ln()) * e).exp();
            let c = [
                cam.from_center[0] + (cam.to_center[0] - cam.from_center[0]) * e,
                cam.from_center[1] + (cam.to_center[1] - cam.from_center[1]) * e,
            ];
            if p >= 1.0 {
                self.zoom = cam.to_zoom;
                self.camera = None;
            } else {
                self.zoom = zoom;
                self.camera = Some(cam);
            }
            self.pan = [
                self.viewport[0] / 2.0 - c[0] * self.zoom,
                self.viewport[1] / 2.0 - c[1] * self.zoom,
            ];
            return self.camera.is_some();
        }
        if (self.zoom - self.target_zoom).abs() == 0.0 {
            return false;
        }
        self.zoom += (self.target_zoom - self.zoom) * exp_follow(dt, ZOOM_RATE);
        if (self.zoom - self.target_zoom).abs() < 0.002 * self.target_zoom {
            self.zoom = self.target_zoom;
        }
        self.pan = [
            self.anchor_screen[0] - self.anchor_world[0] * self.zoom,
            self.anchor_screen[1] - self.anchor_world[1] * self.zoom,
        ];
        self.zoom != self.target_zoom
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_to_2x_takes_at_least_15_frames_to_reach_1_percent() {
        let mut v = ViewState::default();
        v.zoom_at([100.0, 100.0], 2.0);
        let mut n = 0;
        while (v.zoom - 2.0).abs() > 0.02 {
            v.update(1.0 / 60.0);
            n += 1;
        }
        assert!(n >= 15, "reached 1% in {n} frames");
        let mut frames = 0;
        while v.update(1.0 / 60.0) {
            frames += 1;
        }
        assert_eq!(v.zoom, 2.0);
        assert!(frames > 0);
    }

    #[test]
    fn frame_rate_independent() {
        let at = |fps: f32| {
            let mut v = ViewState::default();
            v.zoom_at([10.0, 10.0], 2.0);
            for _ in 0..(0.2 * fps).round() as usize {
                v.update(1.0 / fps);
            }
            v.zoom
        };
        let z60 = at(60.0);
        assert!((at(30.0) / z60 - 1.0).abs() < 0.02);
        assert!((at(120.0) / z60 - 1.0).abs() < 0.02);
    }

    #[test]
    fn cursor_world_point_is_stable() {
        let mut v = ViewState::new(1.3, [40.0, -25.0]);
        let cursor = [321.0, 207.0];
        let w0 = v.screen_to_world(cursor);
        v.zoom_at(cursor, 2.5);
        while v.update(1.0 / 60.0) {
            let s = v.world_to_screen(w0);
            assert!((s[0] - cursor[0]).abs() < 0.01 && (s[1] - cursor[1]).abs() < 0.01);
        }
        let s = v.world_to_screen(w0);
        assert!((s[0] - cursor[0]).abs() < 0.01);
    }

    #[test]
    fn zoom_is_clamped() {
        let mut v = ViewState::default();
        v.zoom_at([0.0, 0.0], 1000.0);
        while v.update(1.0 / 60.0) {}
        assert_eq!(v.zoom, v.max_zoom);
    }

    #[test]
    fn fly_to_ends_exactly() {
        let mut v = ViewState::default();
        v.set_viewport([800.0, 600.0]);
        v.fly_to(2.0, [-100.0, -50.0], CAMERA_SECS);
        let mut n = 0;
        while v.update(1.0 / 60.0) {
            n += 1;
        }
        assert!(n >= 20);
        assert_eq!(v.zoom, 2.0);
        assert!((v.pan[0] + 100.0).abs() < 1e-3 && (v.pan[1] + 50.0).abs() < 1e-3);
        assert!(!v.is_animating());
    }

    #[test]
    fn pan_interrupts_camera() {
        let mut v = ViewState::default();
        v.fly_to(3.0, [0.0, 0.0], 1.0);
        v.update(0.1);
        v.pan_by([5.0, 5.0]);
        assert!(!v.is_animating());
        assert!(!v.update(0.1));
    }

    #[test]
    fn fit_rect_contains_content_with_padding() {
        let rect = [100.0, 50.0, 400.0, 200.0];
        let vp = [800.0, 600.0];
        let (z, pan) = ViewState::fit_rect(rect, vp, 48.0, 10.0);
        let v = ViewState::new(z, pan);
        let a = v.world_to_screen([rect[0], rect[1]]);
        let b = v.world_to_screen([rect[0] + rect[2], rect[1] + rect[3]]);
        assert!(a[0] >= 48.0 - 1e-3 && a[1] >= 48.0 - 1e-3);
        assert!(b[0] <= vp[0] - 48.0 + 1e-3 && b[1] <= vp[1] - 48.0 + 1e-3);
        let (z2, _) = ViewState::fit_rect(rect, vp, 48.0, 1.0);
        assert_eq!(z2, 1.0);
    }
}
