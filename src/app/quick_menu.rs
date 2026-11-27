//! Quick status/settings popup state, logind power capability mapping, and lazy-decoded
//! Yuki Nagato (`21_witch_profile_right`) layered sprite assets.

use std::time::{Duration, Instant};

use waywi::prelude::*;
use waywi::dbus::{LogindPowerCapabilities, PowerActionSupport};

use crate::app::anim::RevealAnim;
use crate::theme::{BLUE, MAUVE, RED, SUBTEXT1, TEXT};

// Embed ONLY the 1x PNG files needed for the 3 scrollable witch poses (~185 KB total).
// At ~0.50x display scale, 1x sources avoid >2x single-pass downscale aliasing.
// Only the active pose is decoded at a time; scrolling or closing evicts old pixels to 0 B.

// Pose 0: `21_witch_profile_right` (side profile, facing right)
const WITCH21_BASE: &[u8] = include_bytes!(
    "../../assets/sprites/yuki_nagato_yakusoku_psp_portraits/21_witch_profile_right/base_body.png"
);
const WITCH21_EYE_PRIMARY: &[u8] = include_bytes!(
    "../../assets/sprites/yuki_nagato_yakusoku_psp_portraits/21_witch_profile_right/expressions/expr01_witchhat_profile_looking_forward_eyes_strip.png"
);
const WITCH21_EYE_ALT: &[u8] = include_bytes!(
    "../../assets/sprites/yuki_nagato_yakusoku_psp_portraits/21_witch_profile_right/expressions/expr02_witchhat_profile_side_glance_viewer_eyes_strip.png"
);
const WITCH21_MOUTH: &[u8] = include_bytes!(
    "../../assets/sprites/yuki_nagato_yakusoku_psp_portraits/21_witch_profile_right/expressions/expr01_witchhat_profile_looking_forward_mouth_strip.png"
);

// Pose 1: `20_witch_3q_right` (3/4 angle, turning toward viewer)
const WITCH20_BASE: &[u8] = include_bytes!(
    "../../assets/sprites/yuki_nagato_yakusoku_psp_portraits/20_witch_3q_right/base_body.png"
);
const WITCH20_EYE_PRIMARY: &[u8] = include_bytes!(
    "../../assets/sprites/yuki_nagato_yakusoku_psp_portraits/20_witch_3q_right/expressions/expr01_witchhat_neutral_calm_eyes_strip.png"
);
const WITCH20_EYE_ALT: &[u8] = include_bytes!(
    "../../assets/sprites/yuki_nagato_yakusoku_psp_portraits/20_witch_3q_right/expressions/expr06_witchhat_side_glance_viewer_eyes_strip.png"
);
const WITCH20_MOUTH_PRIMARY: &[u8] = include_bytes!(
    "../../assets/sprites/yuki_nagato_yakusoku_psp_portraits/20_witch_3q_right/expressions/expr01_witchhat_neutral_calm_mouth_strip.png"
);
const WITCH20_MOUTH_ALT: &[u8] = include_bytes!(
    "../../assets/sprites/yuki_nagato_yakusoku_psp_portraits/20_witch_3q_right/expressions/expr06_witchhat_side_glance_viewer_mouth_strip.png"
);

// Pose 2: `19_witch_front_standard` (facing front straight at viewer)
const WITCH19_BASE: &[u8] = include_bytes!(
    "../../assets/sprites/yuki_nagato_yakusoku_psp_portraits/19_witch_front_standard/base_body.png"
);
const WITCH19_EYE_PRIMARY: &[u8] = include_bytes!(
    "../../assets/sprites/yuki_nagato_yakusoku_psp_portraits/19_witch_front_standard/expressions/expr01_witchhat_neutral_calm_eyes_strip.png"
);
const WITCH19_EYE_ALT: &[u8] = include_bytes!(
    "../../assets/sprites/yuki_nagato_yakusoku_psp_portraits/19_witch_front_standard/expressions/expr04_witchhat_gentle_smile_eyes_strip.png"
);
const WITCH19_MOUTH_PRIMARY: &[u8] = include_bytes!(
    "../../assets/sprites/yuki_nagato_yakusoku_psp_portraits/19_witch_front_standard/expressions/expr01_witchhat_neutral_calm_mouth_strip.png"
);
const WITCH19_MOUTH_ALT: &[u8] = include_bytes!(
    "../../assets/sprites/yuki_nagato_yakusoku_psp_portraits/19_witch_front_standard/expressions/expr04_witchhat_gentle_smile_mouth_strip.png"
);

#[derive(Clone, Copy)]
pub struct WitchPoseSpec {
    pub base_w: f32,
    pub base_h: f32,
    pub eye_x: f32,
    pub eye_y: f32,
    pub eye_w: f32,
    pub eye_h: f32,
    pub mouth_x: f32,
    pub mouth_y: f32,
    pub mouth_w: f32,
    pub mouth_h: f32,
    pub base_png: &'static [u8],
    pub eye_primary_png: &'static [u8],
    pub eye_alt_png: &'static [u8],
    pub mouth_primary_png: &'static [u8],
    pub mouth_alt_png: Option<&'static [u8]>,
}

/// Scroll order: side profile (`21`) → 3/4 view (`20`) → front view (`19`).
pub const WITCH_POSES: [WitchPoseSpec; 3] = [
    WitchPoseSpec {
        base_w: 327.0,
        base_h: 336.0,
        eye_x: 184.0,
        eye_y: 127.0,
        eye_w: 34.0,
        eye_h: 42.0,
        mouth_x: 200.0,
        mouth_y: 175.0,
        mouth_w: 10.0,
        mouth_h: 18.0,
        base_png: WITCH21_BASE,
        eye_primary_png: WITCH21_EYE_PRIMARY,
        eye_alt_png: WITCH21_EYE_ALT,
        mouth_primary_png: WITCH21_MOUTH,
        mouth_alt_png: None,
    },
    WitchPoseSpec {
        base_w: 352.0,
        base_h: 346.0,
        eye_x: 143.0,
        eye_y: 145.0,
        eye_w: 74.0,
        eye_h: 42.0,
        mouth_x: 183.0,
        mouth_y: 185.0,
        mouth_w: 10.0,
        mouth_h: 18.0,
        base_png: WITCH20_BASE,
        eye_primary_png: WITCH20_EYE_PRIMARY,
        eye_alt_png: WITCH20_EYE_ALT,
        mouth_primary_png: WITCH20_MOUTH_PRIMARY,
        mouth_alt_png: Some(WITCH20_MOUTH_ALT),
    },
    WitchPoseSpec {
        base_w: 326.0,
        base_h: 349.0,
        eye_x: 125.0,
        eye_y: 148.0,
        eye_w: 82.0,
        eye_h: 42.0,
        mouth_x: 157.0,
        mouth_y: 188.0,
        mouth_w: 18.0,
        mouth_h: 18.0,
        base_png: WITCH19_BASE,
        eye_primary_png: WITCH19_EYE_PRIMARY,
        eye_alt_png: WITCH19_EYE_ALT,
        mouth_primary_png: WITCH19_MOUTH_PRIMARY,
        mouth_alt_png: Some(WITCH19_MOUTH_ALT),
    },
];

/// Eye gaze / expression toggle available for the active Quick Status witch pose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum YukiGaze {
    #[default]
    LookingForward,
    SideGlanceViewer,
}

/// Decoded layered sprite assets cached lazily per witch pose.
#[derive(Clone)]
pub struct YukiSpriteAssets {
    pub base: ImageAsset,
    pub eye_forward: SpriteSheet,
    pub eye_glance: SpriteSheet,
    pub mouth: SpriteSheet,
    pub mouth_alt: Option<SpriteSheet>,
}

impl YukiSpriteAssets {
    /// Lazily decodes the embedded PNGs for `spec` and builds timed blink & mouth `SpriteSheet`s.
    pub fn load(spec: &WitchPoseSpec, ctx: &mut EffectCtx<'_, crate::app::msg::Msg>) -> Option<Self> {
        let base = ctx.load_image_static(spec.base_png).ok()?;
        let eye_fwd_asset = ctx.load_image_static(spec.eye_primary_png).ok()?;
        let eye_glc_asset = ctx.load_image_static(spec.eye_alt_png).ok()?;
        let mouth_asset = ctx.load_image_static(spec.mouth_primary_png).ok()?;

        let make_mouth_sheet = |asset: ImageAsset| {
            SpriteSheet::new(asset)
                .frame_size(spec.mouth_w.logical_px().ceil_u32().max(1), spec.mouth_h.logical_px().ceil_u32().max(1))
                .timed_animation(
                    "react",
                    [
                        (1, Duration::from_millis(90)),
                        (2, Duration::from_millis(150)),
                        (1, Duration::from_millis(90)),
                        (0, Duration::from_millis(60)),
                    ],
                    PlaybackMode::Once,
                )
        };

        let mouth_alt = match spec.mouth_alt_png {
            Some(bytes) => Some(make_mouth_sheet(ctx.load_image_static(bytes).ok()?)),
            None => None,
        };

        let eye_forward = SpriteSheet::new(eye_fwd_asset)
            .frame_size(spec.eye_w.logical_px().ceil_u32().max(1), spec.eye_h.logical_px().ceil_u32().max(1))
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
            .frame_size(spec.eye_w.logical_px().ceil_u32().max(1), spec.eye_h.logical_px().ceil_u32().max(1))
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

        Some(Self {
            base,
            eye_forward,
            eye_glance,
            mouth: make_mouth_sheet(mouth_asset),
            mouth_alt,
        })
    }

    pub fn evict_pixels(&self, ctx: &mut EffectCtx<'_, crate::app::msg::Msg>) {
        let _ = ctx.evict_image_pixels(self.base);
        let _ = ctx.evict_image_pixels(self.eye_forward.asset);
        let _ = ctx.evict_image_pixels(self.eye_glance.asset);
        let _ = ctx.evict_image_pixels(self.mouth.asset);
        if let Some(ref alt) = self.mouth_alt {
            let _ = ctx.evict_image_pixels(alt.asset);
        }
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

/// Action rail slots displayed in a scrollable 2-slot vertical window inside Quick Menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickActionSlot {
    Settings,
    Power,
    Toolbox,
}

impl QuickActionSlot {
    pub const ALL: [Self; 3] = [Self::Settings, Self::Power, Self::Toolbox];
    pub const VISIBLE: usize = 2;
}

pub struct QuickMenuState {
    pub popup_anim: RevealAnim,
    pub power_menu_open: bool,
    pub settings_open: bool,
    pub toolbox_open: bool,
    /// Scroll window offset (`0..=max_action_scroll()`) into [`QuickActionSlot::ALL`].
    pub action_scroll: usize,
    pub pending_power: Option<PendingPowerAction>,
    pub logind: LogindSnapshot,
    pub power_error: Option<String>,
    pub username: String,
    pub gaze: YukiGaze,
    pub mouth_reacting: bool,
    pub sprite_pose_idx: usize,
    pub sprite_assets: Option<YukiSpriteAssets>,
    cached_sprite_poses: [Option<YukiSpriteAssets>; 3],
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
            popup_anim: RevealAnim::popup(),
            power_menu_open: false,
            settings_open: false,
            toolbox_open: false,
            action_scroll: 0,
            pending_power: None,
            logind: LogindSnapshot::default(),
            power_error: None,
            username,
            gaze: YukiGaze::LookingForward,
            mouth_reacting: false,
            sprite_pose_idx: 0,
            sprite_assets: None,
            cached_sprite_poses: [None, None, None],
        }
    }
}

impl QuickMenuState {
    #[must_use]
    pub fn current_pose_spec(&self) -> WitchPoseSpec {
        WITCH_POSES[self.sprite_pose_idx % WITCH_POSES.len()]
    }

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

    fn load_or_cached_pose(
        &mut self,
        idx: usize,
        ctx: &mut EffectCtx<'_, crate::app::msg::Msg>,
    ) -> Option<YukiSpriteAssets> {
        let slot = idx % WITCH_POSES.len();
        if let Some(ref cached) = self.cached_sprite_poses[slot] {
            return Some(cached.clone());
        }
        let loaded = YukiSpriteAssets::load(&WITCH_POSES[slot], ctx)?;
        self.cached_sprite_poses[slot] = Some(loaded.clone());
        Some(loaded)
    }

    pub fn ensure_sprite_loaded(&mut self, ctx: &mut EffectCtx<'_, crate::app::msg::Msg>) {
        if self.sprite_assets.is_none() {
            let idx = self.sprite_pose_idx % WITCH_POSES.len();
            self.sprite_assets = self.load_or_cached_pose(idx, ctx);
        }
    }

    /// Scrolls through the 3 witch poses (`21_profile` ↔ `20_3q` ↔ `19_front`), lazily decoding
    /// the newly active pose and immediately evicting the previous pose's RGBA8 pixels.
    pub fn scroll_sprite_pose(&mut self, delta: i32, ctx: &mut EffectCtx<'_, crate::app::msg::Msg>) {
        if delta == 0 {
            return;
        }
        let len = WITCH_POSES.len();
        let next_idx = if delta > 0 {
            (self.sprite_pose_idx + 1) % len
        } else {
            (self.sprite_pose_idx + len - 1) % len
        };
        if next_idx == self.sprite_pose_idx {
            return;
        }
        if let Some(ref prev) = self.sprite_assets {
            prev.evict_pixels(ctx);
        }
        self.sprite_pose_idx = next_idx;
        self.sprite_assets = self.load_or_cached_pose(next_idx, ctx);
        self.mouth_reacting = false;
    }

    /// Sets the active Quick Menu witch pose by index (`0..=2`), evicting old pixels when loaded.
    pub fn set_sprite_pose(&mut self, idx: usize, ctx: &mut EffectCtx<'_, crate::app::msg::Msg>) {
        let target = idx % WITCH_POSES.len();
        if target == self.sprite_pose_idx {
            return;
        }
        if let Some(ref prev) = self.sprite_assets {
            prev.evict_pixels(ctx);
        }
        self.sprite_pose_idx = target;
        if self.popup_anim.is_open() || self.sprite_assets.is_some() {
            self.sprite_assets = self.load_or_cached_pose(target, ctx);
        }
        self.mouth_reacting = false;
    }

    /// Evicts decoded RGBA8 sprite pixels back to their compressed PNG bytes while Quick Status
    /// is closed; `waywi` re-decodes them transparently on next open.
    pub fn evict_sprite_pixels(&self, ctx: &mut EffectCtx<'_, crate::app::msg::Msg>) {
        if let Some(ref assets) = self.sprite_assets {
            assets.evict_pixels(ctx);
        }
    }

    pub fn open_popup(&mut self, ctx: &mut EffectCtx<'_, crate::app::msg::Msg>) {
        self.popup_anim.set_open(true);
        self.power_error = None;
        self.ensure_sprite_loaded(ctx);
    }

    pub fn close_popup(&mut self) {
        self.popup_anim.set_open(false);
        self.power_menu_open = false;
        self.settings_open = false;
        self.toolbox_open = false;
        self.pending_power = None;
        self.mouth_reacting = false;
    }

    /// Instantly hides Quick Menu without closing animation (prevents fading UI in screencopy).
    pub fn close_popup_immediate(&mut self) {
        self.popup_anim = RevealAnim::popup();
        self.power_menu_open = false;
        self.settings_open = false;
        self.toolbox_open = false;
        self.pending_power = None;
        self.mouth_reacting = false;
    }

    pub fn start_power_countdown_secs(&mut self, action: PowerAction, secs: u8) {
        self.power_error = None;
        self.pending_power = Some(PendingPowerAction {
            action,
            seconds_left: secs.max(1),
        });
    }

    pub fn cancel_power_countdown(&mut self) {
        self.pending_power = None;
    }

    pub fn toggle_power_menu(&mut self) {
        self.power_menu_open = !self.power_menu_open;
        if self.power_menu_open {
            self.settings_open = false;
            self.toolbox_open = false;
        }
        self.power_error = None;
    }

    pub fn toggle_settings_menu(&mut self) {
        self.settings_open = !self.settings_open;
        if self.settings_open {
            self.power_menu_open = false;
            self.toolbox_open = false;
        }
        self.power_error = None;
    }

    pub fn toggle_toolbox_menu(&mut self) {
        self.toolbox_open = !self.toolbox_open;
        if self.toolbox_open {
            self.settings_open = false;
            self.power_menu_open = false;
            self.action_scroll = Self::max_action_scroll();
        }
        self.power_error = None;
    }

    #[must_use]
    pub const fn max_action_scroll() -> usize {
        QuickActionSlot::ALL
            .len()
            .saturating_sub(QuickActionSlot::VISIBLE)
    }

    #[must_use]
    pub const fn has_actions_above(&self) -> bool {
        self.action_scroll > 0
    }

    #[must_use]
    pub const fn has_actions_below(&self) -> bool {
        self.action_scroll + QuickActionSlot::VISIBLE < QuickActionSlot::ALL.len()
    }

    #[must_use]
    pub fn visible_action_slots(&self) -> [QuickActionSlot; QuickActionSlot::VISIBLE] {
        let start = self.action_scroll.min(Self::max_action_scroll());
        [QuickActionSlot::ALL[start], QuickActionSlot::ALL[start + 1]]
    }

    /// Scrolls the 2-slot left action rail by `delta` (`>0` reveals items below, `<0` reveals items above).
    pub fn scroll_actions(&mut self, delta: i32) -> bool {
        if delta == 0 {
            return false;
        }
        let max_scroll = Self::max_action_scroll();
        let next = if delta > 0 {
            (self.action_scroll + 1).min(max_scroll)
        } else {
            self.action_scroll.saturating_sub(1)
        };
        if next != self.action_scroll {
            self.action_scroll = next;
            true
        } else {
            false
        }
    }

    /// Cycles the action rail on click of the scroll indicator bar.
    pub fn cycle_action_scroll(&mut self) -> bool {
        let max_scroll = Self::max_action_scroll();
        if max_scroll == 0 {
            return false;
        }
        self.action_scroll = if self.action_scroll < max_scroll {
            self.action_scroll + 1
        } else {
            0
        };
        true
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
