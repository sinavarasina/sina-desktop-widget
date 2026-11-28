//! Responsive layout breakpoints derived purely from the primary output's logical viewport
//! `(bar_width, screen_height)` so the bar and popups adapt cleanly across HiDPI scales
//! (1x, 1.25x, 1.5x, 2x, 2.5x), resolutions (720p, 1080p, 1440p, 4K), and aspect ratios
//! (16:9, 21:9 ultrawide, 4:3, portrait).

use crate::theme::{MAX_WINDOW_CHARS, MPRIS_BAR_TEXT_WIDTH, NOTIF_MAX_BAR_ICONS, NOTIF_TICKER_WIDTH};

/// Coarse horizontal density tier based on logical bar width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutTier {
    /// `>= 1500px` logical (1080p@1x, 1440p@1x/1.5x, 4K@2x, ultrawide).
    Full,
    /// `1180..1500px` logical (1080p@1.5x = 1280, 720p@1x = 1280, 1440p@2x = 1280).
    Compact,
    /// `860..1180px` logical (1080p@2x = 960, portrait 1080p = 1080, 1024x768).
    Tight,
    /// `< 860px` logical (1080p@2.5x = 768, portrait 720p = 720, narrow split).
    Minimal,
}

/// Computed bar & popup sizing constraints for the current logical output viewport.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResponsiveLayout {
    pub tier: LayoutTier,
    pub show_sysinfo: bool,
    pub show_window_title: bool,
    pub show_window_category: bool,
    pub max_window_chars: usize,
    pub max_notif_icons: usize,
    pub notif_ticker_width: f32,
    pub show_mpris_text: bool,
    pub mpris_bar_text_width: f32,
    /// Whether the right-bar audio visualizer has enough horizontal headroom to mount.
    pub show_visualizer: bool,
    /// Logical width of the right-bar audio visualizer canvas.
    pub visualizer_width: f32,
    /// Maximum right-side `Bar` metrics kept directly on the bar; overflow moves seamlessly into
    /// the expandable `exp` drawer so left and right bar clusters never collide.
    pub max_bar_metrics: usize,
    /// Vertical height cap for the Quick Menu Settings drawer relative to `screen_height`.
    pub settings_panel_height: f32,
    /// Maximum visible cards in the Clipboard popup relative to `screen_height`.
    pub clipboard_max_cards: usize,
}

impl ResponsiveLayout {
    #[must_use]
    pub fn for_viewport(bar_width: f32, screen_height: f32) -> Self {
        let w = bar_width.max(320.0);
        let h = screen_height.max(240.0);

        let settings_panel_height = (h - 236.0).clamp(150.0, 278.0);
        let clipboard_max_cards = ((h - 104.0) / 46.0).floor().clamp(3.0, 7.0) as usize;

        if w >= 1500.0 {
            Self {
                tier: LayoutTier::Full,
                show_sysinfo: true,
                show_window_title: true,
                show_window_category: true,
                max_window_chars: MAX_WINDOW_CHARS,
                max_notif_icons: NOTIF_MAX_BAR_ICONS,
                notif_ticker_width: NOTIF_TICKER_WIDTH,
                show_mpris_text: true,
                mpris_bar_text_width: MPRIS_BAR_TEXT_WIDTH,
                show_visualizer: true,
                visualizer_width: 108.0,
                max_bar_metrics: usize::MAX,
                settings_panel_height,
                clipboard_max_cards,
            }
        } else if w >= 1180.0 {
            Self {
                tier: LayoutTier::Compact,
                show_sysinfo: true,
                show_window_title: true,
                show_window_category: false,
                max_window_chars: 16,
                max_notif_icons: 3,
                notif_ticker_width: 180.0,
                show_mpris_text: true,
                mpris_bar_text_width: 100.0,
                show_visualizer: true,
                visualizer_width: 84.0,
                max_bar_metrics: usize::MAX,
                settings_panel_height,
                clipboard_max_cards,
            }
        } else if w >= 860.0 {
            Self {
                tier: LayoutTier::Tight,
                show_sysinfo: false,
                show_window_title: w >= 1040.0,
                show_window_category: false,
                max_window_chars: 12,
                max_notif_icons: 2,
                notif_ticker_width: 132.0,
                show_mpris_text: true,
                mpris_bar_text_width: 84.0,
                show_visualizer: false,
                visualizer_width: 72.0,
                max_bar_metrics: if w >= 940.0 { 4 } else { 2 },
                settings_panel_height,
                clipboard_max_cards,
            }
        } else {
            Self {
                tier: LayoutTier::Minimal,
                show_sysinfo: false,
                show_window_title: false,
                show_window_category: false,
                max_window_chars: 10,
                max_notif_icons: 1,
                notif_ticker_width: 104.0,
                show_mpris_text: false,
                mpris_bar_text_width: 72.0,
                show_visualizer: false,
                visualizer_width: 64.0,
                max_bar_metrics: 1,
                settings_panel_height,
                clipboard_max_cards,
            }
        }
    }

    #[must_use]
    pub fn mpris_bar_text_inner_w(&self) -> f32 {
        (self.mpris_bar_text_width - 8.0).max(40.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{LayoutTier, ResponsiveLayout};

    #[test]
    fn responsive_tiers_cover_1080p_scales_portrait_and_ultrawide() {
        let full = ResponsiveLayout::for_viewport(1920.0, 1080.0);
        assert_eq!(full.tier, LayoutTier::Full);
        assert!(full.show_sysinfo && full.show_window_title && full.show_window_category);
        assert_eq!(full.max_notif_icons, 4);
        assert_eq!(full.mpris_bar_text_width, 128.0);
        assert_eq!(full.settings_panel_height, 278.0);

        let scale_1_5 = ResponsiveLayout::for_viewport(1280.0, 720.0);
        assert_eq!(scale_1_5.tier, LayoutTier::Compact);
        assert!(!scale_1_5.show_window_category);
        assert_eq!(scale_1_5.max_window_chars, 16);

        let scale_2_0 = ResponsiveLayout::for_viewport(960.0, 540.0);
        assert_eq!(scale_2_0.tier, LayoutTier::Tight);
        assert!(!scale_2_0.show_sysinfo);
        assert!(!scale_2_0.show_window_title);
        assert_eq!(scale_2_0.max_bar_metrics, 4);
        assert!(scale_2_0.settings_panel_height <= 278.0);
        assert!(scale_2_0.clipboard_max_cards <= 7);

        let narrow = ResponsiveLayout::for_viewport(720.0, 480.0);
        assert_eq!(narrow.tier, LayoutTier::Minimal);
        assert!(!narrow.show_mpris_text);
        assert_eq!(narrow.max_bar_metrics, 1);
    }
}
