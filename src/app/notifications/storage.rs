//! Lazy disk persistence for notifications, cached icon blobs, and DND state across reboots.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use waywi::prelude::{EffectCtx, ImageAsset, NotificationImageData};

pub const MAX_RAM_NOTIFICATIONS: usize = 50;
pub const MAX_DISK_NOTIFICATIONS: usize = 500;
const ICON_CACHE_MAGIC: &[u8; 4] = b"WWIC";
const MAX_CACHED_ICON_DIM: u32 = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredNotification {
    pub id: u32,
    pub app_name: String,
    pub app_icon: String,
    pub desktop_entry: Option<String>,
    pub image_path: Option<String>,
    #[serde(default)]
    pub cached_icon_path: Option<String>,
    pub summary: String,
    pub body: String,
    pub actions: Vec<(String, String)>,
    pub urgency: u8,
    pub received_epoch_secs: u64,
    pub read: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PersistedStore {
    pub dnd: bool,
    pub items: Vec<StoredNotification>,
}

#[must_use]
pub fn default_store_path() -> PathBuf {
    #[cfg(test)]
    {
        return std::env::temp_dir().join(format!(
            "sina-desktop-widget-test-{}.json",
            std::process::id()
        ));
    }
    #[cfg(not(test))]
    {
        if let Ok(state_home) = std::env::var("XDG_STATE_HOME")
            && !state_home.trim().is_empty()
        {
            return PathBuf::from(state_home)
                .join("sina-desktop-widget")
                .join("notifications.json");
        }
        if let Ok(home) = std::env::var("HOME")
            && !home.trim().is_empty()
        {
            return PathBuf::from(home)
                .join(".local")
                .join("state")
                .join("sina-desktop-widget")
                .join("notifications.json");
        }
        PathBuf::from("/tmp/sina-desktop-widget-notifications.json")
    }
}

#[must_use]
pub fn icon_cache_dir(store_path: &Path) -> PathBuf {
    store_path
        .parent()
        .unwrap_or_else(|| Path::new("/tmp"))
        .join("icons")
}

pub fn load_store_from_disk(path: &Path) -> PersistedStore {
    let Ok(bytes) = std::fs::read(path) else {
        return PersistedStore::default();
    };
    serde_json::from_slice::<PersistedStore>(&bytes).unwrap_or_default()
}

pub fn save_store_to_disk(path: &Path, store: &PersistedStore) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_vec(store) {
        let _ = std::fs::write(path, json);
    }
}

pub fn clear_icon_cache(store_path: &Path) {
    let dir = icon_cache_dir(store_path);
    if dir.exists() {
        let _ = std::fs::remove_dir_all(dir);
    }
}

pub fn save_cached_icon(store_path: &Path, raw: &NotificationImageData) -> Option<String> {
    if raw.width <= 0 || raw.height <= 0 || raw.bits_per_sample != 8 {
        return None;
    }
    let src_w = u32::try_from(raw.width).ok()?;
    let src_h = u32::try_from(raw.height).ok()?;
    let channels = usize::try_from(raw.channels).ok().filter(|&c| c == 3 || c == 4)?;
    let src_stride = usize::try_from(raw.rowstride)
        .ok()
        .filter(|&s| s >= (src_w as usize) * channels)?;
    let needed = (src_h as usize)
        .saturating_sub(1)
        .saturating_mul(src_stride)
        .saturating_add((src_w as usize) * channels);
    if raw.data.len() < needed {
        return None;
    }

    // Downscale large notification images to at most 64x64 to keep disk cache tiny.
    let (dst_w, dst_h) = fit_dimensions(src_w, src_h, MAX_CACHED_ICON_DIM);
    let mut pixels = Vec::with_capacity((dst_w as usize) * (dst_h as usize) * 4);
    for y in 0..dst_h {
        let sy = ((u64::from(y) * u64::from(src_h)) / u64::from(dst_h)) as usize;
        for x in 0..dst_w {
            let sx = ((u64::from(x) * u64::from(src_w)) / u64::from(dst_w)) as usize;
            let off = sy * src_stride + sx * channels;
            let r = raw.data[off];
            let g = raw.data[off + 1];
            let b = raw.data[off + 2];
            let a = if raw.has_alpha && channels == 4 {
                raw.data[off + 3]
            } else {
                255
            };
            pixels.extend_from_slice(&[r, g, b, a]);
        }
    }

    let hash = fnv1a_64(&pixels);
    let dir = icon_cache_dir(store_path);
    let _ = std::fs::create_dir_all(&dir);
    let file_path = dir.join(format!("{hash:016x}.rgba"));
    if !file_path.exists() {
        let mut blob = Vec::with_capacity(13 + pixels.len());
        blob.extend_from_slice(ICON_CACHE_MAGIC);
        blob.extend_from_slice(&dst_w.to_le_bytes());
        blob.extend_from_slice(&dst_h.to_le_bytes());
        blob.push(1);
        blob.extend_from_slice(&pixels);
        let _ = std::fs::write(&file_path, blob);
    }
    Some(file_path.to_string_lossy().into_owned())
}

pub fn load_cached_icon(path: &Path, ctx: &mut EffectCtx<'_>) -> Option<ImageAsset> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.len() < 13 || &bytes[0..4] != ICON_CACHE_MAGIC {
        return None;
    }
    let w = u32::from_le_bytes(bytes[4..8].try_into().ok()?);
    let h = u32::from_le_bytes(bytes[8..12].try_into().ok()?);
    let has_alpha = bytes[12] != 0;
    let pixels = &bytes[13..];
    ctx.load_rgba_image(w, h, (w as usize) * 4, has_alpha, pixels)
        .ok()
}

fn fit_dimensions(w: u32, h: u32, max_dim: u32) -> (u32, u32) {
    if w <= max_dim && h <= max_dim {
        return (w.max(1), h.max(1));
    }
    if w >= h {
        let nh = ((u64::from(h) * u64::from(max_dim)) / u64::from(w)).max(1) as u32;
        (max_dim, nh)
    } else {
        let nw = ((u64::from(w) * u64::from(max_dim)) / u64::from(h)).max(1) as u32;
        (nw, max_dim)
    }
}

fn fnv1a_64(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in data {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}
