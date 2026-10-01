//! User-facing motion switches.

/// `reduced_motion`: snap every transition, no beams/intro (prefers-reduced-motion).
/// `ambient`: thin always-on flow on every edge (costs continuous repaint; default off).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MotionSettings {
    pub reduced_motion: bool,
    pub ambient: bool,
}
