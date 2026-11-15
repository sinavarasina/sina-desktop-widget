//! Incoming notification right-to-left marquee ticker state.

use waywi::prelude::*;

use crate::theme::NOTIF_TICKER_WIDTH;

const TICKER_SPEED_PX: f32 = 2.6;

#[derive(Debug, Clone)]
pub struct IncomingTicker {
    pub id: u32,
    pub text: String,
    pub icon: Option<ImageAsset>,
    pub offset_x: f32,
    pub min_offset_x: f32,
}

impl IncomingTicker {
    #[must_use]
    pub fn from_notification(summary: &str, body: &str, icon: Option<ImageAsset>, id: u32) -> Self {
        let clean_summary = summary.trim().replace('\n', " ");
        let clean_body = body.trim().replace('\n', " ");
        let text = if clean_body.is_empty() {
            clean_summary
        } else if clean_summary.is_empty() {
            clean_body
        } else {
            format!("{clean_summary} - {clean_body}")
        };
        let approx_px = (text.chars().count() as f32) * 7.2 + 24.0;
        Self {
            id,
            text,
            icon,
            offset_x: NOTIF_TICKER_WIDTH,
            min_offset_x: -approx_px,
        }
    }

    /// Advances the right-to-left scroll by one frame; returns `false` when finished.
    pub fn step(&mut self) -> bool {
        self.offset_x -= TICKER_SPEED_PX;
        self.offset_x > self.min_offset_x
    }
}
