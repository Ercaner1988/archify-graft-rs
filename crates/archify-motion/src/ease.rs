//! Easing curves on `t` in `0..=1` (inputs are clamped).

/// Cubic ease-out: `1 - (1-t)^3` (camera tweens).
pub fn cubic_out(t: f32) -> f32 {
    let u = 1.0 - t.clamp(0.0, 1.0);
    1.0 - u * u * u
}

/// Hermite smoothstep.
pub fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Symmetric ease-in-out (cubic).
pub fn ease_in_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

/// Generic CSS `cubic-bezier(x1, y1, x2, y2)` solver (Newton, bisection fallback).
pub fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let bez = |a: f32, b: f32, s: f32| {
        let u = 1.0 - s;
        3.0 * u * u * s * a + 3.0 * u * s * s * b + s * s * s
    };
    let dbez = |a: f32, b: f32, s: f32| {
        let u = 1.0 - s;
        3.0 * u * u * a + 6.0 * u * s * (b - a) + 3.0 * s * s * (1.0 - b)
    };
    let mut s = t;
    for _ in 0..8 {
        let err = bez(x1, x2, s) - t;
        if err.abs() < 1e-5 {
            return bez(y1, y2, s);
        }
        let d = dbez(x1, x2, s);
        if d.abs() < 1e-6 {
            break;
        }
        s = (s - err / d).clamp(0.0, 1.0);
    }
    let (mut lo, mut hi) = (0.0_f32, 1.0_f32);
    s = t;
    for _ in 0..32 {
        let x = bez(x1, x2, s);
        if (x - t).abs() < 1e-5 {
            break;
        }
        if x < t {
            lo = s;
        } else {
            hi = s;
        }
        s = (lo + hi) / 2.0;
    }
    bez(y1, y2, s)
}

/// CSS `ease` = `cubic-bezier(.25, .1, .25, 1)` (hover transitions).
pub fn css_ease(t: f32) -> f32 {
    cubic_bezier(0.25, 0.1, 0.25, 1.0, t)
}

/// Frame-rate independent exponential follow factor: `1 - exp(-rate * dt)`.
/// Apply as `v += (target - v) * exp_follow(dt, rate)`.
pub fn exp_follow(dt: f32, rate: f32) -> f32 {
    1.0 - (-rate * dt.max(0.0)).exp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_and_monotonic() {
        for f in [cubic_out, css_ease, ease_in_out, smoothstep] {
            assert!(f(0.0).abs() < 1e-4 && (f(1.0) - 1.0).abs() < 1e-4);
            let mut prev = 0.0;
            for i in 0..=100 {
                let v = f(i as f32 / 100.0);
                assert!(v + 1e-4 >= prev);
                prev = v;
            }
        }
    }

    #[test]
    fn css_ease_reference_points() {
        // cubic-bezier(.25,.1,.25,1) at x=0.5 is ~0.802
        assert!((css_ease(0.5) - 0.802).abs() < 0.01);
    }

    #[test]
    fn exp_follow_composes() {
        let one = 1.0 - exp_follow(0.1, 5.0);
        let two = (1.0 - exp_follow(0.05, 5.0)).powi(2);
        assert!((one - two).abs() < 1e-5);
    }
}
