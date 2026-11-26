//! Native `drun` application launcher state, lazy XDG entry cache, and input metrics.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use waywi::desktop::icons::XdgIconLookup;
use waywi::prelude::*;

use crate::theme::{BAR_HEIGHT, DRUN_INPUT_MIN_W, POPUP_TOP_MARGIN, font_ui};

use super::anim::RevealAnim;
use super::asset_lru::PathAssetCache;

const DRUN_ANIM_DURATION: Duration = Duration::from_millis(160);
const DRUN_ICON_PX: u16 = 32;
const DRUN_VISIBLE_SLOTS: usize = 5;

pub struct DrunState {
    pub anim: RevealAnim,
    pub input: TextInputState,
    pub entries: Vec<DesktopEntry>,
    pub scanned: bool,
    pub match_indices: Vec<usize>,
    pub selected: usize,
    pub scroll_start: usize,
    pub launch_counts: HashMap<String, u32>,
    pub measured_text_w: f32,
    pub cursor_x: f32,
    pub marquee_tick: u32,
}

impl Default for DrunState {
    fn default() -> Self {
        Self {
            anim: RevealAnim::new(DRUN_ANIM_DURATION, Easing::CubicOut),
            input: TextInputState::default(),
            entries: Vec::new(),
            scanned: false,
            match_indices: Vec::new(),
            selected: 0,
            scroll_start: 0,
            launch_counts: HashMap::new(),
            measured_text_w: 0.0,
            cursor_x: 0.0,
            marquee_tick: 0,
        }
    }
}

impl DrunState {
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.anim.is_open()
    }

    #[must_use]
    pub fn progress(&self) -> f32 {
        self.anim.progress()
    }

    #[must_use]
    pub fn is_visible(&self) -> bool {
        self.anim.is_visible()
    }

    #[must_use]
    pub fn is_animating(&self) -> bool {
        self.anim.is_animating()
    }

    pub fn advance_animation(&mut self, now: Instant) {
        self.anim.advance(now);
    }

    pub fn ensure_scanned(&mut self) {
        if !self.scanned {
            self.entries = scan_desktop_entries();
            self.scanned = true;
        }
    }

    pub fn open(&mut self, ctx: &mut EffectCtx<'_, crate::app::msg::Msg>) {
        self.anim.set_open(true);
        self.input.clear();
        self.selected = 0;
        self.scroll_start = 0;
        self.marquee_tick = 0;
        self.ensure_scanned();
        self.refresh_matches();
        self.remeasure_input(ctx);
    }

    pub fn close(&mut self) {
        self.anim.set_open(false);
    }

    pub fn refresh_matches(&mut self) {
        let query = self.input.display_text();
        let matches =
            search_desktop_entries_with_weights(&self.entries, &query, Some(&self.launch_counts));
        self.match_indices = matches
            .into_iter()
            .filter_map(|m| self.entries.iter().position(|e| std::ptr::eq(e, m.entry)))
            .collect();
        self.selected = 0;
        self.scroll_start = 0;
    }

    pub fn remeasure_input(&mut self, ctx: &mut EffectCtx<'_, crate::app::msg::Msg>) {
        let display = self.input.display_text();
        let cursor_byte = self.input.display_cursor_bytes();
        let metrics = ctx.measure_text_cursor(&display, &font_ui(), 11.5.logical_px(), cursor_byte);
        self.measured_text_w = metrics.size.width.get();
        self.cursor_x = metrics.cursor_x.get();
    }

    pub fn move_selection(&mut self, delta: isize) {
        let len = self.match_indices.len();
        if len == 0 {
            self.selected = 0;
            self.scroll_start = 0;
            return;
        }
        let next = if delta >= 0 {
            self.selected.saturating_add(delta as usize).min(len - 1)
        } else {
            self.selected.saturating_sub(delta.unsigned_abs())
        };
        self.selected = next;
        if self.selected < self.scroll_start {
            self.scroll_start = self.selected;
        } else if self.selected >= self.scroll_start + DRUN_VISIBLE_SLOTS {
            self.scroll_start = self.selected - (DRUN_VISIBLE_SLOTS - 1);
        }
    }

    #[must_use]
    pub fn visible_match_indices(&self) -> &[usize] {
        let len = self.match_indices.len();
        if len == 0 {
            return &[];
        }
        let start = self.scroll_start.min(len.saturating_sub(1));
        let end = (start + DRUN_VISIBLE_SLOTS).min(len);
        &self.match_indices[start..end]
    }

    #[must_use]
    pub fn selected_entry(&self) -> Option<&DesktopEntry> {
        let entry_idx = *self.match_indices.get(self.selected)?;
        self.entries.get(entry_idx)
    }

    #[must_use]
    pub fn input_box_width(&self, bar_width: f32) -> f32 {
        let max_w = (bar_width * 0.40).max(220.0);
        (self.measured_text_w + 26.0).clamp(DRUN_INPUT_MIN_W, max_w)
    }

    #[must_use]
    pub fn ime_cursor_rect(&self, bar_width: f32) -> Bounds {
        let input_w = self.input_box_width(bar_width);
        let max_inner = (input_w - 12.0).max(16.0);
        let rel_x = self.cursor_x.min(max_inner);
        // Offset: runner pill (28) + ArrowR(CRUST->MANTLE) (11) + inner padding (6) = 45px.
        Bounds::new(
            (39.0 + 6.0 + rel_x).logical_px(),
            (POPUP_TOP_MARGIN + 3.0).logical_px(),
            2.logical_px(),
            (BAR_HEIGHT - 6.0).logical_px(),
        )
    }

    pub fn resolve_entry_icon(
        entry: &DesktopEntry,
        icon_lookup: &mut XdgIconLookup,
        path_cache: &mut PathAssetCache,
        ctx: &mut EffectCtx<'_, crate::app::msg::Msg>,
    ) -> Option<ImageAsset> {
        let icon_name = entry
            .icon
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())?;
        let sig = format!("drun-icon:{icon_name}");
        if let Some(&cached) = path_cache.get(&sig) {
            return Some(cached);
        }
        let path = if std::path::Path::new(icon_name).is_absolute() {
            std::path::PathBuf::from(icon_name)
        } else {
            icon_lookup.resolve(icon_name, DRUN_ICON_PX)?
        };
        if let Ok(asset) = ctx.load_image_sized(&path, u32::from(DRUN_ICON_PX)) {
            path_cache.insert(sig, asset);
            return Some(asset);
        }
        None
    }
}
