//! Small, quick animations: screen and sign-in transitions, detail-pane fades and the
//! toast slide. Durations are short so the front desk never waits on an animation.

use eframe::egui::emath::easing;
use std::time::{Duration, Instant};

/// Screen-to-screen and sign-in/lock transitions.
pub const SCREEN_SECS: f32 = 0.26;
/// Switching the selected request or form.
pub const DETAIL_SECS: f32 = 0.18;
/// Hover and pressed states.
pub const HOVER_SECS: f32 = 0.12;
/// Sliding highlights (sidebar, filter pills).
pub const SLIDE_SECS: f32 = 0.2;
/// Distance the incoming screen rises, in points.
pub const RISE: f32 = 14.0;

#[derive(Clone, Copy, Debug)]
pub struct Anim {
    start: Instant,
    secs: f32,
}

impl Anim {
    pub fn start(secs: f32) -> Self {
        Self { start: Instant::now(), secs }
    }
    pub fn done() -> Self {
        Self { start: Instant::now() - Duration::from_secs(60), secs: 0.0 }
    }
    /// Linear progress 0..=1.
    pub fn linear(&self) -> f32 {
        if self.secs <= 0.0 {
            return 1.0;
        }
        (self.start.elapsed().as_secs_f32() / self.secs).clamp(0.0, 1.0)
    }
    /// Eased progress 0..=1 (fast start, gentle stop).
    pub fn eased(&self) -> f32 {
        easing::cubic_out(self.linear())
    }
}

/// Restarts an animation whenever the tracked value changes.
#[derive(Debug)]
pub struct Track<T: PartialEq + Clone> {
    last: Option<T>,
    anim: Anim,
}

impl<T: PartialEq + Clone> Default for Track<T> {
    fn default() -> Self {
        Self { last: None, anim: Anim::done() }
    }
}

impl<T: PartialEq + Clone> Track<T> {
    /// Feed the current value; returns eased progress (1.0 when settled or motion is off).
    pub fn update(&mut self, value: &T, secs: f32, enabled: bool) -> f32 {
        if self.last.as_ref() != Some(value) {
            self.last = Some(value.clone());
            self.anim = Anim::start(secs);
        }
        if enabled { self.anim.eased() } else { 1.0 }
    }
}

/// Fade that never starts fully transparent, so content doesn't flash blank.
pub fn soft(t: f32) -> f32 {
    0.2 + 0.8 * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn track_restarts_on_change_and_settles() {
        let mut tr: Track<u8> = Track::default();
        let t0 = tr.update(&1, 10.0, true);
        assert!(t0 < 0.1, "new value starts an animation");
        assert_eq!(tr.update(&1, 10.0, false), 1.0, "motion off is always settled");
        let mut quick: Track<u8> = Track::default();
        quick.update(&1, 0.0, true);
        assert_eq!(quick.update(&1, 0.0, true), 1.0);
        assert!(soft(0.0) > 0.0 && soft(1.0) == 1.0);
    }
}
