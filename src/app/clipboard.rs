//! Bounded Wayland clipboard history state (`ext-data-control-v1` / `zwlr-data-control-v1`)
//! supporting text entries, color swatch detection, PNG/image thumbnails, pinning, and
//! deterministic memory reclamation on eviction/clear.

use std::path::{Path, PathBuf};
use std::time::Instant;

use waywi::desktop::spawn_detached;
use waywi::prelude::*;

use super::anim::RevealAnim;

/// Maximum total clipboard history entries retained in RAM.
pub const MAX_CLIPBOARD_ENTRIES: usize = 30;
/// Maximum image entries retained in RAM (keeps decoded thumbnail + raw payload bounded).
const MAX_CLIPBOARD_IMAGES: usize = 6;
/// Maximum raw image byte payload kept per clipboard image entry (4 MiB).
const MAX_IMAGE_BYTES: usize = 4 * 1024 * 1024;
/// Maximum UTF-8 bytes stored per clipboard text entry (64 KiB).
const MAX_TEXT_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone)]
pub enum ClipboardEntryKind {
    Text {
        content: String,
        /// Parsed CSS/hex swatch color if `content.trim()` is `#rrggbb` or `#rgb`.
        hex_color: Option<Color>,
    },
    Image {
        mime: String,
        bytes: Vec<u8>,
        thumb: Option<ImageAsset>,
        width_px: u32,
        height_px: u32,
    },
}

#[derive(Debug, Clone)]
pub struct ClipboardEntry {
    pub id: u64,
    pub pinned: bool,
    pub kind: ClipboardEntryKind,
}

impl ClipboardEntry {
    #[must_use]
    pub fn preview_line(&self) -> String {
        match &self.kind {
            ClipboardEntryKind::Text { content, .. } => content
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .collect::<Vec<_>>()
                .join(" "),
            ClipboardEntryKind::Image {
                mime,
                width_px,
                height_px,
                bytes,
                ..
            } => {
                let kind = mime
                    .strip_prefix("image/")
                    .unwrap_or("img")
                    .to_ascii_uppercase();
                let kib = bytes.len() / 1024;
                if *width_px > 0 && *height_px > 0 {
                    format!("{kind} · {width_px}×{height_px} ({kib} KiB)")
                } else {
                    format!("{kind} image ({kib} KiB)")
                }
            }
        }
    }

    #[must_use]
    pub const fn hex_color(&self) -> Option<Color> {
        match &self.kind {
            ClipboardEntryKind::Text { hex_color, .. } => *hex_color,
            ClipboardEntryKind::Image { .. } => None,
        }
    }

    /// True if this entry can be opened externally via `xdg-open` (any image, URL, or existing path).
    #[must_use]
    pub fn can_open(&self) -> bool {
        match &self.kind {
            ClipboardEntryKind::Image { .. } => true,
            ClipboardEntryKind::Text { content, .. } => is_openable_target(content.trim()),
        }
    }
}

#[must_use]
fn is_openable_target(trimmed: &str) -> bool {
    if trimmed.is_empty() || trimmed.contains('\n') || trimmed.len() > 2048 {
        return false;
    }
    if trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("file://")
    {
        return true;
    }
    trimmed.starts_with('/') && Path::new(trimmed).exists()
}

/// Returns a private per-user runtime staging path for opening in-memory images via `xdg-open`.
#[must_use]
pub fn temp_open_image_path(stem: &str, ext: &str) -> PathBuf {
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let dir = base.join("sina-desktop-widget");
    let _ = std::fs::create_dir_all(&dir);
    dir.join(format!("{stem}.{ext}"))
}

pub struct ClipboardState {
    pub entries: Vec<ClipboardEntry>,
    pub popup_anim: RevealAnim,
    pub scroll_index: usize,
    pub popup_left_x: f32,
    next_id: u64,
}

impl Default for ClipboardState {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            popup_anim: RevealAnim::popup(),
            scroll_index: 0,
            popup_left_x: 0.0,
            next_id: 1,
        }
    }
}

impl ClipboardState {
    #[must_use]
    pub const fn is_popup_open(&self) -> bool {
        self.popup_anim.is_open()
    }

    #[must_use]
    pub fn is_popup_visible(&self) -> bool {
        self.popup_anim.is_visible()
    }

    #[must_use]
    pub fn is_animating_popup(&self) -> bool {
        self.popup_anim.is_animating()
    }

    #[must_use]
    pub fn popup_progress(&self) -> f32 {
        self.popup_anim.progress()
    }

    pub fn open_popup(&mut self) {
        self.popup_anim.set_open(true);
    }

    pub fn close_popup(&mut self) {
        self.popup_anim.set_open(false);
    }

    pub fn advance_popup_animation(&mut self, now: Instant) {
        self.popup_anim.advance(now);
    }

    /// Returns the parsed swatch color if the most recent clipboard entry is a hex color literal.
    #[must_use]
    pub fn latest_hex_color(&self) -> Option<Color> {
        self.entries.first().and_then(ClipboardEntry::hex_color)
    }

    /// True if any clipboard image entry currently holds `id` as its live thumbnail asset.
    #[must_use]
    pub fn uses_asset(&self, id: AssetId) -> bool {
        self.entries.iter().any(|e| {
            matches!(
                e.kind,
                ClipboardEntryKind::Image {
                    thumb: Some(asset),
                    ..
                } if asset.id == id
            )
        })
    }

    /// Ensures every image entry has a live decoded thumbnail in WayWi's asset cache before
    /// displaying the Clipboard popup (re-hydrating any asset that was evicted or unloaded).
    pub fn ensure_thumbs_loaded(&mut self, ctx: &mut EffectCtx<'_, crate::app::msg::Msg>) {
        for entry in &mut self.entries {
            if let ClipboardEntryKind::Image {
                bytes,
                thumb,
                width_px,
                height_px,
                ..
            } = &mut entry.kind
            {
                let stale = thumb.is_none_or(|a| ctx.image_retention(a).is_none());
                if stale && let Ok(asset) = ctx.load_image_bytes(bytes) {
                    *width_px = asset.width;
                    *height_px = asset.height;
                    *thumb = Some(asset);
                }
            }
        }
    }

    /// Unloads decoded clipboard thumbnail assets when the popup closes to keep idle RAM minimal,
    /// preserving any asset still referenced by `keep_asset` (e.g. an active screenshot preview).
    pub fn evict_thumbs(
        &mut self,
        ctx: &mut EffectCtx<'_, crate::app::msg::Msg>,
        keep_asset: Option<AssetId>,
    ) {
        let mut unloaded_any = false;
        for entry in &mut self.entries {
            if let ClipboardEntryKind::Image { thumb, .. } = &mut entry.kind
                && let Some(asset) = thumb.take()
                && keep_asset != Some(asset.id)
            {
                ctx.unload_image(asset);
                unloaded_any = true;
            }
        }
        if unloaded_any {
            ctx.trim_memory();
        }
    }

    /// Records a new Wayland clipboard snapshot, deduplicating existing entries and enforcing
    /// bounded RAM/image limits.
    pub fn push_snapshot(
        &mut self,
        snap: ClipboardSnapshot,
        ctx: &mut EffectCtx<'_, crate::app::msg::Msg>,
        keep_asset: Option<AssetId>,
    ) {
        if snap.selection != ClipboardSelection::Clipboard || snap.cleared {
            return;
        }

        if let Some(raw_text) = snap.text {
            if raw_text.trim().is_empty() || raw_text.len() > MAX_TEXT_BYTES {
                return;
            }
            if let Some(pos) = self.entries.iter().position(|e| {
                matches!(&e.kind, ClipboardEntryKind::Text { content, .. } if content == &raw_text)
            }) {
                let existing = self.entries.remove(pos);
                self.insert_sorted_top(existing);
                return;
            }
            let hex_color = parse_hex_color(raw_text.trim());
            let entry = ClipboardEntry {
                id: self.alloc_id(),
                pinned: false,
                kind: ClipboardEntryKind::Text {
                    content: raw_text,
                    hex_color,
                },
            };
            self.insert_sorted_top(entry);
            self.enforce_limits(ctx, keep_asset);
            return;
        }

        let Some(mime) = snap.primary_mime else {
            return;
        };
        if !mime.starts_with("image/") || snap.bytes.is_empty() || snap.bytes.len() > MAX_IMAGE_BYTES
        {
            return;
        }
        if let Some(pos) = self.entries.iter().position(|e| {
            matches!(&e.kind, ClipboardEntryKind::Image { bytes, .. } if bytes == &snap.bytes)
        }) {
            let mut existing = self.entries.remove(pos);
            if self.is_popup_visible()
                && let ClipboardEntryKind::Image {
                    bytes,
                    thumb,
                    width_px,
                    height_px,
                    ..
                } = &mut existing.kind
                && thumb.is_none_or(|a| ctx.image_retention(a).is_none())
                && let Ok(asset) = ctx.load_image_bytes(bytes)
            {
                *width_px = asset.width;
                *height_px = asset.height;
                *thumb = Some(asset);
            }
            self.insert_sorted_top(existing);
            return;
        }

        let mut thumb = ctx.load_image_bytes(&snap.bytes).ok();
        let (width_px, height_px) = thumb.map_or((0, 0), |a| (a.width, a.height));
        // If the popup is closed, release the decoded RGBA8 buffer immediately; compressed bytes
        // stay in RAM and `ensure_thumbs_loaded` re-decodes on popup open.
        if !self.is_popup_visible()
            && let Some(asset) = thumb.take()
            && keep_asset != Some(asset.id)
        {
            ctx.unload_image(asset);
        }
        let entry = ClipboardEntry {
            id: self.alloc_id(),
            pinned: false,
            kind: ClipboardEntryKind::Image {
                mime,
                bytes: snap.bytes,
                thumb,
                width_px,
                height_px,
            },
        };
        self.insert_sorted_top(entry);
        self.enforce_limits(ctx, keep_asset);
    }

    /// Re-publishes entry `id` to the Wayland clipboard and bubbles it to the top of its section.
    pub fn copy_entry(
        &mut self,
        id: u64,
        ctx: &mut EffectCtx<'_, crate::app::msg::Msg>,
    ) -> bool {
        let Some(pos) = self.entries.iter().position(|e| e.id == id) else {
            return false;
        };
        let res = match &self.entries[pos].kind {
            ClipboardEntryKind::Text { content, .. } => ctx.set_clipboard_text(content.clone()),
            ClipboardEntryKind::Image { mime, bytes, .. } => ctx.set_clipboard(
                ClipboardSelection::Clipboard,
                vec![(mime.clone(), bytes.clone())],
            ),
        };
        let entry = self.entries.remove(pos);
        self.insert_sorted_top(entry);
        res.is_ok()
    }

    /// Opens entry `id` via `xdg-open` (writes in-memory images to a temporary runtime file first).
    pub fn open_entry(&self, id: u64) -> bool {
        let Some(entry) = self.entries.iter().find(|e| e.id == id) else {
            return false;
        };
        match &entry.kind {
            ClipboardEntryKind::Image { mime, bytes, .. } => {
                let ext = match mime.as_str() {
                    "image/jpeg" | "image/jpg" => "jpg",
                    "image/webp" => "webp",
                    "image/gif" => "gif",
                    _ => "png",
                };
                let path = temp_open_image_path(&format!("clipboard-{id}"), ext);
                if std::fs::write(&path, bytes).is_err() {
                    return false;
                }
                spawn_detached("xdg-open", [path.as_os_str()]).is_ok()
            }
            ClipboardEntryKind::Text { content, .. } => {
                let trimmed = content.trim();
                if !is_openable_target(trimmed) {
                    return false;
                }
                spawn_detached("xdg-open", [trimmed]).is_ok()
            }
        }
    }

    pub fn toggle_pin(&mut self, id: u64) -> bool {
        let Some(pos) = self.entries.iter().position(|e| e.id == id) else {
            return false;
        };
        let mut entry = self.entries.remove(pos);
        entry.pinned = !entry.pinned;
        self.insert_sorted_top(entry);
        true
    }

    pub fn delete_entry(
        &mut self,
        id: u64,
        ctx: &mut EffectCtx<'_, crate::app::msg::Msg>,
        keep_asset: Option<AssetId>,
    ) -> bool {
        let Some(pos) = self.entries.iter().position(|e| e.id == id) else {
            return false;
        };
        let removed = self.entries.remove(pos);
        unload_entry_asset(&removed, ctx, keep_asset);
        if self.scroll_index >= self.entries.len() && self.scroll_index > 0 {
            self.scroll_index = self.entries.len().saturating_sub(1);
        }
        true
    }

    /// Removes all unpinned clipboard entries, unloading image assets and shrinking heap capacity.
    pub fn clear_unpinned(
        &mut self,
        ctx: &mut EffectCtx<'_, crate::app::msg::Msg>,
        keep_asset: Option<AssetId>,
    ) -> bool {
        let before = self.entries.len();
        let mut kept = Vec::with_capacity(self.entries.len());
        let mut unloaded_any = false;
        for entry in self.entries.drain(..) {
            if entry.pinned {
                kept.push(entry);
            } else if unload_entry_asset(&entry, ctx, keep_asset) {
                unloaded_any = true;
            }
        }
        kept.shrink_to_fit();
        self.entries = kept;
        self.scroll_index = 0;
        if unloaded_any {
            ctx.trim_memory();
        }
        self.entries.len() != before
    }

    fn alloc_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        id
    }

    /// Inserts pinned entries at index 0 and unpinned entries immediately after all pinned entries.
    fn insert_sorted_top(&mut self, entry: ClipboardEntry) {
        if entry.pinned {
            self.entries.insert(0, entry);
        } else {
            let idx = self
                .entries
                .iter()
                .position(|e| !e.pinned)
                .unwrap_or(self.entries.len());
            self.entries.insert(idx, entry);
        }
    }

    fn enforce_limits(
        &mut self,
        ctx: &mut EffectCtx<'_, crate::app::msg::Msg>,
        keep_asset: Option<AssetId>,
    ) {
        // Cap image entries first (oldest unpinned images evicted first).
        let mut image_count = self
            .entries
            .iter()
            .filter(|e| matches!(e.kind, ClipboardEntryKind::Image { .. }))
            .count();
        while image_count > MAX_CLIPBOARD_IMAGES {
            let Some(pos) = self.entries.iter().rposition(|e| {
                !e.pinned && matches!(e.kind, ClipboardEntryKind::Image { .. })
            }) else {
                break;
            };
            let removed = self.entries.remove(pos);
            unload_entry_asset(&removed, ctx, keep_asset);
            image_count -= 1;
        }

        // Cap total entries (oldest unpinned entries evicted first).
        while self.entries.len() > MAX_CLIPBOARD_ENTRIES {
            let Some(pos) = self.entries.iter().rposition(|e| !e.pinned) else {
                break;
            };
            let removed = self.entries.remove(pos);
            unload_entry_asset(&removed, ctx, keep_asset);
        }
    }
}

fn unload_entry_asset(
    entry: &ClipboardEntry,
    ctx: &mut EffectCtx<'_, crate::app::msg::Msg>,
    keep_asset: Option<AssetId>,
) -> bool {
    if let ClipboardEntryKind::Image {
        thumb: Some(asset), ..
    } = entry.kind
        && keep_asset != Some(asset.id)
    {
        ctx.unload_image(asset);
        true
    } else {
        false
    }
}

/// Parses `#rrggbb` or `#rgb` hex color strings into a [`Color`].
#[must_use]
pub fn parse_hex_color(s: &str) -> Option<Color> {
    let hex = s.strip_prefix('#')?;
    if !hex.is_ascii() {
        return None;
    }
    match hex.len() {
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(Color::rgb(r, g, b))
        }
        3 => {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
            Some(Color::rgb(r, g, b))
        }
        _ => None,
    }
}

/// Probes whether any V4L2 video capture device (`/dev/video*`) exists on this host so the
/// Camera privacy slot is capability-gated on hosts without a webcam.
#[must_use]
pub fn probe_has_camera_hardware() -> bool {
    let Ok(entries) = std::fs::read_dir("/dev") else {
        return false;
    };
    entries.flatten().any(|e| {
        e.file_name()
            .to_str()
            .is_some_and(|n| n.starts_with("video") && n[5..].chars().all(|c| c.is_ascii_digit()))
    })
}
