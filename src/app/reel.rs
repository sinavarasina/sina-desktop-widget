//! Reusable pause-and-slide rotating slot state for compact bar telemetry lines.

const REEL_ANIM_STEP: f32 = 16.0 / 360.0;

/// Tracks the visible item index, hold countdown (in seconds), and vertical slide progress.
#[derive(Debug, Clone)]
pub struct ReelState {
    pub index: usize,
    pub hold_secs: u16,
    pub hold_remaining: u16,
    pub animating: bool,
    pub progress: f32,
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
            progress: 0.0,
        }
    }

    /// Advances the 1-second hold countdown; triggers a slide transition when the hold expires.
    pub fn tick_second(&mut self, item_count: usize) {
        if item_count <= 1 {
            self.index = 0;
            self.animating = false;
            self.progress = 0.0;
            self.hold_remaining = self.hold_secs;
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
            self.progress = 0.0;
        }
    }

    /// Steps the 60fps slide animation; wraps to the next item and resets the hold countdown on completion.
    pub fn step_animation(&mut self, item_count: usize) {
        if !self.animating {
            return;
        }
        if item_count <= 1 {
            self.reset();
            return;
        }
        self.progress = (self.progress + REEL_ANIM_STEP).min(1.0);
        if self.progress >= 1.0 - 1e-4 {
            self.index = (self.index + 1) % item_count;
            self.progress = 0.0;
            self.animating = false;
            self.hold_remaining = self.hold_secs;
        }
    }

    pub fn reset(&mut self) {
        self.index = 0;
        self.animating = false;
        self.progress = 0.0;
        self.hold_remaining = self.hold_secs;
    }

    #[must_use]
    pub const fn is_animating(&self) -> bool {
        self.animating
    }
}

#[cfg(test)]
mod tests {
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

        for _ in 0..25 {
            reel.step_animation(2);
        }
        assert!(!reel.is_animating());
        assert_eq!(reel.index, 1);
        assert_eq!(reel.hold_remaining, 3);
    }
}
