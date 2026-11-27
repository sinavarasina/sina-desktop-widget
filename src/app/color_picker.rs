//! Native WayWi `screencopy` Color Picker state with pixel-accurate loupe sampling
//! and Wayland clipboard output (`#rrggbb` / `rgb(r, g, b)`).

use std::sync::Arc;

use waywi::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorFormat {
    #[default]
    Hex,
    Rgb,
}

impl ColorFormat {
    #[must_use]
    pub const fn toggle(self) -> Self {
        match self {
            Self::Hex => Self::Rgb,
            Self::Rgb => Self::Hex,
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Hex => "HEX",
            Self::Rgb => "RGB",
        }
    }

    #[must_use]
    pub fn format_rgba(self, [r, g, b, _]: [u8; 4]) -> String {
        match self {
            Self::Hex => format!("#{r:02x}{g:02x}{b:02x}"),
            Self::Rgb => format!("rgb({r}, {g}, {b})"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorPickerPhase {
    #[default]
    Closed,
    Capturing,
    Active,
}

pub struct ColorPickerState {
    pub phase: ColorPickerPhase,
    pub capture_seq: u64,
    pub snapshot: Option<ImageAsset>,
    pub frame: Option<Arc<ScreencopyFrame>>,
    pub viewport_w: f32,
    pub viewport_h: f32,
    pub cursor_pos: (f32, f32),
    pub format: ColorFormat,
    pub last_copied: Option<String>,
    pub last_error: Option<String>,
}

impl Default for ColorPickerState {
    fn default() -> Self {
        Self {
            phase: ColorPickerPhase::Closed,
            capture_seq: 0,
            snapshot: None,
            frame: None,
            viewport_w: 1920.0,
            viewport_h: 1080.0,
            cursor_pos: (960.0, 540.0),
            format: ColorFormat::Hex,
            last_copied: None,
            last_error: None,
        }
    }
}

impl ColorPickerState {
    #[must_use]
    pub const fn is_open_or_capturing(&self) -> bool {
        !matches!(self.phase, ColorPickerPhase::Closed)
    }

    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(self.phase, ColorPickerPhase::Active)
    }

    #[must_use]
    pub const fn should_mount_surface(&self) -> bool {
        matches!(self.phase, ColorPickerPhase::Active)
            && self.snapshot.is_some()
            && self.frame.is_some()
    }

    pub fn begin_capture(
        &mut self,
        viewport_w: f32,
        viewport_h: f32,
        cursor_x: f32,
        cursor_y: f32,
    ) -> u64 {
        self.capture_seq = self.capture_seq.wrapping_add(1);
        self.phase = ColorPickerPhase::Capturing;
        self.viewport_w = viewport_w.max(320.0);
        self.viewport_h = viewport_h.max(240.0);
        self.cursor_pos = (
            cursor_x.clamp(0.0, self.viewport_w),
            cursor_y.clamp(0.0, self.viewport_h),
        );
        self.last_error = None;
        self.capture_seq
    }

    pub fn finish_capture(
        &mut self,
        seq: u64,
        asset: ImageAsset,
        frame: Arc<ScreencopyFrame>,
    ) -> Option<ImageAsset> {
        if seq != self.capture_seq || self.phase != ColorPickerPhase::Capturing {
            return Some(asset);
        }
        let prev = self.snapshot.replace(asset);
        self.frame = Some(frame);
        self.phase = ColorPickerPhase::Active;
        prev
    }

    pub fn fail_capture(&mut self, seq: u64, err: String) {
        if seq == self.capture_seq && self.phase == ColorPickerPhase::Capturing {
            self.phase = ColorPickerPhase::Closed;
            self.last_error = Some(err);
        }
    }

    pub fn close_immediate(&mut self) -> Option<ImageAsset> {
        self.capture_seq = self.capture_seq.wrapping_add(1);
        self.phase = ColorPickerPhase::Closed;
        self.frame = None;
        self.snapshot.take()
    }

    pub fn set_cursor(&mut self, x: f32, y: f32) {
        self.cursor_pos = (
            x.clamp(0.0, self.viewport_w),
            y.clamp(0.0, self.viewport_h),
        );
    }

    /// Maps logical overlay coordinates `(lx, ly)` to physical pixel coordinates inside `frame`.
    #[must_use]
    pub fn logical_to_pixel(&self, lx: f32, ly: f32) -> Option<(u32, u32)> {
        let frame = self.frame.as_ref()?;
        if frame.width == 0 || frame.height == 0 {
            return None;
        }
        let sx = frame.width as f32 / self.viewport_w.max(1.0);
        let sy = frame.height as f32 / self.viewport_h.max(1.0);
        let px = ((lx * sx).floor() as i64).clamp(0, (frame.width - 1) as i64) as u32;
        let py = ((ly * sy).floor() as i64).clamp(0, (frame.height - 1) as i64) as u32;
        Some((px, py))
    }

    /// Samples the RGBA8 color under the current cursor position.
    #[must_use]
    pub fn sampled_rgba(&self) -> [u8; 4] {
        let Some(frame) = self.frame.as_ref() else {
            return [30, 30, 46, 255];
        };
        let Some((px, py)) = self.logical_to_pixel(self.cursor_pos.0, self.cursor_pos.1) else {
            return [30, 30, 46, 255];
        };
        frame.pixel_rgba(px, py).unwrap_or([30, 30, 46, 255])
    }

    #[must_use]
    pub fn formatted_current(&self) -> String {
        self.format.format_rgba(self.sampled_rgba())
    }
}
