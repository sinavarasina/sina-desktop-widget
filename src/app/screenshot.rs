//! Native WayWi `screencopy` Screenshot tool state (Area, Window, and Full capture)
//! with PNG encoding, Wayland clipboard `image/png` export, and timestamped disk saving.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use waywi::prelude::*;
use waywi::system::LocalDateTime;

use super::anim::RevealAnim;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScreenshotMode {
    #[default]
    Area,
    Window,
    Full,
}

impl ScreenshotMode {
    #[must_use]
    pub const fn icon(self) -> &'static str {
        match self {
            // Verified present in MesloLGSNerdFont-Regular.ttf.
            Self::Area => "\u{f125}",
            Self::Window => "\u{f2d0}",
            Self::Full => "\u{f108}",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Area => "Area",
            Self::Window => "Window",
            Self::Full => "Full",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScreenshotPhase {
    #[default]
    Closed,
    Capturing,
    Active,
}

/// Output-local logical rectangle of a mapped Hyprland window on the active workspace.
#[derive(Debug, Clone, PartialEq)]
pub struct WindowTarget {
    pub title: String,
    pub class: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl WindowTarget {
    #[must_use]
    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px <= self.x + self.w && py >= self.y && py <= self.y + self.h
    }
}

/// Post-capture floating Android-style preview card state (auto-dismissed & unloaded after timeout).
pub struct ScreenshotPreview {
    pub seq: u64,
    pub thumb: ImageAsset,
    pub width_px: u32,
    pub height_px: u32,
    pub saved_path: Option<PathBuf>,
    pub png_bytes: Vec<u8>,
    pub copied: bool,
    pub hovered: bool,
    pub anim: RevealAnim,
}

/// Parameters for launching the post-capture floating preview card.
pub struct PreviewInit {
    pub thumb: ImageAsset,
    pub width_px: u32,
    pub height_px: u32,
    pub saved_path: Option<PathBuf>,
    pub png_bytes: Vec<u8>,
    pub copied: bool,
}

/// Background worker output from cropping, PNG-encoding, optionally saving, and downscaling
/// a screenshot preview thumbnail off the UI thread.
pub struct ScreenshotCommitOutcome {
    pub png_bytes: Vec<u8>,
    pub width_px: u32,
    pub height_px: u32,
    pub thumb_rgba: Vec<u8>,
    pub thumb_w: u32,
    pub thumb_h: u32,
    pub saved_path: Option<PathBuf>,
    pub save_error: Option<String>,
}

pub struct ScreenshotState {
    pub phase: ScreenshotPhase,
    pub capture_seq: u64,
    pub commit_seq: u64,
    pub snapshot: Option<ImageAsset>,
    pub frame: Option<Arc<ScreencopyFrame>>,
    pub viewport_w: f32,
    pub viewport_h: f32,
    pub mode: ScreenshotMode,
    /// When `true`, opening with `ScreenshotMode::Full` commits immediately upon screencopy arrival.
    pub auto_commit_full: bool,
    pub copy_to_clipboard: bool,
    pub save_to_file: bool,
    pub windows: Vec<WindowTarget>,
    pub cursor_pos: (f32, f32),
    pub drag_start: Option<(f32, f32)>,
    pub drag_current: Option<(f32, f32)>,
    pub last_saved_path: Option<PathBuf>,
    pub last_error: Option<String>,
    pub preview: Option<ScreenshotPreview>,
    preview_seq: u64,
}

impl Default for ScreenshotState {
    fn default() -> Self {
        Self {
            phase: ScreenshotPhase::Closed,
            capture_seq: 0,
            commit_seq: 0,
            snapshot: None,
            frame: None,
            viewport_w: 1920.0,
            viewport_h: 1080.0,
            mode: ScreenshotMode::Area,
            auto_commit_full: false,
            copy_to_clipboard: true,
            save_to_file: true,
            windows: Vec::new(),
            cursor_pos: (960.0, 540.0),
            drag_start: None,
            drag_current: None,
            last_saved_path: None,
            last_error: None,
            preview: None,
            preview_seq: 0,
        }
    }
}

/// Parameters for starting a new screenshot capture session.
pub struct ScreenshotStart {
    pub viewport_w: f32,
    pub viewport_h: f32,
    pub cursor_x: f32,
    pub cursor_y: f32,
    pub mode: ScreenshotMode,
    pub auto_commit_full: bool,
    pub windows: Vec<WindowTarget>,
}

impl ScreenshotState {
    #[must_use]
    pub const fn is_open_or_capturing(&self) -> bool {
        !matches!(self.phase, ScreenshotPhase::Closed)
    }

    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(self.phase, ScreenshotPhase::Active)
    }

    #[must_use]
    pub const fn should_mount_surface(&self) -> bool {
        matches!(self.phase, ScreenshotPhase::Active)
            && self.snapshot.is_some()
            && self.frame.is_some()
    }

    pub fn begin_capture(&mut self, start: ScreenshotStart) -> u64 {
        self.capture_seq = self.capture_seq.wrapping_add(1);
        self.phase = ScreenshotPhase::Capturing;
        self.viewport_w = start.viewport_w.max(320.0);
        self.viewport_h = start.viewport_h.max(240.0);
        self.cursor_pos = (
            start.cursor_x.clamp(0.0, self.viewport_w),
            start.cursor_y.clamp(0.0, self.viewport_h),
        );
        self.mode = start.mode;
        self.auto_commit_full = start.auto_commit_full;
        self.windows = start.windows;
        self.drag_start = None;
        self.drag_current = None;
        self.last_error = None;
        self.capture_seq
    }

    pub fn finish_capture(
        &mut self,
        seq: u64,
        asset: ImageAsset,
        frame: Arc<ScreencopyFrame>,
    ) -> Option<ImageAsset> {
        if seq != self.capture_seq || self.phase != ScreenshotPhase::Capturing {
            return Some(asset);
        }
        let prev = self.snapshot.replace(asset);
        self.frame = Some(frame);
        self.phase = ScreenshotPhase::Active;
        prev
    }

    pub fn fail_capture(&mut self, seq: u64, err: String) {
        if seq == self.capture_seq && self.phase == ScreenshotPhase::Capturing {
            self.phase = ScreenshotPhase::Closed;
            self.windows.clear();
            self.last_error = Some(err);
        }
    }

    pub fn next_commit_seq(&mut self) -> u64 {
        self.commit_seq = self.commit_seq.wrapping_add(1).max(1);
        self.commit_seq
    }

    pub fn close_immediate(&mut self) -> Option<ImageAsset> {
        self.capture_seq = self.capture_seq.wrapping_add(1);
        self.phase = ScreenshotPhase::Closed;
        self.auto_commit_full = false;
        self.frame = None;
        self.windows.clear();
        self.windows.shrink_to_fit();
        self.drag_start = None;
        self.drag_current = None;
        self.snapshot.take()
    }

    /// Shows the Android-style post-capture preview card, returning any previous preview thumbnail
    /// asset so the caller can immediately unload it.
    pub fn show_preview(&mut self, init: PreviewInit) -> Option<ImageAsset> {
        let prev = self.preview.take().map(|p| p.thumb);
        self.preview_seq = self.preview_seq.wrapping_add(1).max(1);
        let mut anim = RevealAnim::popup();
        anim.set_open(true);
        self.preview = Some(ScreenshotPreview {
            seq: self.preview_seq,
            thumb: init.thumb,
            width_px: init.width_px,
            height_px: init.height_px,
            saved_path: init.saved_path,
            png_bytes: init.png_bytes,
            copied: init.copied,
            hovered: false,
            anim,
        });
        prev
    }

    pub fn dismiss_preview(&mut self) {
        if let Some(p) = &mut self.preview {
            p.hovered = false;
            p.anim.set_open(false);
        }
    }

    pub fn clear_preview_immediate(&mut self) -> Option<ImageAsset> {
        self.preview.take().map(|p| p.thumb)
    }

    #[must_use]
    pub fn has_visible_preview(&self) -> bool {
        self.preview.as_ref().is_some_and(|p| p.anim.is_visible())
    }

    #[must_use]
    pub fn is_animating_preview(&self) -> bool {
        self.preview.as_ref().is_some_and(|p| p.anim.is_animating())
    }

    /// Advances the preview reveal animation and returns its thumbnail asset to unload once closed.
    pub fn advance_preview_animation(&mut self, now: Instant) -> Option<ImageAsset> {
        let p = self.preview.as_mut()?;
        p.anim.advance(now);
        if !p.anim.is_visible() {
            return self.preview.take().map(|done| done.thumb);
        }
        None
    }

    /// Sequence ID of an active unhovered preview eligible for the auto-dismiss timer.
    #[must_use]
    pub fn active_preview_timeout_seq(&self) -> Option<u64> {
        let p = self.preview.as_ref()?;
        (p.anim.is_open() && !p.hovered).then_some(p.seq)
    }

    /// Asset ID currently held by an active post-capture preview thumbnail, if any.
    #[must_use]
    pub fn preview_asset_id(&self) -> Option<AssetId> {
        self.preview.as_ref().map(|p| p.thumb.id)
    }

    /// Removes the saved screenshot file on disk (if any) and clears `last_saved_path` when it matches.
    pub fn delete_saved_file(&mut self, path: Option<PathBuf>) -> bool {
        let Some(path) = path else {
            return false;
        };
        let removed = std::fs::remove_file(&path).is_ok();
        if self.last_saved_path.as_ref() == Some(&path) {
            self.last_saved_path = None;
        }
        removed
    }

    /// Returns the frontmost window under `cursor_pos` in `Window` mode.
    #[must_use]
    pub fn hovered_window(&self) -> Option<&WindowTarget> {
        let (cx, cy) = self.cursor_pos;
        self.windows.iter().find(|w| w.contains(cx, cy))
    }

    /// Returns the normalized logical selection rectangle `(x, y, w, h)` currently highlighted.
    #[must_use]
    pub fn active_selection_rect(&self) -> Option<(f32, f32, f32, f32)> {
        match self.mode {
            ScreenshotMode::Area => {
                let (x0, y0) = self.drag_start?;
                let (x1, y1) = self.drag_current?;
                let x = x0.min(x1).clamp(0.0, self.viewport_w);
                let y = y0.min(y1).clamp(0.0, self.viewport_h);
                let w = (x1 - x0).abs().min(self.viewport_w - x);
                let h = (y1 - y0).abs().min(self.viewport_h - y);
                (w >= 2.0 && h >= 2.0).then_some((x, y, w, h))
            }
            ScreenshotMode::Window => self
                .hovered_window()
                .map(|w| (w.x, w.y, w.w, w.h)),
            ScreenshotMode::Full => Some((0.0, 0.0, self.viewport_w, self.viewport_h)),
        }
    }
}

/// Crops a logical rectangle `(x, y, w, h)` from `frame` into packed RGBA8 pixels.
fn crop_rgba(
    frame: &ScreencopyFrame,
    rect_logical: (f32, f32, f32, f32),
    viewport_w: f32,
    viewport_h: f32,
) -> std::result::Result<(Vec<u8>, u32, u32), String> {
    if frame.width == 0 || frame.height == 0 {
        return Err("empty screencopy frame".to_owned());
    }
    let sx = frame.width as f32 / viewport_w.max(1.0);
    let sy = frame.height as f32 / viewport_h.max(1.0);

    let (lx, ly, lw, lh) = rect_logical;
    let x0 = ((lx * sx).round() as i64).clamp(0, (frame.width - 1) as i64) as u32;
    let y0 = ((ly * sy).round() as i64).clamp(0, (frame.height - 1) as i64) as u32;
    let x1 = (((lx + lw) * sx).round() as i64).clamp((x0 + 1) as i64, frame.width as i64) as u32;
    let y1 = (((ly + lh) * sy).round() as i64).clamp((y0 + 1) as i64, frame.height as i64) as u32;
    let pw = x1.saturating_sub(x0).max(1);
    let ph = y1.saturating_sub(y0).max(1);

    let row_bytes = (pw as usize) * 4;
    let mut cropped = Vec::with_capacity(row_bytes * (ph as usize));
    let stride = frame.stride as usize;
    for row in y0..y1 {
        let start = (row as usize) * stride + (x0 as usize) * 4;
        let end = start + row_bytes;
        let slice = frame
            .rgba
            .get(start..end)
            .ok_or_else(|| "screencopy pixel slice out of bounds".to_owned())?;
        cropped.extend_from_slice(slice);
    }
    Ok((cropped, pw, ph))
}

fn encode_png_rgba(rgba: &[u8], pw: u32, ph: u32) -> std::result::Result<Vec<u8>, String> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, pw, ph);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::Fast);
        let mut writer = encoder
            .write_header()
            .map_err(|e| format!("png header error: {e}"))?;
        writer
            .write_image_data(rgba)
            .map_err(|e| format!("png write error: {e}"))?;
    }
    Ok(out)
}

/// Fast area-averaged RGBA8 downscaler for post-capture preview thumbnails (max `max_edge` px).
/// Avoids decoding the freshly encoded PNG back on the UI thread and keeps preview RAM tiny.
fn downscale_rgba_preview(rgba: &[u8], src_w: u32, src_h: u32, max_edge: u32) -> (Vec<u8>, u32, u32) {
    let longest = src_w.max(src_h);
    if longest <= max_edge || max_edge == 0 {
        return (rgba.to_vec(), src_w, src_h);
    }
    let scale = (max_edge as f64) / (longest as f64);
    let dst_w = ((src_w as f64 * scale).round() as u32).clamp(1, src_w);
    let dst_h = ((src_h as f64 * scale).round() as u32).clamp(1, src_h);
    let mut out = vec![0u8; (dst_w as usize) * (dst_h as usize) * 4];
    let sw = src_w as usize;
    let sh = src_h as usize;
    let dw = dst_w as usize;
    let dh = dst_h as usize;

    for dy in 0..dh {
        let sy0 = (dy * sh) / dh;
        let sy1 = (((dy + 1) * sh) / dh).max(sy0 + 1).min(sh);
        for dx in 0..dw {
            let sx0 = (dx * sw) / dw;
            let sx1 = (((dx + 1) * sw) / dw).max(sx0 + 1).min(sw);
            let mut r = 0u32;
            let mut g = 0u32;
            let mut b = 0u32;
            let mut a = 0u32;
            let mut count = 0u32;
            for sy in sy0..sy1 {
                let row_off = sy * sw * 4;
                for sx in sx0..sx1 {
                    let i = row_off + sx * 4;
                    r += u32::from(rgba[i]);
                    g += u32::from(rgba[i + 1]);
                    b += u32::from(rgba[i + 2]);
                    a += u32::from(rgba[i + 3]);
                    count += 1;
                }
            }
            let dst = (dy * dw + dx) * 4;
            let half = count / 2;
            out[dst] = ((r + half) / count) as u8;
            out[dst + 1] = ((g + half) / count) as u8;
            out[dst + 2] = ((b + half) / count) as u8;
            out[dst + 3] = ((a + half) / count) as u8;
        }
    }
    (out, dst_w, dst_h)
}

/// Crops a logical rectangle `(x, y, w, h)` from `frame` and encodes it as lossless RGBA8 PNG bytes.
#[cfg(test)]
pub fn crop_and_encode_png(
    frame: &ScreencopyFrame,
    rect_logical: (f32, f32, f32, f32),
    viewport_w: f32,
    viewport_h: f32,
) -> std::result::Result<(Vec<u8>, u32, u32), String> {
    let (cropped, pw, ph) = crop_rgba(frame, rect_logical, viewport_w, viewport_h)?;
    let png_bytes = encode_png_rgba(&cropped, pw, ph)?;
    Ok((png_bytes, pw, ph))
}

/// Runs crop, compact preview thumbnail downscaling, PNG encoding, and optional disk save
/// (intended for WayWi's background blocking task worker so the UI thread never stalls).
pub fn crop_encode_and_save_screenshot(
    frame: &ScreencopyFrame,
    rect_logical: (f32, f32, f32, f32),
    viewport_w: f32,
    viewport_h: f32,
    save_path: Option<PathBuf>,
) -> std::result::Result<ScreenshotCommitOutcome, String> {
    const PREVIEW_THUMB_MAX_EDGE: u32 = 560;
    let (cropped, pw, ph) = crop_rgba(frame, rect_logical, viewport_w, viewport_h)?;
    let (thumb_rgba, thumb_w, thumb_h) =
        downscale_rgba_preview(&cropped, pw, ph, PREVIEW_THUMB_MAX_EDGE);
    let png_bytes = encode_png_rgba(&cropped, pw, ph)?;
    drop(cropped);

    let mut saved_path = None;
    let mut save_error = None;
    if let Some(path) = save_path {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match std::fs::write(&path, &png_bytes) {
            Ok(()) => saved_path = Some(path),
            Err(err) => save_error = Some(err.to_string()),
        }
    }

    Ok(ScreenshotCommitOutcome {
        png_bytes,
        width_px: pw,
        height_px: ph,
        thumb_rgba,
        thumb_w,
        thumb_h,
        saved_path,
        save_error,
    })
}

/// Resolves the default timestamped screenshot output path (`~/Pictures/Screenshots/...`).
#[must_use]
pub fn default_screenshot_path(now: LocalDateTime) -> PathBuf {
    let base = std::env::var_os("XDG_PICTURES_DIR")
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .filter(|s| !s.is_empty())
                .map(|h| PathBuf::from(h).join("Pictures"))
        })
        .unwrap_or_else(std::env::temp_dir);
    let dir = base.join("Screenshots");
    let filename = format!(
        "screenshot-{:04}{:02}{:02}-{:02}{:02}{:02}.png",
        now.year, now.month, now.day, now.hour, now.minute, now.second
    );
    dir.join(filename)
}
