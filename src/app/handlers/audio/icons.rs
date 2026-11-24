//! Per-application audio stream icon resolution (`application.icon-name` / `application.id` ->
//! `.desktop` entry lookup -> binary basename / stream name -> generic fallback).

use std::collections::HashMap;
use std::path::Path;

use waywi::prelude::*;

use crate::app::state::State;

const AUDIO_ICON_PX: u16 = 24;
const GENERIC_AUDIO_ICON: &str = "application-x-executable";

struct StreamIconQuery {
    id: u32,
    name: String,
    binary: Option<String>,
    app_name: Option<String>,
}

/// Resolves desktop icons for every playback/capture stream, keyed by stream id.
pub(super) fn refresh_stream_icons(state: &mut State, ctx: &mut EffectCtx<'_, crate::app::msg::Msg>) {
    let Some(audio) = state.audio.as_ref() else {
        return;
    };

    let query = |s: &AudioStream| StreamIconQuery {
        id: s.id,
        name: s.name.clone(),
        binary: s.binary_name.clone(),
        app_name: s.app_name.clone(),
    };

    let playback_queries: Vec<StreamIconQuery> = audio
        .playback_streams()
        .map(&query)
        .collect();

    let capture_queries: Vec<StreamIconQuery> = audio
        .capture_streams()
        .map(&query)
        .collect();

    let mut streams = HashMap::with_capacity(playback_queries.len());
    for q in &playback_queries {
        streams.insert(q.id, resolve_stream_icon(state, q, ctx));
    }
    state.audio_ui.stream_icons = streams;

    let mut captures = HashMap::with_capacity(capture_queries.len());
    for q in &capture_queries {
        captures.insert(q.id, resolve_stream_icon(state, q, ctx));
    }
    state.audio_ui.capture_icons = captures;
}

fn resolve_stream_icon(
    state: &mut State,
    q: &StreamIconQuery,
    ctx: &mut EffectCtx<'_, crate::app::msg::Msg>,
) -> Option<ImageAsset> {
    // 1. Match against `.desktop` entries (sharing the cached `drun` scan) before falling back to
    //    raw theme lookups so wrappers (`firefox-bin`, `electron`, `ALSA plug-in [mpv]`) pick up
    //    their real `Icon=` value.
    let bin_base = q.binary.as_deref().and_then(binary_basename);
    let bracket_target = extract_bracket_target(&q.name);

    state.drun.ensure_scanned();
    if let Some(desktop_icon) = find_desktop_icon(
        &state.drun.entries,
        q.app_name.as_deref(),
        bin_base,
        &q.name,
        bracket_target,
    ) {
        let mut icon_names = Vec::new();
        push_case_variants(&mut icon_names, &desktop_icon);
        if let Some(asset) = resolve_from_candidates(state, &icon_names, ctx) {
            return Some(asset);
        }
    }

    // 2. Fall back to XDG icon theme lookup over binary basename, bracketed target, application
    //    name, stream name, and finally the generic executable icon.
    let mut fallbacks = Vec::new();
    if let Some(base) = bin_base {
        push_case_variants(&mut fallbacks, base);
    }
    if let Some(target) = bracket_target {
        push_case_variants(&mut fallbacks, target);
    }
    if let Some(app_name) = q.app_name.as_deref() {
        push_case_variants(&mut fallbacks, app_name);
    }
    push_case_variants(&mut fallbacks, &q.name);
    fallbacks.push(GENERIC_AUDIO_ICON.to_owned());

    resolve_from_candidates(state, &fallbacks, ctx)
}

fn resolve_from_candidates(
    state: &mut State,
    candidates: &[String],
    ctx: &mut EffectCtx<'_, crate::app::msg::Msg>,
) -> Option<ImageAsset> {
    for candidate in candidates {
        if let Some(path) = state.icon_lookup.resolve(candidate, AUDIO_ICON_PX)
            && let Some(asset) = load_app_icon_asset(state, &path, ctx)
        {
            return Some(asset);
        }
    }
    None
}

fn push_case_variants(out: &mut Vec<String>, raw: &str) {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return;
    }
    if !out.iter().any(|seen| seen == trimmed) {
        out.push(trimmed.to_owned());
    }
    let lower = trimmed.to_lowercase();
    if lower != trimmed && !out.iter().any(|seen| seen == &lower) {
        out.push(lower);
    }
}

/// Extracts only the executable file name so absolute binary paths like `/usr/lib/.../firefox-bin`
/// do not trigger `XdgIconLookup`'s file-path fast path on the ELF binary itself.
fn binary_basename(raw: &str) -> Option<&str> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.contains('/') {
        Path::new(trimmed)
            .file_name()
            .and_then(|s| s.to_str())
            .filter(|s| !s.is_empty())
    } else {
        Some(trimmed)
    }
}

/// Extracts `<inner>` from wrapper stream titles such as `"ALSA plug-in [mpv]"`.
fn extract_bracket_target(name: &str) -> Option<&str> {
    let (_, rest) = name.split_once('[')?;
    let (inner, _) = rest.split_once(']')?;
    let trimmed = inner.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}

fn find_desktop_icon(
    entries: &[DesktopEntry],
    app_id: Option<&str>,
    bin_base: Option<&str>,
    stream_name: &str,
    bracket_target: Option<&str>,
) -> Option<String> {
    let mut keys: Vec<String> = Vec::new();
    for raw in [app_id, bin_base, bracket_target, Some(stream_name)]
        .into_iter()
        .flatten()
    {
        let clean = raw.trim().strip_suffix(".desktop").unwrap_or(raw.trim());
        if !clean.is_empty() && !keys.iter().any(|k| k.eq_ignore_ascii_case(clean)) {
            keys.push(clean.to_ascii_lowercase());
        }
    }

    for key in &keys {
        // Prefer exact desktop ID or executable basename match, then StartupWMClass, then Name.
        if let Some(icon) = entries.iter().find_map(|entry| {
            let icon = entry
                .icon
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())?;
            let id_base = entry.id.strip_suffix(".desktop").unwrap_or(&entry.id);
            let id_hit = id_base.eq_ignore_ascii_case(key);
            let exec_hit = desktop_exec_key(entry).is_some_and(|ex| ex.eq_ignore_ascii_case(key))
                || entry
                    .try_exec
                    .as_deref()
                    .and_then(binary_basename)
                    .is_some_and(|tx| tx.eq_ignore_ascii_case(key));
            (id_hit || exec_hit).then(|| icon.to_owned())
        }) {
            return Some(icon);
        }

        if let Some(icon) = entries.iter().find_map(|entry| {
            let icon = entry
                .icon
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())?;
            let wm_hit = entry
                .startup_wm_class
                .as_deref()
                .is_some_and(|wm| wm.trim().eq_ignore_ascii_case(key));
            let name_hit = entry.name.trim().eq_ignore_ascii_case(key);
            (wm_hit || name_hit).then(|| icon.to_owned())
        }) {
            return Some(icon);
        }
    }
    None
}

fn desktop_exec_key(entry: &DesktopEntry) -> Option<&str> {
    let raw = entry.exec.as_deref()?.trim();
    if raw.is_empty() {
        return None;
    }
    let mut iter = raw.split_whitespace();
    let mut cmd = iter.next()?;
    let cmd_name = binary_basename(cmd)?;
    if cmd_name == "env" {
        for tok in iter.by_ref() {
            if tok.contains('=') && !tok.starts_with('-') {
                continue;
            }
            cmd = tok;
            break;
        }
    }
    let resolved = binary_basename(cmd)?;
    if resolved == "flatpak" {
        let mut after_run = false;
        for tok in iter {
            if tok == "run" {
                after_run = true;
                continue;
            }
            if after_run && !tok.starts_with('-') && !tok.starts_with('%') {
                return Some(tok);
            }
        }
    }
    Some(resolved)
}

/// Loads a resolved icon path through the path-asset cache keyed by its signature.
fn load_app_icon_asset(
    state: &mut State,
    path: &Path,
    ctx: &mut EffectCtx<'_, crate::app::msg::Msg>,
) -> Option<ImageAsset> {
    let sig = format!("audio-app:{}", path.display());
    if let Some(&cached) = state.path_asset_cache.get(&sig) {
        return Some(cached);
    }
    if let Ok(asset) = ctx.load_image_sized(path, u32::from(AUDIO_ICON_PX)) {
        state.path_asset_cache.insert(sig, asset);
        return Some(asset);
    }
    None
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use waywi::prelude::DesktopEntry;

    use super::{binary_basename, desktop_exec_key, extract_bracket_target, find_desktop_icon};

    fn entry(
        id: &str,
        name: &str,
        exec: Option<&str>,
        icon: Option<&str>,
        wm_class: Option<&str>,
    ) -> DesktopEntry {
        DesktopEntry {
            id: id.into(),
            path: PathBuf::from(format!("/usr/share/applications/{id}")),
            name: name.into(),
            generic_name: None,
            comment: None,
            exec: exec.map(str::to_owned),
            try_exec: None,
            working_dir: None,
            icon: icon.map(str::to_owned),
            categories: Vec::new(),
            keywords: Vec::new(),
            terminal: false,
            no_display: false,
            hidden: false,
            only_show_in: Vec::new(),
            not_show_in: Vec::new(),
            startup_wm_class: wm_class.map(str::to_owned),
            dbus_activatable: false,
            actions: Vec::new(),
        }
    }

    #[test]
    fn extracts_binary_basename_and_alsa_bracket_target() {
        assert_eq!(
            binary_basename("/usr/lib/firefox/firefox-bin"),
            Some("firefox-bin")
        );
        assert_eq!(binary_basename("mpv"), Some("mpv"));
        assert_eq!(binary_basename("   "), None);
        assert_eq!(
            extract_bracket_target("ALSA plug-in [wine64-preloader]"),
            Some("wine64-preloader")
        );
        assert_eq!(extract_bracket_target("Firefox"), None);
    }

    #[test]
    fn parses_desktop_exec_with_env_and_flatpak() {
        let env_e = entry(
            "discord.desktop",
            "Discord",
            Some("env GDK_BACKEND=x11 /opt/discord/Discord --start-minimized"),
            Some("discord"),
            None,
        );
        assert_eq!(desktop_exec_key(&env_e), Some("Discord"));

        let fp_e = entry(
            "com.spotify.Client.desktop",
            "Spotify",
            Some("flatpak run --branch=stable com.spotify.Client %U"),
            Some("spotify-client"),
            None,
        );
        assert_eq!(desktop_exec_key(&fp_e), Some("com.spotify.Client"));
    }

    #[test]
    fn finds_desktop_icon_across_wrapper_binaries_and_app_ids() {
        let entries = vec![
            entry(
                "firefox-bin.desktop",
                "Firefox",
                Some("firefox-bin %u"),
                Some("firefox"),
                Some("firefox-bin"),
            ),
            entry(
                "com.valvesoftware.Steam.desktop",
                "Steam",
                Some("steam %U"),
                Some("steam"),
                None,
            ),
        ];

        assert_eq!(
            find_desktop_icon(&entries, None, Some("firefox-bin"), "AudioIPC", None).as_deref(),
            Some("firefox")
        );
        assert_eq!(
            find_desktop_icon(
                &entries,
                Some("com.valvesoftware.Steam"),
                Some("steamwebhelper"),
                "Steam",
                None
            )
            .as_deref(),
            Some("steam")
        );
    }
}
