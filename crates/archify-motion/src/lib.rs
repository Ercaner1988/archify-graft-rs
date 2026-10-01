//! archify-motion: pure-Rust, egui-independent time-based motion state machines.
//!
//! All durations are seconds, all lengths are `f32`. Every state machine's
//! `update(dt)` returns `true` while it is still animating; the caller ORs the
//! results and only requests a repaint when at least one returned `true`
//! (idle CPU stays ~0).

pub mod beam;
pub mod ease;
pub mod focus;
pub mod intro;
pub mod settings;
pub mod tour;
pub mod transition;
pub mod view;

pub use beam::{
    ambient_window, beam_intensity, beam_window, edge_dirs, gradient_pieces, point_at,
    polyline_len, slice, BeamCycle, EdgeDir,
};
pub use ease::{css_ease, cubic_bezier, cubic_out, ease_in_out, exp_follow, smoothstep};
pub use focus::{adjacent_focus, FocusAnim, FocusSet, HoverIntent};
pub use intro::IntroTrace;
pub use settings::MotionSettings;
pub use tour::{Tour, TourEvent, TourStep};
pub use transition::{Transition, DIM_FOCUS, DIM_HOVER, HOVER_SECS};
pub use view::{wheel_factor, ViewState, CAMERA_SECS, TOUR_CAMERA_SECS};
