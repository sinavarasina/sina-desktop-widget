//! Animated screen magnifier supporting both:
//! - **Snapshot mode** (WayWi `screencopy` fullscreen overlay): silky enter/exit animation,
//!   central deadzone edge-pan follow, cursor-anchored scroll zoom, left-drag pan, and zero-copy
//!   GPU/CPU transforms; frees snapshot texture on close.
//! - **Live mode** (Hyprland native `cursor:zoom_factor` compositor zoom): real-time interactive
//!   60–144Hz compositor zoom with zero flicker/recursion, animated smoothly via VSync Hyprland
//!   IPC, with only a compact click-through bottom HUD pill mounted.

use std::time::{Duration, Instant};

use waywi::prelude::*;

/// Initial zoom factor when opening Magnifier mode.
pub const DEFAULT_ZOOM: f32 = 2.4;
/// Minimum zoom factor (`1.0x` = full screen).
pub const MIN_ZOOM: f32 = 1.0;
/// Maximum zoom factor.
pub const MAX_ZOOM: f32 = 10.0;
/// Multiplicative step per scroll notch / keyboard zoom press.
pub const ZOOM_STEP_FACTOR: f32 = 1.25;
/// Normalized central deadzone half-extent (`0.55` -> inner 55% steady, outer 45% glides).
pub const DEADZONE_HALF_FRAC: f32 = 0.55;
/// Maximum world-space pan speed (logical px/s at `1.0x`; divides by `zoom`) at the viewport edge.
const EDGE_PAN_MAX_SPEED: f32 = 920.0;
/// Exponential damping rate for zoom factor transitions (higher = snappier).
const ZOOM_DAMP_LAMBDA: f32 = 16.0;
/// Exponential damping rate for camera pan transitions.
const CAM_DAMP_LAMBDA: f32 = 14.0;
/// Exponential damping rate for closing zoom-out transition.
const CLOSE_DAMP_LAMBDA: f32 = 19.0;
/// HUD bar reveal animation speed.
const HUD_DAMP_LAMBDA: f32 = 18.0;
/// Short compositor settle delay when closing an overlay or recapturing before `capture_screen`.
pub const CAPTURE_SETTLE_DELAY: Duration = Duration::from_millis(40);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ZoomMode {
    /// Frozen WayWi `screencopy` snapshot overlay (full deadzone pan, scroll zoom, drag pan).
    #[default]
    Snapshot,
    /// Real-time interactive Hyprland native `cursor:zoom_factor` compositor zoom.
    Live,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ZoomPhase {
    #[default]
    Closed,
    /// Waiting for compositor settle / async `capture_screen` frame.
    Capturing,
    /// Interactive zoomed view (animating or settled).
    Active,
    /// Animating smoothly back to `1.0x` before unmounting & reclaiming resources.
    Closing,
}

/// Side effects produced by a single VSync [`ZoomState::advance`] step.
#[derive(Debug, Default)]
pub struct ZoomAdvanceResult {
    /// Snapshot texture to unload when a Snapshot close animation finishes.
    pub unloaded_snapshot: Option<ImageAsset>,
    /// New `cursor:zoom_factor` value to push via Hyprland IPC when in Live mode.
    pub live_zoom_update: Option<f32>,
    /// True when a Live mode close animation finished (reset `zoom_factor` to `1.0`).
    pub live_finished: bool,
}

pub struct ZoomState {
    pub mode: ZoomMode,
    pub phase: ZoomPhase,
    /// Monotonic token so stale async screencopy callbacks from cancelled opens are ignored.
    pub capture_seq: u64,
    /// Captured fullscreen snapshot texture (only present in `Snapshot` mode; unloaded on close/live).
    pub snapshot: Option<ImageAsset>,
    /// Logical viewport size (`screen_w`, `screen_h`) captured/displayed against.
    pub viewport_w: f32,
    pub viewport_h: f32,
    /// Current animated zoom factor.
    pub zoom: f32,
    /// Desired zoom factor.
    pub target_zoom: f32,
    /// Last `cursor:zoom_factor` sent to Hyprland in Live mode (avoids redundant IPC writes).
    pub live_last_pushed_zoom: f32,
    /// Current animated camera center in world coordinates (`0..viewport_w`, `0..viewport_h`).
    pub cam_x: f32,
    pub cam_y: f32,
    /// Desired camera center in world coordinates.
    pub target_cam_x: f32,
    pub target_cam_y: f32,
    /// Animated HUD visibility (`0.0..=1.0`).
    pub hud_progress: f32,
    /// Last known cursor position inside the overlay viewport (`0..viewport_w`, `0..viewport_h`).
    pub cursor_pos: Option<(f32, f32)>,
    /// Cursor position recorded when zoom was triggered (used to center the zoom-in animation).
    pub initial_focus: (f32, f32),
    /// Active left-button pan drag anchor `(last_screen_x, last_screen_y)`.
    pub drag_last: Option<(f32, f32)>,
    /// True when pointer is hovering the bottom HUD pill (suppresses edge-follow panning).
    pub pointer_over_hud: bool,
    /// True for crisp `ImageFilter::Nearest` / `zoom_disable_aa`; false for smooth bilinear.
    pub crisp_pixels: bool,
    /// Optional preserved target zoom across a `Recapture` (`R`) or Live->Snapshot cycle.
    pub pending_restore_zoom: Option<(f32, f32, f32)>,
    /// Timestamp of last VSync frame advance.
    pub last_tick: Option<Instant>,
    /// Optional capture error surfaced briefly if compositor lacks screencopy.
    pub last_error: Option<String>,
}

impl Default for ZoomState {
    fn default() -> Self {
        Self {
            mode: ZoomMode::Snapshot,
            phase: ZoomPhase::Closed,
            capture_seq: 0,
            snapshot: None,
            viewport_w: 1920.0,
            viewport_h: 1080.0,
            zoom: MIN_ZOOM,
            target_zoom: MIN_ZOOM,
            live_last_pushed_zoom: MIN_ZOOM,
            cam_x: 960.0,
            cam_y: 540.0,
            target_cam_x: 960.0,
            target_cam_y: 540.0,
            hud_progress: 0.0,
            cursor_pos: None,
            initial_focus: (960.0, 540.0),
            drag_last: None,
            pointer_over_hud: false,
            crisp_pixels: false,
            pending_restore_zoom: None,
            last_tick: None,
            last_error: None,
        }
    }
}

impl ZoomState {
    #[must_use]
    pub const fn is_open_or_capturing(&self) -> bool {
        !matches!(self.phase, ZoomPhase::Closed)
    }

    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(self.phase, ZoomPhase::Active)
    }

    #[must_use]
    pub const fn is_closing(&self) -> bool {
        matches!(self.phase, ZoomPhase::Closing)
    }

    #[must_use]
    pub const fn is_live(&self) -> bool {
        matches!(self.mode, ZoomMode::Live) && matches!(self.phase, ZoomPhase::Active | ZoomPhase::Closing)
    }

    /// True whenever the magnifier overlay (fullscreen snapshot OR compact Live HUD pill) should mount.
    #[must_use]
    pub const fn should_mount_surface(&self) -> bool {
        match self.mode {
            ZoomMode::Snapshot => {
                matches!(self.phase, ZoomPhase::Active | ZoomPhase::Closing)
                    && self.snapshot.is_some()
            }
            ZoomMode::Live => matches!(self.phase, ZoomPhase::Active | ZoomPhase::Closing),
        }
    }

    /// True whenever VSync `Animation::frames` must keep ticking for zoom/camera/HUD motion.
    #[must_use]
    pub fn is_animating(&self) -> bool {
        if !self.should_mount_surface() {
            return false;
        }
        if self.phase == ZoomPhase::Closing {
            return true;
        }
        let zoom_moving = (self.zoom - self.target_zoom).abs() > 0.002;
        let hud_moving = (self.hud_progress - 1.0).abs() > 0.005;
        if self.mode == ZoomMode::Live {
            return zoom_moving || hud_moving;
        }
        let cam_moving = (self.cam_x - self.target_cam_x).abs() > 0.15
            || (self.cam_y - self.target_cam_y).abs() > 0.15;
        let edge_panning = self.edge_pan_velocity().is_some();
        zoom_moving || cam_moving || hud_moving || edge_panning
    }

    /// Prepares a new Snapshot capture session and returns its sequence token.
    pub fn begin_capture(
        &mut self,
        viewport_w: f32,
        viewport_h: f32,
        focus_x: f32,
        focus_y: f32,
        restore: Option<(f32, f32, f32)>,
    ) -> u64 {
        self.capture_seq = self.capture_seq.wrapping_add(1);
        self.mode = ZoomMode::Snapshot;
        self.phase = ZoomPhase::Capturing;
        self.viewport_w = viewport_w.max(320.0);
        self.viewport_h = viewport_h.max(240.0);
        self.initial_focus = (
            focus_x.clamp(0.0, self.viewport_w),
            focus_y.clamp(0.0, self.viewport_h),
        );
        self.cursor_pos = Some(self.initial_focus);
        self.drag_last = None;
        self.pointer_over_hud = false;
        self.pending_restore_zoom = restore;
        self.last_error = None;
        self.last_tick = None;
        self.capture_seq
    }

    /// Activates Hyprland native Live compositor zoom mode, returning any previously held
    /// snapshot texture so the caller can immediately unload it.
    pub fn begin_live(
        &mut self,
        viewport_w: f32,
        viewport_h: f32,
        target_zoom: f32,
        now: Instant,
    ) -> Option<ImageAsset> {
        self.capture_seq = self.capture_seq.wrapping_add(1);
        let prev = self.snapshot.take();
        self.mode = ZoomMode::Live;
        self.phase = ZoomPhase::Active;
        self.viewport_w = viewport_w.max(320.0);
        self.viewport_h = viewport_h.max(240.0);
        self.zoom = MIN_ZOOM;
        self.target_zoom = target_zoom.clamp(MIN_ZOOM + 0.1, MAX_ZOOM);
        self.live_last_pushed_zoom = MIN_ZOOM;
        self.hud_progress = 0.0;
        self.drag_last = None;
        self.pointer_over_hud = false;
        self.pending_restore_zoom = None;
        self.last_error = None;
        self.last_tick = Some(now);
        prev
    }

    /// Activates Snapshot mode once the screencopy frame has been loaded into an [`ImageAsset`].
    pub fn finish_capture(
        &mut self,
        seq: u64,
        asset: ImageAsset,
        now: Instant,
    ) -> Option<ImageAsset> {
        if seq != self.capture_seq || self.phase != ZoomPhase::Capturing {
            return Some(asset);
        }
        let prev = self.snapshot.replace(asset);
        self.mode = ZoomMode::Snapshot;
        self.phase = ZoomPhase::Active;
        self.last_tick = Some(now);

        let w = self.viewport_w;
        let h = self.viewport_h;
        if let Some((saved_zoom, saved_cx, saved_cy)) = self.pending_restore_zoom.take() {
            let z = saved_zoom.clamp(MIN_ZOOM, MAX_ZOOM);
            let (cx, cy) = clamp_camera(saved_cx, saved_cy, z, w, h);
            self.zoom = z;
            self.target_zoom = z;
            self.cam_x = cx;
            self.cam_y = cy;
            self.target_cam_x = cx;
            self.target_cam_y = cy;
            self.hud_progress = 1.0;
        } else {
            // Start at exact 1:1 fullscreen so Frame 1 has zero visual pop, then animate into focus.
            self.zoom = MIN_ZOOM;
            self.cam_x = w * 0.5;
            self.cam_y = h * 0.5;
            self.target_zoom = DEFAULT_ZOOM;
            let (tx, ty) = clamp_camera(
                self.initial_focus.0,
                self.initial_focus.1,
                self.target_zoom,
                w,
                h,
            );
            self.target_cam_x = tx;
            self.target_cam_y = ty;
            self.hud_progress = 0.0;
        }
        prev
    }

    /// Cancels an in-flight capture (e.g. compositor returned an error or user closed early).
    pub fn fail_capture(&mut self, seq: u64, err: String) {
        if seq == self.capture_seq && self.phase == ZoomPhase::Capturing {
            self.phase = ZoomPhase::Closed;
            self.pending_restore_zoom = None;
            self.last_error = Some(err);
        }
    }

    /// Starts the smooth zoom-out exit animation back to `1.0x`.
    pub fn begin_close(&mut self, now: Instant) {
        match self.phase {
            ZoomPhase::Closed => {}
            ZoomPhase::Capturing => {
                self.capture_seq = self.capture_seq.wrapping_add(1);
                self.phase = ZoomPhase::Closed;
                self.pending_restore_zoom = None;
            }
            ZoomPhase::Active | ZoomPhase::Closing => {
                self.phase = ZoomPhase::Closing;
                self.drag_last = None;
                self.target_zoom = MIN_ZOOM;
                self.target_cam_x = self.viewport_w * 0.5;
                self.target_cam_y = self.viewport_h * 0.5;
                self.last_tick = Some(now);
            }
        }
    }

    /// Immediately closes and returns any owned snapshot [`ImageAsset`] so the caller can unload it.
    pub fn close_immediate(&mut self) -> Option<ImageAsset> {
        self.capture_seq = self.capture_seq.wrapping_add(1);
        self.phase = ZoomPhase::Closed;
        self.mode = ZoomMode::Snapshot;
        self.drag_last = None;
        self.pointer_over_hud = false;
        self.pending_restore_zoom = None;
        self.last_tick = None;
        self.zoom = MIN_ZOOM;
        self.target_zoom = MIN_ZOOM;
        self.live_last_pushed_zoom = MIN_ZOOM;
        self.hud_progress = 0.0;
        self.snapshot.take()
    }

    /// Steps zoom level by `steps` (`+1` = zoom in, `-1` = zoom out), keeping the world point
    /// under `anchor_screen` (or cursor/center) anchored in place in Snapshot mode.
    pub fn step_zoom(&mut self, steps: i32, anchor_screen: Option<(f32, f32)>, now: Instant) {
        if self.phase != ZoomPhase::Active || steps == 0 {
            return;
        }
        let factor = ZOOM_STEP_FACTOR.powi(steps);
        let old_target_zoom = self.target_zoom.max(MIN_ZOOM);
        let new_target_zoom = (old_target_zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        if (new_target_zoom - old_target_zoom).abs() < 1e-4 {
            return;
        }

        if self.mode == ZoomMode::Snapshot {
            let w = self.viewport_w;
            let h = self.viewport_h;
            let (ax, ay) = anchor_screen
                .or(self.cursor_pos)
                .unwrap_or((w * 0.5, h * 0.5));

            let world_x = self.target_cam_x + (ax - w * 0.5) / old_target_zoom;
            let world_y = self.target_cam_y + (ay - h * 0.5) / old_target_zoom;
            let next_cx = world_x - (ax - w * 0.5) / new_target_zoom;
            let next_cy = world_y - (ay - h * 0.5) / new_target_zoom;
            let (clamped_x, clamped_y) = clamp_camera(next_cx, next_cy, new_target_zoom, w, h);
            self.target_cam_x = clamped_x;
            self.target_cam_y = clamped_y;
        }

        self.target_zoom = new_target_zoom;
        self.last_tick.get_or_insert(now);
    }

    /// Resets view smoothly back to `DEFAULT_ZOOM` (or `1.0x` if already near default).
    pub fn reset_view(&mut self, now: Instant) {
        if self.phase != ZoomPhase::Active {
            return;
        }
        if self.mode == ZoomMode::Live {
            self.target_zoom = if (self.target_zoom - DEFAULT_ZOOM).abs() < 0.15 {
                MIN_ZOOM
            } else {
                DEFAULT_ZOOM
            };
            self.last_tick.get_or_insert(now);
            return;
        }

        let w = self.viewport_w;
        let h = self.viewport_h;
        if (self.target_zoom - DEFAULT_ZOOM).abs() < 0.15 {
            self.target_zoom = MIN_ZOOM;
            self.target_cam_x = w * 0.5;
            self.target_cam_y = h * 0.5;
        } else {
            self.target_zoom = DEFAULT_ZOOM;
            let (fx, fy) = self.cursor_pos.unwrap_or((w * 0.5, h * 0.5));
            let (cx, cy) = clamp_camera(fx, fy, self.target_zoom, w, h);
            self.target_cam_x = cx;
            self.target_cam_y = cy;
        }
        self.last_tick.get_or_insert(now);
    }

    /// Updates cursor position and handles manual left-button drag panning (Snapshot mode).
    pub fn on_pointer_move(&mut self, x: f32, y: f32, now: Instant) {
        if self.phase != ZoomPhase::Active || self.mode != ZoomMode::Snapshot {
            return;
        }
        let clamped_x = x.clamp(0.0, self.viewport_w);
        let clamped_y = y.clamp(0.0, self.viewport_h);
        self.cursor_pos = Some((clamped_x, clamped_y));

        if let Some((last_x, last_y)) = self.drag_last {
            let dx = clamped_x - last_x;
            let dy = clamped_y - last_y;
            self.drag_last = Some((clamped_x, clamped_y));
            let inv_zoom = 1.0 / self.zoom.max(MIN_ZOOM);
            let (nx, ny) = clamp_camera(
                self.target_cam_x - dx * inv_zoom,
                self.target_cam_y - dy * inv_zoom,
                self.target_zoom,
                self.viewport_w,
                self.viewport_h,
            );
            self.target_cam_x = nx;
            self.target_cam_y = ny;
            self.last_tick.get_or_insert(now);
        } else if self.edge_pan_velocity().is_some() {
            self.last_tick.get_or_insert(now);
        }
    }

    pub fn begin_drag(&mut self, x: f32, y: f32) {
        if self.phase == ZoomPhase::Active && self.mode == ZoomMode::Snapshot {
            self.drag_last = Some((x.clamp(0.0, self.viewport_w), y.clamp(0.0, self.viewport_h)));
        }
    }

    pub const fn end_drag(&mut self) {
        self.drag_last = None;
    }

    /// Returns `(vx, vy)` world-space camera glide velocity (px/s) when the pointer is outside
    /// the central deadzone and the camera still has room to pan in that direction.
    #[must_use]
    pub fn edge_pan_velocity(&self) -> Option<(f32, f32)> {
        if self.mode != ZoomMode::Snapshot
            || self.phase != ZoomPhase::Active
            || self.drag_last.is_some()
            || self.pointer_over_hud
            || self.target_zoom <= MIN_ZOOM + 0.02
        {
            return None;
        }
        let (cx, cy) = self.cursor_pos?;
        let half_w = (self.viewport_w * 0.5).max(1.0);
        let half_h = (self.viewport_h * 0.5).max(1.0);

        let nx = (cx - half_w) / half_w;
        let ny = (cy - half_h) / half_h;

        let raw_vx = deadzone_axis_speed(nx);
        let raw_vy = deadzone_axis_speed(ny);
        if raw_vx == 0.0 && raw_vy == 0.0 {
            return None;
        }

        let inv_zoom = 1.0 / self.zoom.max(MIN_ZOOM);
        let mut vx = raw_vx * EDGE_PAN_MAX_SPEED * inv_zoom;
        let mut vy = raw_vy * EDGE_PAN_MAX_SPEED * inv_zoom;

        let min_cx = half_w / self.target_zoom.max(MIN_ZOOM);
        let max_cx = (self.viewport_w - min_cx).max(min_cx);
        let min_cy = half_h / self.target_zoom.max(MIN_ZOOM);
        let max_cy = (self.viewport_h - min_cy).max(min_cy);

        if (vx < 0.0 && self.target_cam_x <= min_cx + 0.25)
            || (vx > 0.0 && self.target_cam_x >= max_cx - 0.25)
        {
            vx = 0.0;
        }
        if (vy < 0.0 && self.target_cam_y <= min_cy + 0.25)
            || (vy > 0.0 && self.target_cam_y >= max_cy - 0.25)
        {
            vy = 0.0;
        }

        if vx == 0.0 && vy == 0.0 {
            None
        } else {
            Some((vx, vy))
        }
    }

    /// Advances zoom, deadzone edge-pan, camera damping, and HUD reveal to `now`.
    pub fn advance(&mut self, now: Instant) -> ZoomAdvanceResult {
        let mut out = ZoomAdvanceResult::default();
        if !self.should_mount_surface() {
            self.last_tick = None;
            return out;
        }
        let dt = self.last_tick.map_or(1.0 / 60.0, |prev| {
            now.saturating_duration_since(prev)
                .as_secs_f32()
                .clamp(1.0 / 240.0, 0.05)
        });
        self.last_tick = Some(now);

        let is_closing = self.phase == ZoomPhase::Closing;
        let zoom_alpha =
            1.0 - (-dt * if is_closing { CLOSE_DAMP_LAMBDA } else { ZOOM_DAMP_LAMBDA }).exp();
        let hud_alpha = 1.0 - (-dt * HUD_DAMP_LAMBDA).exp();

        self.zoom += (self.target_zoom - self.zoom) * zoom_alpha;
        let hud_target = if is_closing { 0.0 } else { 1.0 };
        self.hud_progress += (hud_target - self.hud_progress) * hud_alpha;

        if self.mode == ZoomMode::Live {
            if is_closing {
                if (self.zoom - MIN_ZOOM).abs() < 0.015 {
                    self.close_immediate();
                    out.live_zoom_update = Some(MIN_ZOOM);
                    out.live_finished = true;
                    return out;
                }
            } else {
                if (self.zoom - self.target_zoom).abs() <= 0.002 {
                    self.zoom = self.target_zoom;
                }
                if (self.hud_progress - 1.0).abs() <= 0.005 {
                    self.hud_progress = 1.0;
                }
                if !self.is_animating() {
                    self.last_tick = None;
                }
            }
            if (self.zoom - self.live_last_pushed_zoom).abs() > 0.002 {
                self.live_last_pushed_zoom = self.zoom;
                out.live_zoom_update = Some(self.zoom.max(MIN_ZOOM));
            }
            return out;
        }

        // Snapshot mode camera & deadzone follow.
        if let Some((vx, vy)) = self.edge_pan_velocity() {
            let (nx, ny) = clamp_camera(
                self.target_cam_x + vx * dt,
                self.target_cam_y + vy * dt,
                self.target_zoom,
                self.viewport_w,
                self.viewport_h,
            );
            self.target_cam_x = nx;
            self.target_cam_y = ny;
        }

        let cam_alpha =
            1.0 - (-dt * if is_closing { CLOSE_DAMP_LAMBDA } else { CAM_DAMP_LAMBDA }).exp();
        self.cam_x += (self.target_cam_x - self.cam_x) * cam_alpha;
        self.cam_y += (self.target_cam_y - self.cam_y) * cam_alpha;
        let (clamped_x, clamped_y) = clamp_camera(
            self.cam_x,
            self.cam_y,
            self.zoom,
            self.viewport_w,
            self.viewport_h,
        );
        self.cam_x = clamped_x;
        self.cam_y = clamped_y;

        if is_closing {
            let zoom_done = (self.zoom - MIN_ZOOM).abs() < 0.01;
            let cam_done = (self.cam_x - self.viewport_w * 0.5).abs() < 1.5
                && (self.cam_y - self.viewport_h * 0.5).abs() < 1.5;
            if zoom_done && cam_done {
                out.unloaded_snapshot = self.close_immediate();
            }
        } else {
            if (self.zoom - self.target_zoom).abs() <= 0.002 {
                self.zoom = self.target_zoom;
            }
            if (self.cam_x - self.target_cam_x).abs() <= 0.15 {
                self.cam_x = self.target_cam_x;
            }
            if (self.cam_y - self.target_cam_y).abs() <= 0.15 {
                self.cam_y = self.target_cam_y;
            }
            if (self.hud_progress - 1.0).abs() <= 0.005 {
                self.hud_progress = 1.0;
            }
            if !self.is_animating() {
                self.last_tick = None;
            }
        }

        out
    }

    /// Computes the `(translate_x, translate_y, scale)` transform mapping world `(0..W, 0..H)`
    /// onto the fullscreen viewport using top-left pivot `(0.0, 0.0)`.
    #[must_use]
    pub fn image_transform(&self) -> Transform {
        let z = self.zoom.max(MIN_ZOOM);
        let tx = self.viewport_w * 0.5 - self.cam_x * z;
        let ty = self.viewport_h * 0.5 - self.cam_y * z;
        Transform::identity()
            .pivot(0.0, 0.0)
            .scale(z)
            .translate(tx.logical_px(), ty.logical_px())
    }

    #[must_use]
    pub const fn image_filter(&self) -> ImageFilter {
        if self.crisp_pixels {
            ImageFilter::Nearest
        } else {
            ImageFilter::Bilinear
        }
    }
}

/// Maps normalized axis position `n` in `[-1, 1]` to a smooth quadratic speed in `[-1, 1]`
/// with a deadzone of `DEADZONE_HALF_FRAC` around `0`.
#[inline]
#[must_use]
fn deadzone_axis_speed(n: f32) -> f32 {
    let abs = n.abs();
    if abs <= DEADZONE_HALF_FRAC {
        return 0.0;
    }
    let excess = ((abs - DEADZONE_HALF_FRAC) / (1.0 - DEADZONE_HALF_FRAC)).clamp(0.0, 1.0);
    n.signum() * excess * excess
}

/// Clamps camera center `(cx, cy)` so the zoomed viewport stays strictly inside `[0..w] x [0..h]`.
#[inline]
#[must_use]
pub fn clamp_camera(cx: f32, cy: f32, zoom: f32, w: f32, h: f32) -> (f32, f32) {
    let z = zoom.max(MIN_ZOOM);
    if z <= MIN_ZOOM + 1e-4 {
        return (w * 0.5, h * 0.5);
    }
    let half_w = (w * 0.5) / z;
    let half_h = (h * 0.5) / z;
    (
        cx.clamp(half_w, (w - half_w).max(half_w)),
        cy.clamp(half_h, (h - half_h).max(half_h)),
    )
}
