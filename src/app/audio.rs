//! Mixer popup state (open/animation, per-row scrolling, per-app icon cache), plus helpers over
//! the waywi audio snapshot model.

use std::collections::HashMap;
use std::time::Instant;

use waywi::audio::compute_balance_with_map;
use waywi::prelude::*;

use super::anim::RevealAnim;

#[derive(Default)]
pub struct AudioUiState {
    pub popup_anim: RevealAnim,
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
    pub const fn is_popup_open(&self) -> bool {
        self.popup_anim.is_open()
    }

    #[must_use]
    pub fn popup_progress(&self) -> f32 {
        self.popup_anim.progress()
    }

    #[must_use]
    pub fn is_popup_visible(&self) -> bool {
        self.popup_anim.is_visible()
    }

    #[must_use]
    pub fn is_animating_popup(&self) -> bool {
        self.popup_anim.is_animating()
    }

    pub fn advance_popup_animation(&mut self, now: Instant) {
        self.popup_anim.advance(now);
    }

    pub fn open_popup(&mut self) {
        self.popup_anim.set_open(true);
    }

    pub fn toggle_popup(&mut self) {
        if !self.popup_anim.toggle() {
            self.reset_panel_state();
        }
    }

    pub fn close_popup(&mut self) {
        self.popup_anim.set_open(false);
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

/// App-side helpers over [`AudioSnapshot`] that preserve the flat default-device access the UI
/// was built against before waywi unified sinks/sources into device lists.
pub(crate) trait AudioSnapshotExt {
    /// The default sink (`is_source: false`) or source device.
    fn target_device(&self, is_source: bool) -> Option<&AudioDevice>;
    /// Volume of the default sink (input `is_source`) or `None` when absent.
    fn target_volume(&self, is_source: bool) -> Option<&AudioVolume>;
    /// The default device for the direction, mutable.
    fn default_device_mut(&mut self, is_source: bool) -> Option<&mut AudioDevice>;
    /// L/R balance of the target direction's default device (`0.0` centered or mono).
    fn target_balance(&self, is_source: bool) -> f32;
    /// The sound card backing the default device, when it can be identified.
    fn target_card(&self, is_source: bool) -> Option<&AudioCard>;
}

impl AudioSnapshotExt for AudioSnapshot {
    fn target_device(&self, is_source: bool) -> Option<&AudioDevice> {
        if is_source {
            self.default_source()
        } else {
            self.default_sink()
        }
    }

    fn target_volume(&self, is_source: bool) -> Option<&AudioVolume> {
        self.target_device(is_source).map(|d| &d.volume)
    }

    fn default_device_mut(&mut self, is_source: bool) -> Option<&mut AudioDevice> {
        let default_id = if is_source {
            self.default_source_id
        } else {
            self.default_sink_id
        };
        let devices = if is_source {
            &mut self.sources
        } else {
            &mut self.sinks
        };
        if let Some(id) = default_id
            && let Some(dev) = devices.iter_mut().find(|d| d.id == id)
        {
            return Some(dev);
        }
        devices.iter_mut().find(|d| d.is_default)
    }

    fn target_balance(&self, is_source: bool) -> f32 {
        match self.target_volume(is_source) {
            Some(v) => compute_balance_with_map(&v.channels, &v.channel_map),
            None => 0.0,
        }
    }

    fn target_card(&self, is_source: bool) -> Option<&AudioCard> {
        let device = if is_source {
            self.default_source()?
        } else {
            self.default_sink()?
        };
        if self.cards.is_empty() {
            return None;
        }
        // backends no longer link devices to their card; match the shared ALSA/Bluez UID instead
        // (`alsa_card.pci-…` ↔ `alsa_output.pci-….analog-stereo`), falling back to a single card.
        let uid = card_uid(&device.name, &["alsa_output.", "alsa_input.", "bluez_output."])?;
        self.cards
            .iter()
            .find(|c| card_uid(&c.name, &["alsa_card.", "bluez_card."]).is_some_and(|u| u == uid))
            .or_else(|| (self.cards.len() == 1).then(|| &self.cards[0]))
    }
}

/// Strips the well-known direction prefix of a PulseAudio-style name, if present.
fn card_uid<'a>(name: &'a str, prefixes: &[&str]) -> Option<&'a str> {
    prefixes
        .iter()
        .find_map(|p| name.strip_prefix(p))
        .filter(|uid| !uid.is_empty())
}
