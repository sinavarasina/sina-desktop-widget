//! Lazy notification icon resolution from XDG icon names, file paths, or cached/raw pixel hints.

use std::path::Path;

use crate::app::asset_lru::PathAssetCache;
use waywi::desktop::icons::XdgIconLookup;
use waywi::prelude::*;
use waywi::dbus::NotificationImageData;

use super::storage::load_cached_icon;

const NOTIF_ICON_SOURCE_PX: u16 = 32;

#[allow(clippy::too_many_arguments)]
pub fn resolve_notification_icon(
    app_icon: &str,
    desktop_entry: Option<&str>,
    image_path: Option<&str>,
    cached_icon_path: Option<&str>,
    image_data: Option<&NotificationImageData>,
    icon_lookup: &mut XdgIconLookup,
    path_cache: &mut PathAssetCache,
    ctx: &mut EffectCtx<'_, crate::app::msg::Msg>,
) -> Option<ImageAsset> {
    if let Some(raw) = image_data
        && raw.width > 0
        && raw.height > 0
        && raw.bits_per_sample == 8
        && let (Ok(w), Ok(h)) = (u32::try_from(raw.width), u32::try_from(raw.height))
    {
        let stride = usize::try_from(raw.rowstride).unwrap_or((w as usize) * 4);
        if let Ok(asset) = ctx.load_rgba_image(w, h, stride, raw.has_alpha, &raw.data) {
            return Some(asset);
        }
    }

    if let Some(cached_path) = cached_icon_path
        && !cached_path.is_empty()
    {
        let sig = format!("notif-cached:{cached_path}");
        if let Some(&cached) = path_cache.get(&sig) {
            return Some(cached);
        }
        if let Some(asset) = load_cached_icon(Path::new(cached_path), ctx) {
            path_cache.insert(sig, asset);
            return Some(asset);
        }
    }

    if let Some(path_str) = image_path
        && !path_str.is_empty()
    {
        let normalized = path_str.strip_prefix("file://").unwrap_or(path_str);
        let sig = format!("notif-path:{normalized}");
        if let Some(&cached) = path_cache.get(&sig) {
            return Some(cached);
        }
        if Path::new(normalized).exists()
            && let Ok(asset) =
                ctx.load_image_sized(Path::new(normalized), u32::from(NOTIF_ICON_SOURCE_PX))
        {
            path_cache.insert(sig, asset);
            return Some(asset);
        }
    }

    for candidate in [Some(app_icon), desktop_entry].into_iter().flatten() {
        if candidate.is_empty() {
            continue;
        }
        let sig = format!("notif-name:{candidate}");
        if let Some(&cached) = path_cache.get(&sig) {
            return Some(cached);
        }
        if Path::new(candidate).is_absolute()
            && Path::new(candidate).exists()
            && let Ok(asset) =
                ctx.load_image_sized(Path::new(candidate), u32::from(NOTIF_ICON_SOURCE_PX))
        {
            path_cache.insert(sig, asset);
            return Some(asset);
        }
        if let Some(resolved) = icon_lookup.resolve(candidate, NOTIF_ICON_SOURCE_PX)
            && let Ok(asset) = ctx.load_image_sized(&resolved, u32::from(NOTIF_ICON_SOURCE_PX))
        {
            path_cache.insert(sig, asset);
            return Some(asset);
        }
    }

    None
}
