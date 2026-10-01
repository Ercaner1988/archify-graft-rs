//! One-shot opening "trace": edges draw on in sequence, nodes pulse once.

use crate::ease::smoothstep;

const STEP_DELAY: f32 = 0.16;
const MAX_STEP: usize = 12;
const EDGE_SECS: f32 = 2.4;
const NODE_SECS: f32 = 3.6;

#[derive(Debug, Clone, Copy, Default)]
pub struct IntroTrace {
    skipped: bool,
}

fn delay(i: usize) -> f32 {
    i.min(MAX_STEP) as f32 * STEP_DELAY
}

impl IntroTrace {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reduced-motion: everything already drawn, no pulse.
    pub fn skipped() -> Self {
        Self { skipped: true }
    }

    /// Fraction of edge `i` already drawn at time `t` (linear, 0..=1).
    pub fn edge_progress(&self, i: usize, t: f32) -> f32 {
        if self.skipped {
            return 1.0;
        }
        ((t - delay(i)) / EDGE_SECS).clamp(0.0, 1.0)
    }

    /// Edge opacity: 0.42 at the start, full once 88% drawn.
    pub fn edge_opacity(&self, i: usize, t: f32) -> f32 {
        0.42 + 0.58 * (self.edge_progress(i, t) / 0.88).min(1.0)
    }

    /// Node pulse 0 -> 1 -> 0; rises until 18%, holds to 36%, falls until 72% of 3.6 s.
    pub fn node_pulse(&self, i: usize, t: f32) -> f32 {
        if self.skipped {
            return 0.0;
        }
        let p = (t - delay(i)) / NODE_SECS;
        if !(0.0..1.0).contains(&p) {
            return 0.0;
        }
        if p < 0.18 {
            smoothstep(p / 0.18)
        } else if p <= 0.36 {
            1.0
        } else if p < 0.72 {
            1.0 - smoothstep((p - 0.36) / 0.36)
        } else {
            0.0
        }
    }

    /// Time at which everything (for `n` items) has settled.
    pub fn total_secs(&self, n: usize) -> f32 {
        if self.skipped || n == 0 {
            return 0.0;
        }
        delay(n - 1) + NODE_SECS
    }

    pub fn is_done(&self, t: f32, n: usize) -> bool {
        t >= self.total_secs(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edges_are_staggered_and_complete() {
        let tr = IntroTrace::new();
        assert_eq!(tr.edge_progress(0, 0.0), 0.0);
        assert!((tr.edge_progress(0, 1.2) - 0.5).abs() < 1e-5);
        assert_eq!(tr.edge_progress(5, 0.5), 0.0); // delayed 0.8 s
        assert_eq!(tr.edge_progress(0, 2.4), 1.0);
        // index beyond 12 shares the 1.92 s delay
        assert_eq!(tr.edge_progress(40, 1.0), tr.edge_progress(12, 1.0));
        assert!((tr.edge_opacity(0, 0.0) - 0.42).abs() < 1e-6);
        assert!((tr.edge_opacity(0, 5.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn node_pulse_peaks_and_returns_to_zero() {
        let tr = IntroTrace::new();
        assert_eq!(tr.node_pulse(0, 0.0), 0.0);
        assert_eq!(tr.node_pulse(0, 3.6 * 0.25), 1.0);
        let mid = tr.node_pulse(0, 3.6 * 0.5);
        assert!(mid > 0.0 && mid < 1.0);
        assert_eq!(tr.node_pulse(0, 3.6 * 0.8), 0.0);
        assert_eq!(tr.node_pulse(0, 10.0), 0.0);
    }

    #[test]
    fn done_and_skipped() {
        let tr = IntroTrace::new();
        assert!(!tr.is_done(3.0, 20));
        assert!(tr.is_done(tr.total_secs(20), 20));
        assert!((tr.total_secs(20) - (1.92 + 3.6)).abs() < 1e-5);
        let s = IntroTrace::skipped();
        assert!(s.is_done(0.0, 50));
        assert_eq!(s.edge_progress(3, 0.0), 1.0);
        assert_eq!(s.node_pulse(3, 1.0), 0.0);
    }
}
