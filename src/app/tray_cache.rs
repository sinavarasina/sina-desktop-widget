//! System tray entry deduplication and icon asset caching.

use std::collections::HashMap;

use waywi::desktop::icons::XdgIconLookup;
use waywi::prelude::*;

use super::types::TrayEntry;

/// Minimum source raster dimension (2x supersampled for `TRAY_ICON_SIZE`) so both XDG SVGs and
/// StatusNotifier pixmaps downscale cleanly instead of upscaling 16px bitmaps.
const TRAY_ICON_SOURCE_PX: u16 = 32;

pub fn upsert_tray_entry_into(entries: &mut Vec<TrayEntry>, new_entry: TrayEntry) {
    if let Some(pos) = entries
        .iter()
        .position(|e| same_tray_identity(&e.snapshot, &new_entry.snapshot))
    {
        entries[pos] = new_entry;
    } else {
        entries.push(new_entry);
    }
}

fn same_tray_identity(a: &TrayItemSnapshot, b: &TrayItemSnapshot) -> bool {
    if a.service == b.service {
        return true;
    }
    if !a.bus_name.is_empty() && a.bus_name == b.bus_name && a.object_path == b.object_path {
        return true;
    }
    !a.id.is_empty() && a.id == b.id && a.object_path == b.object_path && a.menu_path == b.menu_path
}

/// Resolves a `TrayItemSnapshot` icon into a cached `ImageAsset` without re-decoding unchanged items.
pub fn resolve_tray_icon(
    entry: &TrayItemSnapshot,
    icon_lookup: &mut XdgIconLookup,
    path_cache: &mut HashMap<String, ImageAsset>,
    ctx: &mut EffectCtx<'_>,
) -> (Option<ImageAsset>, String) {
    let icon_name =
        if entry.status == TrayStatus::NeedsAttention && !entry.attention_icon_name.is_empty() {
            entry.attention_icon_name.as_str()
        } else {
            entry.icon_name.as_str()
        };

    if !icon_name.is_empty() {
        let sig = format!("name:{icon_name}");
        if let Some(&cached) = path_cache.get(&sig) {
            return (Some(cached), sig);
        }
        if let Some(path) = icon_lookup.resolve(icon_name, TRAY_ICON_SOURCE_PX)
            && let Ok(asset) = ctx.load_image_sized(&path, u32::from(TRAY_ICON_SOURCE_PX))
        {
            path_cache.insert(sig.clone(), asset);
            return (Some(asset), sig);
        }
    }

    if let Some(pix) =
        TrayIconPixmap::best_for_size(&entry.icon_pixmap, i32::from(TRAY_ICON_SOURCE_PX))
        && let Some(rgba) = pix.to_rgba()
        && let (Ok(w), Ok(h)) = (u32::try_from(pix.width), u32::try_from(pix.height))
        && let Ok(asset) = ctx.load_rgba_image(w, h, (w as usize) * 4, true, &rgba)
    {
        let sig = format!("pix:{}x{}:{}", w, h, rgba.len());
        return (Some(asset), sig);
    }

    (None, String::new())
}
