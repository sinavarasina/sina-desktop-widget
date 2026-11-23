//! Mixer popup state (open/animation, per-row scrolling, per-app icon cache).

use std::collections::HashMap;

use waywi::prelude::*;

const POPUP_ANIM_STEP: f32 = 16.0 / 180.0;

#[derive(Default)]
pub struct AudioUiState {
    pub popup_target: bool,
    pub popup_progress: f32,
    pub popup_left_x: f32,
    /// Expandable Routing & Balance card visibility (device/port/profile + L/R balance).
    pub routing_open: bool,
    /// Inline card profile dropdown inside the Routing card.
    pub profiles_open: bool,
    /// Vertical scroll offset inside the Routing card viewport.
    pub routing_scroll_y: f32,
    /// First visible per-app row (scroll window over `streams`/`captures`).
    pub scroll_index: usize,
    pub marquee_tick: u32,
    /// Resolved app icons keyed by stream id; `None` means glyph fallback.
    pub stream_icons: HashMap<u32, Option<ImageAsset>>,
    /// Resolved app icons keyed by capture id; `None` means glyph fallback.
    pub capture_icons: HashMap<u32, Option<ImageAsset>>,
}

impl AudioUiState {
    #[must_use]
    pub fn is_popup_visible(&self) -> bool {
        self.popup_target || self.popup_progress > 0.001
    }

    #[must_use]
    pub fn is_animating_popup(&self) -> bool {
        let target = if self.popup_target { 1.0 } else { 0.0 };
        (self.popup_progress - target).abs() > 0.001
    }

    pub fn step_popup_animation(&mut self) {
        let target = if self.popup_target { 1.0_f32 } else { 0.0_f32 };
        if (self.popup_progress - target).abs() <= POPUP_ANIM_STEP {
            self.popup_progress = target;
        } else if self.popup_progress < target {
            self.popup_progress = (self.popup_progress + POPUP_ANIM_STEP).min(1.0);
        } else {
            self.popup_progress = (self.popup_progress - POPUP_ANIM_STEP).max(0.0);
        }
    }

    pub fn toggle_popup(&mut self) {
        self.popup_target = !self.popup_target;
        if !self.popup_target {
            self.reset_panel_state();
        }
    }

    pub fn close_popup(&mut self) {
        self.popup_target = false;
        self.reset_panel_state();
    }

    fn reset_panel_state(&mut self) {
        self.scroll_index = 0;
        self.routing_open = false;
        self.profiles_open = false;
        self.routing_scroll_y = 0.0;
    }

    pub fn advance_marquee(&mut self) {
        self.marquee_tick = self.marquee_tick.wrapping_add(1);
    }
}
