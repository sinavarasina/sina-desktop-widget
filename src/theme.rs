//! Catppuccin Mocha palette, and typography.

use waywi::prelude::*;

pub const BAR_HEIGHT: f32 = 24.0;
pub const SEP_WIDTH: f32 = 11.0;

/// Distance from the top of the monitor (`exclusive_zone` ignored on floating popups).
pub const POPUP_TOP_MARGIN: f32 = 26.0;

pub const LOGO_MIN_WIDTH: f32 = 22.0;
pub const LOGO_PAD_LEFT: f32 = 12.0;
pub const LOGO_PAD_RIGHT: f32 = 4.0;

pub const TRAY_ICON_SIZE: f32 = 16.0;
pub const TRAY_SPACING: f32 = 8.0;
pub const TRAY_PAD_H: f32 = 8.0;

pub const MODULE_PAD_H: f32 = 6.0;
pub const CLOCK_PAD_H: f32 = 12.0;
pub const WINDOW_PAD_H: f32 = 8.0;
pub const MAX_WINDOW_CHARS: usize = 24;

pub const METRIC_ICON_WIDTH: f32 = 11.0;
pub const METRIC_ICON_GAP: f32 = 2.0;

pub const EXPAND_SIGN_WIDTH: f32 = 28.0;
pub const GPU_EXPANDED_WIDTH: f32 = 56.0;
pub const TEMP_EXPANDED_WIDTH: f32 = 52.0;
pub const DISK_EXPANDED_WIDTH: f32 = 72.0;
pub const BT_EXPANDED_WIDTH: f32 = 56.0;
pub const DRAWER_REVEALED_WIDTH: f32 =
    GPU_EXPANDED_WIDTH + TEMP_EXPANDED_WIDTH + DISK_EXPANDED_WIDTH + BT_EXPANDED_WIDTH;

pub const NOTIF_ICON_SIZE: f32 = 16.0;
pub const NOTIF_CENTER_ICON_SIZE: f32 = 44.0;
pub const NOTIF_MAX_BAR_ICONS: usize = 4;
pub const NOTIF_TICKER_WIDTH: f32 = 240.0;
pub const NOTIF_CENTER_WIDTH: f32 = 380.0;
pub const NOTIF_CENTER_TOP_MARGIN: f32 = 34.0;
pub const NOTIF_CARD_TEXT_WIDTH: f32 = 288.0;

pub const MPRIS_ART_SIZE: f32 = 20.0;
pub const MPRIS_BAR_TEXT_WIDTH: f32 = 128.0;
pub const MPRIS_POPUP_WIDTH: f32 = 310.0;
pub const MPRIS_SEEK_TRACK_WIDTH: f32 = 216.0;

pub const MIXER_POPUP_WIDTH: f32 = 310.0;
/// Volume track width shared by the master bar and every per-app row.
pub const MIXER_TRACK_WIDTH: f32 = 244.0;

pub const DRUN_INPUT_MIN_W: f32 = 132.0;

pub const GENTOO_MAUVE_SVG: &[u8] = include_bytes!("../assets/logo/gentoo-mauve.svg");

// Catppuccin Mocha palette from style.css
pub const MAUVE: Color = Color::rgb(203, 166, 247);
pub const RED: Color = Color::rgb(243, 139, 168);
pub const GREEN: Color = Color::rgb(166, 227, 161);
pub const BLUE: Color = Color::rgb(137, 180, 250);
pub const TEXT: Color = Color::rgb(205, 214, 244);
pub const SUBTEXT1: Color = Color::rgb(186, 194, 222);
pub const SUBTEXT0: Color = Color::rgb(166, 173, 200);
pub const OVERLAY0: Color = Color::rgb(108, 112, 134);
pub const SURFACE1: Color = Color::rgb(69, 71, 90);
pub const SURFACE0: Color = Color::rgb(49, 50, 68);
pub const MANTLE: Color = Color::rgb(24, 24, 37);
pub const CRUST: Color = Color::rgb(17, 17, 27);

#[must_use]
pub fn font_ui() -> Font {
    Font::family("MesloLGS Nerd Font")
}

#[must_use]
pub fn font_ui_semibold() -> Font {
    font_ui().weight(FontWeight::SemiBold)
}

#[must_use]
pub fn font_ui_bold() -> Font {
    font_ui().weight(FontWeight::Bold)
}

#[must_use]
pub fn font_window_scroll() -> Font {
    Font::family("BlexMono Nerd Font")
}
