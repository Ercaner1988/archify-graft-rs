//! "Explore this system" tour player: steps, dwell timing, play/pause/next/prev.
//!
//! On every step change `update` yields `TourEvent::Enter(i)`; the caller then
//! flies the camera to that step's nodes over `TOUR_CAMERA_SECS`.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TourStep {
    /// Node indices highlighted at this step, in story order.
    pub nodes: Vec<usize>,
    pub title: String,
    pub desc: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TourEvent {
    Enter(usize),
    Finished,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Idle,
    Playing,
    Paused,
}

#[derive(Debug, Clone)]
pub struct Tour {
    steps: Vec<TourStep>,
    state: State,
    index: usize,
    timer: f32,
    pending: Option<TourEvent>,
}

impl Tour {
    pub fn new(steps: Vec<TourStep>) -> Self {
        Self {
            steps,
            state: State::Idle,
            index: 0,
            timer: 0.0,
            pending: None,
        }
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    pub fn step(&self, i: usize) -> Option<&TourStep> {
        self.steps.get(i)
    }

    /// Seconds each step stays: `max(1.1, 3.2 / n)`.
    pub fn dwell_secs(&self) -> f32 {
        (3.2 / self.steps.len().max(1) as f32).max(1.1)
    }

    /// Start playing from step 0.
    pub fn start(&mut self) {
        if self.steps.is_empty() {
            return;
        }
        self.state = State::Playing;
        self.enter(0);
    }

    /// `P`: start when idle, otherwise pause/resume.
    pub fn toggle_play(&mut self) {
        match self.state {
            State::Idle => self.start(),
            State::Playing => self.state = State::Paused,
            State::Paused => self.state = State::Playing,
        }
    }

    /// `]`: next step; past the last step the tour finishes.
    pub fn next(&mut self) {
        match self.state {
            State::Idle => self.start(),
            _ if self.index + 1 < self.steps.len() => self.enter(self.index + 1),
            _ => self.finish(),
        }
    }

    /// `[`: previous step (no-op at the first).
    pub fn prev(&mut self) {
        if self.state != State::Idle && self.index > 0 {
            self.enter(self.index - 1);
        }
    }

    pub fn goto(&mut self, i: usize) {
        if i >= self.steps.len() {
            return;
        }
        if self.state == State::Idle {
            self.state = State::Paused;
        }
        self.enter(i);
    }

    pub fn stop(&mut self) {
        self.state = State::Idle;
        self.timer = 0.0;
        self.pending = None;
    }

    fn enter(&mut self, i: usize) {
        self.index = i;
        self.timer = 0.0;
        self.pending = Some(TourEvent::Enter(i));
    }

    fn finish(&mut self) {
        self.state = State::Idle;
        self.timer = 0.0;
        self.pending = Some(TourEvent::Finished);
    }

    /// Advance the dwell timer. A queued user-triggered event is returned first.
    pub fn update(&mut self, dt: f32) -> Option<TourEvent> {
        if let Some(e) = self.pending.take() {
            return Some(e);
        }
        if self.state != State::Playing {
            return None;
        }
        self.timer += dt.max(0.0);
        if self.timer < self.dwell_secs() {
            return None;
        }
        if self.index + 1 < self.steps.len() {
            self.enter(self.index + 1);
        } else {
            self.finish();
        }
        self.pending.take()
    }

    /// Progress of the current step's dwell, `0..=1` (progress bar).
    pub fn progress(&self) -> f32 {
        if self.state == State::Idle {
            0.0
        } else {
            (self.timer / self.dwell_secs()).clamp(0.0, 1.0)
        }
    }

    /// Current step index while active (playing or paused).
    pub fn current(&self) -> Option<usize> {
        (self.state != State::Idle).then_some(self.index)
    }

    pub fn is_playing(&self) -> bool {
        self.state == State::Playing
    }

    pub fn is_active(&self) -> bool {
        self.state != State::Idle
    }

    /// The tour needs frames (playing, or an event is queued).
    pub fn is_animating(&self) -> bool {
        self.state == State::Playing || self.pending.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tour(n: usize) -> Tour {
        Tour::new(
            (0..n)
                .map(|i| TourStep {
                    nodes: vec![i],
                    title: format!("s{i}"),
                    desc: String::new(),
                })
                .collect(),
        )
    }

    #[test]
    fn dwell_formula() {
        assert_eq!(tour(1).dwell_secs(), 3.2);
        assert_eq!(tour(2).dwell_secs(), 1.6);
        assert_eq!(tour(5).dwell_secs(), 1.1);
        assert_eq!(tour(0).dwell_secs(), 3.2);
    }

    #[test]
    fn plays_through_and_finishes() {
        let mut t = tour(3);
        assert!(!t.is_active());
        t.start();
        assert_eq!(t.update(0.0), Some(TourEvent::Enter(0)));
        let mut events = Vec::new();
        for _ in 0..600 {
            if let Some(e) = t.update(1.0 / 60.0) {
                events.push(e);
            }
        }
        assert_eq!(
            events,
            vec![
                TourEvent::Enter(1),
                TourEvent::Enter(2),
                TourEvent::Finished
            ]
        );
        assert!(!t.is_active());
        assert_eq!(t.current(), None);
    }

    #[test]
    fn pause_freezes_timer_and_progress_moves() {
        let mut t = tour(4);
        t.start();
        t.update(0.0);
        t.update(0.5);
        let p = t.progress();
        assert!(p > 0.0 && p < 1.0);
        t.toggle_play();
        assert!(!t.is_playing());
        assert_eq!(t.update(5.0), None);
        assert_eq!(t.progress(), p);
        t.toggle_play();
        assert!(t.is_playing());
    }

    #[test]
    fn manual_nav_resets_timer_and_emits_enter() {
        let mut t = tour(4);
        t.start();
        t.update(0.0);
        t.update(0.8);
        t.next();
        assert_eq!(t.update(0.0), Some(TourEvent::Enter(1)));
        assert_eq!(t.progress(), 0.0);
        t.prev();
        assert_eq!(t.update(0.0), Some(TourEvent::Enter(0)));
        t.prev(); // no-op at first step
        assert_eq!(t.update(0.0), None);
        t.goto(3);
        assert_eq!(t.update(0.0), Some(TourEvent::Enter(3)));
        t.next(); // last -> finished
        assert_eq!(t.update(0.0), Some(TourEvent::Finished));
        assert!(!t.is_active());
    }

    #[test]
    fn next_from_idle_starts_and_paused_goto_stays_paused() {
        let mut t = tour(3);
        t.next();
        assert_eq!(t.update(0.0), Some(TourEvent::Enter(0)));
        assert!(t.is_playing());
        t.stop();
        t.goto(2);
        assert!(t.is_active() && !t.is_playing());
        assert_eq!(t.current(), Some(2));
        assert_eq!(t.step(2).map(|s| s.title.as_str()), Some("s2"));
    }
}
