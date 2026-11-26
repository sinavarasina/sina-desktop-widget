//! Reusable VSync animation controller backed by `waywi::animation::{Animated, Easing}`.

use std::time::{Duration, Instant};

use waywi::prelude::*;

/// Standard popup/panel reveal duration matching the sharp Catppuccin Mocha feel.
pub const POPUP_ANIM_DURATION: Duration = Duration::from_millis(180);

/// Smooth `0.0..=1.0` open/close controller backed by `waywi::animation::Animated<f32>` and
/// driven by monitor-synchronized `Animation::frames` VSync callbacks (zero wakeups once settled).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RevealAnim {
    anim: Animated<f32>,
    duration: Duration,
    easing: Easing,
    open: bool,
}

impl Default for RevealAnim {
    fn default() -> Self {
        Self::popup()
    }
}

impl RevealAnim {
    #[must_use]
    pub const fn new(duration: Duration, easing: Easing) -> Self {
        Self {
            anim: Animated::new(0.0),
            duration,
            easing,
            open: false,
        }
    }

    #[must_use]
    pub const fn popup() -> Self {
        Self::new(POPUP_ANIM_DURATION, Easing::CubicOut)
    }

    pub fn set_open(&mut self, open: bool) {
        if self.open == open {
            return;
        }
        self.open = open;
        let target = if open { 1.0 } else { 0.0 };
        self.anim
            .transition_to(target, self.duration, self.easing, Instant::now());
    }

    pub fn toggle(&mut self) -> bool {
        let next = !self.open;
        self.set_open(next);
        next
    }

    pub fn snap_closed(&mut self) {
        self.open = false;
        self.anim.snap_to(0.0);
    }

    pub fn advance(&mut self, now: Instant) -> f32 {
        self.anim.sample(now)
    }

    #[inline]
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.open
    }

    #[inline]
    #[must_use]
    pub fn is_visible(&self) -> bool {
        self.open || self.anim.value() > 0.001
    }

    #[inline]
    #[must_use]
    pub fn is_animating(&self) -> bool {
        (self.anim.value() - self.anim.target()).abs() > 0.001
    }

    /// Returns the eased `0.0..=1.0` visual progress value.
    #[inline]
    #[must_use]
    pub fn progress(&self) -> f32 {
        self.anim.value().clamp(0.0, 1.0)
    }
}
