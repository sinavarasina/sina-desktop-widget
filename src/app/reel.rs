//! Reusable pause-and-slide rotating slot state backed by `waywi::animation::Animated`.

use std::time::{Duration, Instant};

use waywi::prelude::*;

const REEL_ANIM_DURATION: Duration = Duration::from_millis(360);

/// Tracks the visible item index, hold countdown (in seconds), and VSync-paced slide progress.
#[derive(Debug, Clone)]
pub struct ReelState {
    pub index: usize,
    pub hold_secs: u16,
    pub hold_remaining: u16,
    pub animating: bool,
    anim: Animated<f32>,
}

impl ReelState {
    #[must_use]
    pub const fn new(hold_secs: u16) -> Self {
        let clamped = if hold_secs == 0 { 1 } else { hold_secs };
        Self {
            index: 0,
            hold_secs: clamped,
            hold_remaining: clamped,
            animating: false,
            anim: Animated::new(0.0),
        }
    }

    /// Advances the 1-second hold countdown; triggers a slide transition when the hold expires.
    pub fn tick_second(&mut self, item_count: usize) {
        if item_count <= 1 {
            self.reset();
            return;
        }
        if self.index >= item_count {
            self.index %= item_count;
        }
        if self.animating {
            return;
        }
        if self.hold_remaining > 1 {
            self.hold_remaining -= 1;
        } else {
            self.animating = true;
            self.anim.snap_to(0.0);
            self.anim
                .transition_to(1.0, REEL_ANIM_DURATION, Easing::CubicOut, Instant::now());
        }
    }

    /// Advances the VSync slide animation; wraps to the next item and resets the hold countdown.
    pub fn advance(&mut self, now: Instant, item_count: usize) {
        if !self.animating {
            return;
        }
        if item_count <= 1 {
            self.reset();
            return;
        }
        let val = self.anim.sample(now);
        if val >= 1.0 - 1e-4 {
            self.index = (self.index + 1) % item_count;
            self.anim.snap_to(0.0);
            self.animating = false;
            self.hold_remaining = self.hold_secs;
        }
    }

    pub fn reset(&mut self) {
        self.index = 0;
        self.animating = false;
        self.anim.snap_to(0.0);
        self.hold_remaining = self.hold_secs;
    }

    #[must_use]
    pub const fn is_animating(&self) -> bool {
        self.animating
    }

    /// Eased `0.0..=1.0` slide progress.
    #[must_use]
    pub fn progress(&self) -> f32 {
        self.anim.value().clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::ReelState;

    #[test]
    fn holds_for_configured_seconds_then_slides_and_wraps() {
        let mut reel = ReelState::new(3);
        reel.tick_second(2);
        assert!(!reel.is_animating());
        assert_eq!(reel.index, 0);

        reel.tick_second(2);
        assert!(!reel.is_animating());

        reel.tick_second(2);
        assert!(reel.is_animating());

        let t0 = Instant::now();
        reel.advance(t0 + Duration::from_millis(180), 2);
        assert!(reel.is_animating());
        assert!(reel.progress() > 0.5);

        reel.advance(t0 + Duration::from_millis(400), 2);
        assert!(!reel.is_animating());
        assert_eq!(reel.index, 1);
        assert_eq!(reel.hold_remaining, 3);
    }
}
