//! Quick status/settings popup state, logind power capability mapping, and lazy-decoded
//! Yuki Nagato (`21_witch_profile_right`) layered sprite assets.

use std::time::Duration;

use waywi::prelude::*;

use crate::theme::{BLUE, MAUVE, RED, SUBTEXT1, TEXT};

const QUICK_MENU_ANIM_STEP: f32 = 16.0 / 180.0;

// Embed ONLY the 4 deduplicated 1x PNG files needed for `21_witch_profile_right` (~51 KB total).
// At 0.50x display scale (163.5x168 px), 1x sources avoid >2x single-pass downscale aliasing.
// No CSV or JSON is loaded or parsed at runtime.
const WITCH_BASE_PNG: &[u8] = include_bytes!(
    "../../assets/sprites/yuki_nagato_yakusoku_psp_portraits/21_witch_profile_right/base_body.png"
);
const WITCH_EYE_FORWARD_STRIP_PNG: &[u8] = include_bytes!(
    "../../assets/sprites/yuki_nagato_yakusoku_psp_portraits/21_witch_profile_right/expressions/expr01_witchhat_profile_looking_forward_eyes_strip.png"
);
const WITCH_EYE_GLANCE_STRIP_PNG: &[u8] = include_bytes!(
    "../../assets/sprites/yuki_nagato_yakusoku_psp_portraits/21_witch_profile_right/expressions/expr02_witchhat_profile_side_glance_viewer_eyes_strip.png"
);
// `expr01` and `expr02` mouth strips are byte-identical (10x54, 3 frames of 10x18).
const WITCH_MOUTH_STRIP_PNG: &[u8] = include_bytes!(
    "../../assets/sprites/yuki_nagato_yakusoku_psp_portraits/21_witch_profile_right/expressions/expr01_witchhat_profile_looking_forward_mouth_strip.png"
);

// Exact reference pixel coordinates for Pose 21 (`21_witch_profile_right`).
pub const SPRITE_BASE_W: f32 = 327.0;
pub const SPRITE_BASE_H: f32 = 336.0;
pub const SPRITE_EYE_X: f32 = 184.0;
pub const SPRITE_EYE_Y: f32 = 127.0;
pub const SPRITE_EYE_W: f32 = 34.0;
pub const SPRITE_EYE_H: f32 = 42.0;
pub const SPRITE_MOUTH_X: f32 = 200.0;
pub const SPRITE_MOUTH_Y: f32 = 175.0;
pub const SPRITE_MOUTH_W: f32 = 10.0;
pub const SPRITE_MOUTH_H: f32 = 18.0;

/// Eye gaze variants available for `21_witch_profile_right`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum YukiGaze {
    #[default]
    LookingForward,
    SideGlanceViewer,
}

/// Decoded layered sprite assets cached lazily on first Quick Menu open.
#[derive(Clone)]
pub struct YukiSpriteAssets {
    pub base: ImageAsset,
    pub eye_forward: SpriteSheet,
    pub eye_glance: SpriteSheet,
    pub mouth: SpriteSheet,
}

impl YukiSpriteAssets {
    /// Lazily decodes the 4 embedded PNGs and builds timed blink & mouth-reaction `SpriteSheet`s.
    pub fn load(ctx: &mut EffectCtx<'_>) -> Option<Self> {
        let base = ctx.load_image_bytes(WITCH_BASE_PNG).ok()?;
        let eye_fwd_asset = ctx.load_image_bytes(WITCH_EYE_FORWARD_STRIP_PNG).ok()?;
        let eye_glc_asset = ctx.load_image_bytes(WITCH_EYE_GLANCE_STRIP_PNG).ok()?;
        let mouth_asset = ctx.load_image_bytes(WITCH_MOUTH_STRIP_PNG).ok()?;

        let eye_forward = SpriteSheet::new(eye_fwd_asset)
            .frame_size(SPRITE_EYE_W.logical_px(), SPRITE_EYE_H.logical_px())
            .timed_animation(
                "blink",
                [
                    (0, Duration::from_millis(3600)),
                    (1, Duration::from_millis(70)),
                    (2, Duration::from_millis(110)),
                    (1, Duration::from_millis(70)),
                ],
                PlaybackMode::Loop,
            );

        let eye_glance = SpriteSheet::new(eye_glc_asset)
            .frame_size(SPRITE_EYE_W.logical_px(), SPRITE_EYE_H.logical_px())
            .timed_animation(
                "blink",
                [
                    (0, Duration::from_millis(2900)),
                    (1, Duration::from_millis(70)),
                    (2, Duration::from_millis(110)),
                    (1, Duration::from_millis(70)),
                ],
                PlaybackMode::Loop,
            );

        let mouth = SpriteSheet::new(mouth_asset)
            .frame_size(SPRITE_MOUTH_W.logical_px(), SPRITE_MOUTH_H.logical_px())
            .timed_animation(
                "react",
                [
                    (1, Duration::from_millis(90)),
                    (2, Duration::from_millis(150)),
                    (1, Duration::from_millis(90)),
                    (0, Duration::from_millis(60)),
                ],
                PlaybackMode::Once,
            );

        Some(Self {
            base,
            eye_forward,
            eye_glance,
            mouth,
        })
    }
}

/// System power actions driven by `waywi`'s `org.freedesktop.login1` capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PowerAction {
    PowerOff,
    Reboot,
    RebootToFirmware,
    SoftReboot,
    Suspend,
    Hibernate,
    HybridSleep,
    SuspendThenHibernate,
    Halt,
    Logout,
}

impl PowerAction {
    pub const ALL: [Self; 10] = [
        Self::PowerOff,
        Self::Reboot,
        Self::RebootToFirmware,
        Self::SoftReboot,
        Self::Suspend,
        Self::Hibernate,
        Self::HybridSleep,
        Self::SuspendThenHibernate,
        Self::Halt,
        Self::Logout,
    ];

    #[must_use]
    pub const fn icon(self) -> &'static str {
        match self {
            Self::PowerOff => "\u{f011}",
            Self::Reboot => "\u{f021}",
            Self::RebootToFirmware => "\u{f2db}",
            Self::SoftReboot => "\u{f0e2}",
            Self::Suspend => "\u{f186}",
            Self::Hibernate => "\u{f2dc}",
            Self::HybridSleep => "\u{f0c2}",
            Self::SuspendThenHibernate => "\u{f252}",
            Self::Halt => "\u{f28d}",
            Self::Logout => "\u{f2f5}",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::PowerOff => "Power Off",
            Self::Reboot => "Reboot",
            Self::RebootToFirmware => "UEFI Setup",
            Self::SoftReboot => "Soft Reboot",
            Self::Suspend => "Suspend",
            Self::Hibernate => "Hibernate",
            Self::HybridSleep => "Hybrid Sleep",
            Self::SuspendThenHibernate => "Suspend->Hibernate",
            Self::Halt => "Halt",
            Self::Logout => "Logout",
        }
    }

    #[must_use]
    pub const fn accent(self) -> Color {
        match self {
            Self::PowerOff | Self::Halt => RED,
            Self::Reboot | Self::SoftReboot => MAUVE,
            Self::RebootToFirmware | Self::Suspend | Self::Hibernate => BLUE,
            Self::HybridSleep | Self::SuspendThenHibernate => SUBTEXT1,
            Self::Logout => TEXT,
        }
    }

    #[must_use]
    pub const fn support(self, caps: &LogindPowerCapabilities) -> PowerActionSupport {
        match self {
            Self::PowerOff => caps.power_off,
            Self::Reboot => caps.reboot,
            Self::RebootToFirmware => caps.reboot_to_firmware_setup,
            Self::SoftReboot => caps.soft_reboot,
            Self::Suspend => caps.suspend,
            Self::Hibernate => caps.hibernate,
            Self::HybridSleep => caps.hybrid_sleep,
            Self::SuspendThenHibernate => caps.suspend_then_hibernate,
            Self::Halt => caps.halt,
            // `Session.Terminate` is always available for the active user session.
            Self::Logout => PowerActionSupport::Yes,
        }
    }

    /// Returns all actions supported by the current host according to `LogindPowerCapabilities`.
    #[must_use]
    pub fn supported_actions(caps: &LogindPowerCapabilities) -> Vec<(Self, PowerActionSupport)> {
        Self::ALL
            .into_iter()
            .filter_map(|action| {
                let support = action.support(caps);
                support.is_supported().then_some((action, support))
            })
            .collect()
    }
}

/// Scheduled power action waiting on a 3-second cancelable countdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingPowerAction {
    pub action: PowerAction,
    pub seconds_left: u8,
}

pub const POWER_COUNTDOWN_SECS: u8 = 3;

pub struct QuickMenuState {
    pub popup_target: bool,
    pub popup_progress: f32,
    pub power_menu_open: bool,
    pub pending_power: Option<PendingPowerAction>,
    pub logind: LogindSnapshot,
    pub power_error: Option<String>,
    pub username: String,
    pub gaze: YukiGaze,
    pub mouth_reacting: bool,
    pub sprite_assets: Option<YukiSpriteAssets>,
}

impl Default for QuickMenuState {
    fn default() -> Self {
        let username = std::env::var("USER")
            .or_else(|_| std::env::var("LOGNAME"))
            .ok()
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| String::from("user"));

        Self {
            popup_target: false,
            popup_progress: 0.0,
            power_menu_open: false,
            pending_power: None,
            logind: LogindSnapshot::default(),
            power_error: None,
            username,
            gaze: YukiGaze::LookingForward,
            mouth_reacting: false,
            sprite_assets: None,
        }
    }
}

impl QuickMenuState {
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
        if (self.popup_progress - target).abs() <= QUICK_MENU_ANIM_STEP {
            self.popup_progress = target;
        } else if self.popup_progress < target {
            self.popup_progress = (self.popup_progress + QUICK_MENU_ANIM_STEP).min(1.0);
        } else {
            self.popup_progress = (self.popup_progress - QUICK_MENU_ANIM_STEP).max(0.0);
        }
    }

    pub fn ensure_sprite_loaded(&mut self, ctx: &mut EffectCtx<'_>) {
        if self.sprite_assets.is_none() {
            self.sprite_assets = YukiSpriteAssets::load(ctx);
        }
    }

    /// Evicts decoded RGBA8 sprite pixels (~470 KB) back to their ~52 KB compressed PNG bytes
    /// while the Quick Status popup is closed; `waywi` re-decodes them transparently on next open.
    pub fn evict_sprite_pixels(&self, ctx: &mut EffectCtx<'_>) {
        if let Some(ref assets) = self.sprite_assets {
            let _ = ctx.evict_asset_pixels(assets.base);
            let _ = ctx.evict_asset_pixels(assets.eye_forward.asset);
            let _ = ctx.evict_asset_pixels(assets.eye_glance.asset);
            let _ = ctx.evict_asset_pixels(assets.mouth.asset);
        }
    }

    pub fn open_popup(&mut self, ctx: &mut EffectCtx<'_>) {
        self.popup_target = true;
        self.power_error = None;
        self.ensure_sprite_loaded(ctx);
    }

    pub fn close_popup(&mut self) {
        self.popup_target = false;
        self.power_menu_open = false;
        self.pending_power = None;
        self.mouth_reacting = false;
    }

    pub fn start_power_countdown(&mut self, action: PowerAction) {
        self.power_error = None;
        self.pending_power = Some(PendingPowerAction {
            action,
            seconds_left: POWER_COUNTDOWN_SECS,
        });
    }

    pub fn cancel_power_countdown(&mut self) {
        self.pending_power = None;
    }

    pub fn toggle_power_menu(&mut self) {
        self.power_menu_open = !self.power_menu_open;
        self.power_error = None;
    }

    pub fn interact_sprite(&mut self) {
        self.gaze = match self.gaze {
            YukiGaze::LookingForward => YukiGaze::SideGlanceViewer,
            YukiGaze::SideGlanceViewer => YukiGaze::LookingForward,
        };
        self.mouth_reacting = true;
    }

    #[must_use]
    pub fn supported_power_actions(&self) -> Vec<(PowerAction, PowerActionSupport)> {
        PowerAction::supported_actions(&self.logind.power_capabilities)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_power_actions_filters_by_real_logind_capabilities() {
        let caps = LogindPowerCapabilities {
            power_off: PowerActionSupport::Yes,
            reboot: PowerActionSupport::Challenge,
            reboot_to_firmware_setup: PowerActionSupport::No,
            suspend: PowerActionSupport::Yes,
            hibernate: PowerActionSupport::NotAvailable,
            hybrid_sleep: PowerActionSupport::NotAvailable,
            suspend_then_hibernate: PowerActionSupport::No,
            soft_reboot: PowerActionSupport::Yes,
            halt: PowerActionSupport::NotAvailable,
        };

        let actions = PowerAction::supported_actions(&caps);
        let kinds: Vec<PowerAction> = actions.iter().map(|(a, _)| *a).collect();
        assert_eq!(
            kinds,
            vec![
                PowerAction::PowerOff,
                PowerAction::Reboot,
                PowerAction::SoftReboot,
                PowerAction::Suspend,
                PowerAction::Logout,
            ]
        );
        assert!(actions[1].1.needs_auth());
        assert!(!actions[0].1.needs_auth());
    }
}
