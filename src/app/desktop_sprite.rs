//! Desktop-layer rotating Yuki Nagato (`yuki_nagato_gekitou_wii`) sprite state, slow
//! fade-out/fade-in transitions, Hyprland workspace-change trigger, natural eye blinking,
//! and per-variant RGBA8 pixel eviction.

use std::time::Duration;

use waywi::prelude::*;

/// How long each sprite pose/expression holds before gently rotating (3 minutes).
pub const DESKTOP_SPRITE_HOLD_SECS: u16 = 180;
/// Opacity delta per 16ms tick (~640ms slow fade-out / ~640ms slow fade-in).
const FADE_STEP: f32 = 16.0 / 640.0;

// --- Embedded 2x seamlessly upscaled (`waifu2x-ncnn-vulkan` cunet n=1 TTA) lossless WebP slices ---
// Total embedded size: ~318 KB (40 slices including speaking M1/M2 mouths for notification reaction).
// Zero `CW03` compact duplicates, zero CSV/JSON parsing at runtime.

const SP00_A: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/CHARA_03_SP00_A.webp");
const SP00_D: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/CHARA_03_SP00_D.webp");
const SP01_A: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/CHARA_03_SP01_A.webp");
const SP01_D: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/CHARA_03_SP01_D.webp");
const SP02_A: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/CHARA_03_SP02_A.webp");
const SP02_D: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/CHARA_03_SP02_D.webp");

const SP00_NORM_E1: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/C03FaceEyeSP00_00_Normal01.webp");
const SP00_NORM_E2: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/C03FaceEyeSP00_00_Normal02.webp");
const SP00_NORM_E3: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/C03FaceEyeSP00_00_Normal03.webp");
const SP00_NORM_M1: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/C03FaceMouthSP00_00_Normal01.webp");
const SP00_NORM_M2: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/C03FaceMouthSP00_00_Normal02.webp");
const SP00_NORM_M3: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/C03FaceMouthSP00_00_Normal03.webp");

const SP00_GLASS_E1: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/C03FaceEyeSP00_01_glass01.webp");
const SP00_GLASS_E2: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/C03FaceEyeSP00_01_glass02.webp");
const SP00_GLASS_E3: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/C03FaceEyeSP00_01_glass03.webp");
const SP00_GLASS_M1: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/C03FaceMouthSP00_01_glass01.webp");
const SP00_GLASS_M2: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/C03FaceMouthSP00_01_glass02.webp");
const SP00_GLASS_M3: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/C03FaceMouthSP00_01_glass03.webp");

const SP00_GLOOM_E1: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/C03FaceEyeSP00_02_gloom01.webp");
const SP00_GLOOM_E2: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/C03FaceEyeSP00_02_gloom02.webp");
const SP00_GLOOM_E3: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/C03FaceEyeSP00_02_gloom03.webp");
const SP00_GLOOM_M1: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/C03FaceMouthSP00_02_gloom01.webp");
const SP00_GLOOM_M2: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/C03FaceMouthSP00_02_gloom02.webp");
const SP00_GLOOM_M3: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/C03FaceMouthSP00_02_gloom03.webp");

const SP00_GGLASS_E1: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/C03FaceEyeSP00_03_gloomglass01.webp");
const SP00_GGLASS_E2: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/C03FaceEyeSP00_03_gloomglass02.webp");
const SP00_GGLASS_E3: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/C03FaceEyeSP00_03_gloomglass03.webp");
const SP00_GGLASS_M1: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/C03FaceMouthSP00_03_gloomglass01.webp");
const SP00_GGLASS_M2: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/C03FaceMouthSP00_03_gloomglass02.webp");
const SP00_GGLASS_M3: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/C03FaceMouthSP00_03_gloomglass03.webp");

const SP01_NORM_E1: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/C03FaceEyeSP01_00_Normal01.webp");
const SP01_NORM_E2: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/C03FaceEyeSP01_00_Normal02.webp");
const SP01_NORM_E3: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/C03FaceEyeSP01_00_Normal03.webp");
const SP01_NORM_M1: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/C03FaceMouthSP01_00_Normal01.webp");
const SP01_NORM_M2: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/C03FaceMouthSP01_00_Normal02.webp");
const SP01_NORM_M3: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/C03FaceMouthSP01_00_Normal03.webp");

const SP02_NORM_E1: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/C03FaceEyeSP02_00_Normal01.webp");
const SP02_NORM_E2: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/C03FaceEyeSP02_00_Normal02.webp");
const SP02_NORM_E3: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/C03FaceEyeSP02_00_Normal03.webp");
const SP02_NORM_M1: &[u8] = include_bytes!("../../assets/sprites/yuki_nagato_gekitou_wii/2x/C03FaceMouthSP02_00_Normal01.webp");

#[derive(Debug, Clone, Copy)]
pub struct SliceRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Clone, Copy)]
pub struct PoseGeometry {
    pub canvas_w: f32,
    pub canvas_h: f32,
    /// Target fraction of `screen_height` for clean proportions matching the Gentoo wallpaper.
    pub target_height_ratio: f32,
    pub top_hair: SliceRect,
    pub eyes: SliceRect,
    pub mouth: SliceRect,
    pub body: SliceRect,
}

/// Exact horizontal center between the rightmost pixel of the `"gentoo linux™"` logo (`1811 / 3840`)
/// and the right edge of `gentoo-wallpaper-base-dark.png` (`3840`): `(0.4716 + 1.0) / 2.0`.
pub const DESKTOP_SPRITE_CENTER_X_RATIO: f32 = 0.7358;

#[derive(Clone, Copy)]
pub struct VariantSpec {
    pub geometry: PoseGeometry,
    pub top_hair_bytes: &'static [u8],
    pub body_bytes: &'static [u8],
    pub eye_open_bytes: &'static [u8],
    pub eye_half_bytes: &'static [u8],
    pub eye_closed_bytes: &'static [u8],
    pub mouth_closed_bytes: &'static [u8],
    pub mouth_half_bytes: Option<&'static [u8]>,
    pub mouth_open_bytes: Option<&'static [u8]>,
}

const GEOM_SP00: PoseGeometry = PoseGeometry {
    canvas_w: 236.0,
    canvas_h: 584.0,
    target_height_ratio: 0.86,
    top_hair: SliceRect { x: 17.0, y: 0.0, w: 202.0, h: 80.0 },
    eyes: SliceRect { x: 25.0, y: 80.0, w: 206.0, h: 58.0 },
    mouth: SliceRect { x: 57.0, y: 138.0, w: 128.0, h: 24.0 },
    body: SliceRect { x: 0.0, y: 162.0, w: 236.0, h: 422.0 },
};

const GEOM_SP01: PoseGeometry = PoseGeometry {
    canvas_w: 234.0,
    canvas_h: 574.0,
    target_height_ratio: 0.86,
    top_hair: SliceRect { x: 26.0, y: 0.0, w: 178.0, h: 84.0 },
    eyes: SliceRect { x: 26.0, y: 84.0, w: 188.0, h: 58.0 },
    mouth: SliceRect { x: 59.0, y: 142.0, w: 124.0, h: 16.0 },
    body: SliceRect { x: 0.0, y: 158.0, w: 234.0, h: 416.0 },
};

const GEOM_SP02: PoseGeometry = PoseGeometry {
    canvas_w: 360.0,
    canvas_h: 474.0,
    target_height_ratio: 0.76,
    top_hair: SliceRect { x: 97.0, y: 0.0, w: 188.0, h: 84.0 },
    eyes: SliceRect { x: 111.0, y: 84.0, w: 184.0, h: 54.0 },
    mouth: SliceRect { x: 161.0, y: 138.0, w: 126.0, h: 60.0 },
    body: SliceRect { x: 0.0, y: 198.0, w: 360.0, h: 276.0 },
};

/// All 6 full-resolution Yuki Nagato Wii poses & expressions in rotation order.
pub const DESKTOP_VARIANTS: [VariantSpec; 6] = [
    VariantSpec {
        geometry: GEOM_SP00,
        top_hair_bytes: SP00_A,
        body_bytes: SP00_D,
        eye_open_bytes: SP00_NORM_E1,
        eye_half_bytes: SP00_NORM_E2,
        eye_closed_bytes: SP00_NORM_E3,
        mouth_closed_bytes: SP00_NORM_M3,
        mouth_half_bytes: Some(SP00_NORM_M2),
        mouth_open_bytes: Some(SP00_NORM_M1),
    },
    VariantSpec {
        geometry: GEOM_SP00,
        top_hair_bytes: SP00_A,
        body_bytes: SP00_D,
        eye_open_bytes: SP00_GLASS_E1,
        eye_half_bytes: SP00_GLASS_E2,
        eye_closed_bytes: SP00_GLASS_E3,
        mouth_closed_bytes: SP00_GLASS_M3,
        mouth_half_bytes: Some(SP00_GLASS_M2),
        mouth_open_bytes: Some(SP00_GLASS_M1),
    },
    VariantSpec {
        geometry: GEOM_SP01,
        top_hair_bytes: SP01_A,
        body_bytes: SP01_D,
        eye_open_bytes: SP01_NORM_E1,
        eye_half_bytes: SP01_NORM_E2,
        eye_closed_bytes: SP01_NORM_E3,
        mouth_closed_bytes: SP01_NORM_M3,
        mouth_half_bytes: Some(SP01_NORM_M2),
        mouth_open_bytes: Some(SP01_NORM_M1),
    },
    VariantSpec {
        geometry: GEOM_SP02,
        top_hair_bytes: SP02_A,
        body_bytes: SP02_D,
        eye_open_bytes: SP02_NORM_E1,
        eye_half_bytes: SP02_NORM_E2,
        eye_closed_bytes: SP02_NORM_E3,
        mouth_closed_bytes: SP02_NORM_M1,
        mouth_half_bytes: None,
        mouth_open_bytes: None,
    },
    VariantSpec {
        geometry: GEOM_SP00,
        top_hair_bytes: SP00_A,
        body_bytes: SP00_D,
        eye_open_bytes: SP00_GLOOM_E1,
        eye_half_bytes: SP00_GLOOM_E2,
        eye_closed_bytes: SP00_GLOOM_E3,
        mouth_closed_bytes: SP00_GLOOM_M3,
        mouth_half_bytes: Some(SP00_GLOOM_M2),
        mouth_open_bytes: Some(SP00_GLOOM_M1),
    },
    VariantSpec {
        geometry: GEOM_SP00,
        top_hair_bytes: SP00_A,
        body_bytes: SP00_D,
        eye_open_bytes: SP00_GGLASS_E1,
        eye_half_bytes: SP00_GGLASS_E2,
        eye_closed_bytes: SP00_GGLASS_E3,
        mouth_closed_bytes: SP00_GGLASS_M3,
        mouth_half_bytes: Some(SP00_GGLASS_M2),
        mouth_open_bytes: Some(SP00_GGLASS_M1),
    },
];

#[derive(Debug, Clone, Copy)]
pub struct LoadedVariantAssets {
    pub top_hair: ImageAsset,
    pub body: ImageAsset,
    pub eye_open: ImageAsset,
    pub eye_half: ImageAsset,
    pub eye_closed: ImageAsset,
    pub mouth_closed: ImageAsset,
    pub mouth_half: Option<ImageAsset>,
    pub mouth_open: Option<ImageAsset>,
}

impl LoadedVariantAssets {
    fn load(spec: &VariantSpec, ctx: &mut EffectCtx<'_>) -> Option<Self> {
        let mouth_half = match spec.mouth_half_bytes {
            Some(bytes) => Some(ctx.load_image_bytes(bytes).ok()?),
            None => None,
        };
        let mouth_open = match spec.mouth_open_bytes {
            Some(bytes) => Some(ctx.load_image_bytes(bytes).ok()?),
            None => None,
        };
        Some(Self {
            top_hair: ctx.load_image_bytes(spec.top_hair_bytes).ok()?,
            body: ctx.load_image_bytes(spec.body_bytes).ok()?,
            eye_open: ctx.load_image_bytes(spec.eye_open_bytes).ok()?,
            eye_half: ctx.load_image_bytes(spec.eye_half_bytes).ok()?,
            eye_closed: ctx.load_image_bytes(spec.eye_closed_bytes).ok()?,
            mouth_closed: ctx.load_image_bytes(spec.mouth_closed_bytes).ok()?,
            mouth_half,
            mouth_open,
        })
    }

    fn evict_unique_pixels(&self, next: Option<&Self>, ctx: &mut EffectCtx<'_>) {
        let keep = |id: ImageAsset| {
            next.is_some_and(|n| {
                n.top_hair.id == id.id
                    || n.body.id == id.id
                    || n.eye_open.id == id.id
                    || n.eye_half.id == id.id
                    || n.eye_closed.id == id.id
                    || n.mouth_closed.id == id.id
                    || n.mouth_half.is_some_and(|m| m.id == id.id)
                    || n.mouth_open.is_some_and(|m| m.id == id.id)
            })
        };
        for asset in [
            Some(self.top_hair),
            Some(self.body),
            Some(self.eye_open),
            Some(self.eye_half),
            Some(self.eye_closed),
            Some(self.mouth_closed),
            self.mouth_half,
            self.mouth_open,
        ]
        .into_iter()
        .flatten()
        {
            if !keep(asset) {
                let _ = ctx.evict_asset_pixels(asset);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouthFrame {
    Closed,
    HalfOpen,
    Open,
}

/// Calm two-syllable speech cadence played when an incoming notification arrives.
const MOUTH_REACT_STEPS: [(MouthFrame, Duration); 7] = [
    (MouthFrame::HalfOpen, Duration::from_millis(85)),
    (MouthFrame::Open, Duration::from_millis(150)),
    (MouthFrame::HalfOpen, Duration::from_millis(85)),
    (MouthFrame::Closed, Duration::from_millis(120)),
    (MouthFrame::HalfOpen, Duration::from_millis(85)),
    (MouthFrame::Open, Duration::from_millis(170)),
    (MouthFrame::HalfOpen, Duration::from_millis(90)),
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpriteFadePhase {
    Hold { elapsed_secs: u16 },
    FadingOut { opacity: f32, next_idx: usize },
    FadingIn { opacity: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlinkPhase {
    #[default]
    Open,
    HalfClosing,
    Closed,
    HalfOpening,
}

impl BlinkPhase {
    #[must_use]
    pub const fn duration(self) -> Duration {
        match self {
            Self::Open => Duration::from_millis(4200),
            Self::HalfClosing | Self::HalfOpening => Duration::from_millis(75),
            Self::Closed => Duration::from_millis(110),
        }
    }

    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Open => Self::HalfClosing,
            Self::HalfClosing => Self::Closed,
            Self::Closed => Self::HalfOpening,
            Self::HalfOpening => Self::Open,
        }
    }
}

pub struct DesktopSpriteState {
    pub active_idx: usize,
    pub phase: SpriteFadePhase,
    pub blink: BlinkPhase,
    pub mouth_step: Option<usize>,
    pub mouth_nonce: u32,
    pub active_assets: Option<LoadedVariantAssets>,
    cached_handles: [Option<LoadedVariantAssets>; 6],
}

impl Default for DesktopSpriteState {
    fn default() -> Self {
        Self {
            active_idx: 0,
            phase: SpriteFadePhase::Hold { elapsed_secs: 0 },
            blink: BlinkPhase::Open,
            mouth_step: None,
            mouth_nonce: 0,
            active_assets: None,
            cached_handles: [None; 6],
        }
    }
}

impl DesktopSpriteState {
    /// Initializes the first desktop sprite variant and starts a gentle initial fade-in.
    pub fn ensure_loaded(&mut self, ctx: &mut EffectCtx<'_>) {
        if self.active_assets.is_some() {
            return;
        }
        let idx = self.active_idx % DESKTOP_VARIANTS.len();
        if let Some(loaded) = self.load_or_cached(idx, ctx) {
            self.active_assets = Some(loaded);
            self.phase = SpriteFadePhase::FadingIn { opacity: 0.0 };
            self.blink = BlinkPhase::Open;
        }
    }

    fn load_or_cached(
        &mut self,
        idx: usize,
        ctx: &mut EffectCtx<'_>,
    ) -> Option<LoadedVariantAssets> {
        if let Some(cached) = self.cached_handles[idx] {
            return Some(cached);
        }
        let loaded = LoadedVariantAssets::load(&DESKTOP_VARIANTS[idx], ctx)?;
        self.cached_handles[idx] = Some(loaded);
        Some(loaded)
    }

    #[must_use]
    pub const fn is_visible(&self) -> bool {
        self.active_assets.is_some()
    }

    #[must_use]
    pub const fn is_fading(&self) -> bool {
        !matches!(self.phase, SpriteFadePhase::Hold { .. })
    }

    /// Computes `(scale, sprite_w, sprite_h, margin_right)` for the active pose on the current screen.
    #[must_use]
    pub fn layout_metrics(&self, bar_width: f32, screen_height: f32) -> (f32, f32, f32, f32) {
        let geom = self.current_geometry();
        let scale = ((screen_height * geom.target_height_ratio) / geom.canvas_h).clamp(0.6, 2.5);
        let sprite_w = (geom.canvas_w * scale).round().max(1.0);
        let sprite_h = (geom.canvas_h * scale).round().max(1.0);
        let target_center_x = bar_width * DESKTOP_SPRITE_CENTER_X_RATIO;
        let margin_right = (bar_width - (target_center_x + sprite_w * 0.5))
            .clamp(8.0, (bar_width - sprite_w - 8.0).max(8.0))
            .round();
        (scale, sprite_w, sprite_h, margin_right)
    }

    #[must_use]
    pub const fn opacity(&self) -> f32 {
        match self.phase {
            SpriteFadePhase::Hold { .. } => 1.0,
            SpriteFadePhase::FadingOut { opacity, .. } | SpriteFadePhase::FadingIn { opacity } => {
                opacity
            }
        }
    }

    #[must_use]
    pub fn current_geometry(&self) -> PoseGeometry {
        DESKTOP_VARIANTS[self.active_idx % DESKTOP_VARIANTS.len()].geometry
    }

    #[must_use]
    pub fn current_eye_asset(&self) -> Option<ImageAsset> {
        let assets = self.active_assets?;
        Some(match self.blink {
            BlinkPhase::Open => assets.eye_open,
            BlinkPhase::HalfClosing | BlinkPhase::HalfOpening => assets.eye_half,
            BlinkPhase::Closed => assets.eye_closed,
        })
    }

    #[must_use]
    pub fn current_mouth_frame(&self) -> MouthFrame {
        self.mouth_step
            .and_then(|idx| MOUTH_REACT_STEPS.get(idx))
            .map_or(MouthFrame::Closed, |(frame, _)| *frame)
    }

    #[must_use]
    pub fn current_mouth_asset(&self) -> Option<ImageAsset> {
        let assets = self.active_assets?;
        Some(match self.current_mouth_frame() {
            MouthFrame::Closed => assets.mouth_closed,
            MouthFrame::HalfOpen => assets.mouth_half.unwrap_or(assets.mouth_closed),
            MouthFrame::Open => assets.mouth_open.unwrap_or(assets.mouth_closed),
        })
    }

    /// Returns `(nonce, step, duration)` when a notification mouth-reaction timer should be active.
    #[must_use]
    pub fn mouth_timer(&self) -> Option<(u32, u8, Duration)> {
        let step = self.mouth_step?;
        let (_, dur) = MOUTH_REACT_STEPS.get(step)?;
        Some((self.mouth_nonce, step as u8, *dur))
    }

    /// Triggers a calm mouth reaction on incoming notification (no-op on `SP02` book pose which
    /// only has a single static closed-mouth frame).
    pub fn trigger_mouth_react(&mut self) {
        let Some(assets) = self.active_assets else {
            return;
        };
        if assets.mouth_open.is_none() && assets.mouth_half.is_none() {
            return;
        }
        self.mouth_nonce = self.mouth_nonce.wrapping_add(1);
        self.mouth_step = Some(0);
    }

    pub fn step_mouth(&mut self) {
        let Some(step) = self.mouth_step else {
            return;
        };
        let next = step + 1;
        if next < MOUTH_REACT_STEPS.len() {
            self.mouth_step = Some(next);
        } else {
            self.mouth_step = None;
        }
    }

    /// Triggers a smooth fade-out → next variant → fade-in rotation (used for idle timer).
    pub fn trigger_rotate(&mut self) {
        if !self.is_visible() {
            return;
        }
        let next_idx = (self.active_idx + 1) % DESKTOP_VARIANTS.len();
        match self.phase {
            SpriteFadePhase::Hold { .. } => {
                self.phase = SpriteFadePhase::FadingOut {
                    opacity: 1.0,
                    next_idx,
                };
            }
            SpriteFadePhase::FadingIn { opacity } => {
                self.phase = SpriteFadePhase::FadingOut { opacity, next_idx };
            }
            SpriteFadePhase::FadingOut { .. } => {}
        }
    }

    /// Instantly switches to the next sprite variant with zero fade delay (used on workspace switch),
    /// evicting the outgoing variant's unique decoded RGBA8 pixels immediately.
    pub fn rotate_instant(&mut self, ctx: &mut EffectCtx<'_>) {
        if !self.is_visible() {
            return;
        }
        let next_idx = (self.active_idx + 1) % DESKTOP_VARIANTS.len();
        let prev_assets = self.active_assets;
        let next_assets = self.load_or_cached(next_idx, ctx);
        if let Some(prev) = prev_assets {
            prev.evict_unique_pixels(next_assets.as_ref(), ctx);
        }
        self.active_idx = next_idx;
        self.active_assets = next_assets;
        self.blink = BlinkPhase::Open;
        self.mouth_step = None;
        self.phase = SpriteFadePhase::Hold { elapsed_secs: 0 };
    }

    /// Called once per second from `Msg::TimeTick`; triggers rotation after 3 minutes of hold.
    pub fn tick_second(&mut self) {
        if !self.is_visible() {
            return;
        }
        if let SpriteFadePhase::Hold { ref mut elapsed_secs } = self.phase {
            *elapsed_secs = elapsed_secs.saturating_add(1);
            if *elapsed_secs >= DESKTOP_SPRITE_HOLD_SECS {
                self.trigger_rotate();
            }
        }
    }

    /// Advances the 60fps fade animation, swapping and evicting old variant pixels at opacity 0.
    pub fn step_fade(&mut self, ctx: &mut EffectCtx<'_>) {
        match self.phase {
            SpriteFadePhase::Hold { .. } => {}
            SpriteFadePhase::FadingOut { opacity, next_idx } => {
                let next_op = (opacity - FADE_STEP).max(0.0);
                if next_op <= 0.001 {
                    let prev_assets = self.active_assets;
                    let next_assets = self.load_or_cached(next_idx, ctx);
                    if let Some(prev) = prev_assets {
                        prev.evict_unique_pixels(next_assets.as_ref(), ctx);
                    }
                    self.active_idx = next_idx;
                    self.active_assets = next_assets;
                    self.blink = BlinkPhase::Open;
                    self.mouth_step = None;
                    self.phase = SpriteFadePhase::FadingIn { opacity: 0.0 };
                } else {
                    self.phase = SpriteFadePhase::FadingOut {
                        opacity: next_op,
                        next_idx,
                    };
                }
            }
            SpriteFadePhase::FadingIn { opacity } => {
                let next_op = (opacity + FADE_STEP).min(1.0);
                if next_op >= 0.999 {
                    self.phase = SpriteFadePhase::Hold { elapsed_secs: 0 };
                } else {
                    self.phase = SpriteFadePhase::FadingIn { opacity: next_op };
                }
            }
        }
    }

    pub fn step_blink(&mut self) {
        if self.is_visible() {
            self.blink = self.blink.next();
        }
    }
}
