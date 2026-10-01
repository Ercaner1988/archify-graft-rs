//! Scalar value that eases toward a target over a fixed duration.

use crate::ease::css_ease;

/// Hover / focus transition length (CSS used .18s; .20 gives >= 10 frames at 60 fps).
pub const HOVER_SECS: f32 = 0.20;
/// Opacity of non-related items while hovering (Intent Trace).
pub const DIM_HOVER: f32 = 0.2;
/// Opacity of non-related items while a node is selected (focus).
pub const DIM_FOCUS: f32 = 0.13;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transition {
    from: f32,
    to: f32,
    value: f32,
    elapsed: f32,
    duration: f32,
}

impl Transition {
    pub fn new(v: f32) -> Self {
        Self {
            from: v,
            to: v,
            value: v,
            elapsed: 0.0,
            duration: 0.0,
        }
    }

    /// Start easing from the *current* value to `to`. No-op if `to` is already the
    /// target; `duration <= 0` snaps.
    pub fn set_target(&mut self, to: f32, duration: f32) {
        if (to - self.to).abs() < 1e-6 {
            return;
        }
        if duration <= 0.0 {
            self.snap(to);
            return;
        }
        self.from = self.value;
        self.to = to;
        self.elapsed = 0.0;
        self.duration = duration;
    }

    /// Advance by `dt`; returns `true` while still animating.
    pub fn update(&mut self, dt: f32) -> bool {
        if !self.is_animating() {
            return false;
        }
        self.elapsed += dt.max(0.0);
        if self.elapsed >= self.duration {
            self.value = self.to;
            return false;
        }
        self.value = self.from + (self.to - self.from) * css_ease(self.elapsed / self.duration);
        true
    }

    pub fn snap(&mut self, v: f32) {
        *self = Self::new(v);
    }

    pub fn value(&self) -> f32 {
        self.value
    }

    pub fn target(&self) -> f32 {
        self.to
    }

    pub fn is_animating(&self) -> bool {
        (self.value - self.to).abs() > 1e-6
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hover_yields_at_least_ten_intermediate_frames_at_60fps() {
        let mut t = Transition::new(0.0);
        t.set_target(1.0, HOVER_SECS);
        let mut mid = 0;
        for _ in 0..30 {
            t.update(1.0 / 60.0);
            if t.value() > 0.0 && t.value() < 1.0 {
                mid += 1;
            }
        }
        assert!(mid >= 10, "only {mid} intermediate frames");
        assert_eq!(t.value(), 1.0);
        assert!(!t.is_animating());
    }

    #[test]
    fn retarget_starts_from_current_and_noop_on_same_target() {
        let mut t = Transition::new(0.0);
        t.set_target(1.0, 0.2);
        for _ in 0..5 {
            t.update(1.0 / 60.0);
        }
        let mid = t.value();
        t.set_target(1.0, 0.2); // no-op
        assert_eq!(t.value(), mid);
        t.set_target(0.0, 0.2);
        t.update(0.0);
        assert!((t.value() - mid).abs() < 1e-4);
        assert!(t.update(0.01));
    }

    #[test]
    fn zero_duration_snaps() {
        let mut t = Transition::new(0.3);
        t.set_target(0.9, 0.0);
        assert_eq!(t.value(), 0.9);
        assert!(!t.update(0.016));
    }
}
