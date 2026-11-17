//! Audio input handlers: mixer popup toggle, master volume, and per-app streams.

use std::collections::HashMap;
use std::path::Path;

use waywi::prelude::*;

use crate::app::state::{AudioControlTarget, State};
use crate::theme::{MIXER_TRACK_WIDTH, SEP_WIDTH};

use super::events::scroll_direction_y;

/// Per-app scroll step, matched to the master track and bar module step.
const VOLUME_SCROLL_STEP: f32 = 0.01;

pub fn handle_mixer_bar_pointer(state: &mut State, ev: PointerEvent, ctx: &EffectCtx<'_>) {
    match ev.button {
        Some(MouseButton::Left) => {
            let module_left = ev.position.x.get() - ev.local_position.x.get();
            state.audio_ui.popup_left_x = (module_left + SEP_WIDTH).max(8.0);
            state.audio_ui.toggle_popup();
        }
        Some(MouseButton::Right) => toggle_target_mute(state, ctx),
        _ => {}
    }
}

pub fn handle_mixer_set_target(state: &mut State, target: AudioControlTarget) {
    state.audio_target = target;
}

pub fn handle_mixer_master_seek_click(state: &mut State, ev: PointerEvent, ctx: &EffectCtx<'_>) {
    match ev.button {
        Some(MouseButton::Left) => {
            let ratio = (ev.local_position.x.get() / MIXER_TRACK_WIDTH).clamp(0.0, 1.0);
            set_master_volume(state, ratio, ctx);
        }
        Some(MouseButton::Right) => toggle_target_mute(state, ctx),
        _ => {}
    }
}

pub fn handle_mixer_stream_scroll(
    state: &mut State,
    id: u32,
    sc: ScrollEvent,
    ctx: &EffectCtx<'_>,
) {
    let step = -scroll_direction_y(sc);
    if step == 0 {
        return;
    }
    let Some(current) = stream_linear(state, id) else {
        return;
    };
    let next = adjust_step(current, step);
    if let Ok(ac) = ctx.audio() {
        let _ = ac.set_stream_volume_linear(id, next);
    }
    if let Some(audio) = &mut state.audio
        && let Some(s) = audio.streams.iter_mut().find(|s| s.id == id)
    {
        s.linear = next;
    }
}

pub fn handle_mixer_stream_seek_click(
    state: &mut State,
    id: u32,
    ev: PointerEvent,
    ctx: &EffectCtx<'_>,
) {
    if ev.button != Some(MouseButton::Left) {
        return;
    }
    let ratio = (ev.local_position.x.get() / MIXER_TRACK_WIDTH).clamp(0.0, 1.0);
    if let Ok(ac) = ctx.audio() {
        let _ = ac.set_stream_volume_linear(id, ratio);
    }
    if let Some(audio) = &mut state.audio
        && let Some(s) = audio.streams.iter_mut().find(|s| s.id == id)
    {
        s.linear = ratio;
    }
}

pub fn handle_mixer_capture_scroll(
    state: &mut State,
    id: u32,
    sc: ScrollEvent,
    ctx: &EffectCtx<'_>,
) {
    let step = -scroll_direction_y(sc);
    if step == 0 {
        return;
    }
    let Some(current) = capture_linear(state, id) else {
        return;
    };
    let next = adjust_step(current, step);
    if let Ok(ac) = ctx.audio() {
        let _ = ac.set_capture_volume_linear(id, next);
    }
    if let Some(audio) = &mut state.audio
        && let Some(c) = audio.captures.iter_mut().find(|c| c.id == id)
    {
        c.linear = next;
    }
}

pub fn handle_mixer_capture_seek_click(
    state: &mut State,
    id: u32,
    ev: PointerEvent,
    ctx: &EffectCtx<'_>,
) {
    if ev.button != Some(MouseButton::Left) {
        return;
    }
    let ratio = (ev.local_position.x.get() / MIXER_TRACK_WIDTH).clamp(0.0, 1.0);
    if let Ok(ac) = ctx.audio() {
        let _ = ac.set_capture_volume_linear(id, ratio);
    }
    if let Some(audio) = &mut state.audio
        && let Some(c) = audio.captures.iter_mut().find(|c| c.id == id)
    {
        c.linear = ratio;
    }
}

pub fn handle_audio_updated(state: &mut State, snap: AudioSnapshot, ctx: &mut EffectCtx<'_>) {
    state.audio = Some(snap);
    refresh_stream_icons(state, ctx);
}

fn set_master_volume(state: &mut State, level: f32, ctx: &EffectCtx<'_>) {
    if let Ok(ac) = ctx.audio() {
        match state.audio_target {
            AudioControlTarget::Sink => {
                let _ = ac.set_volume_linear(level);
            }
            AudioControlTarget::Source => {
                let _ = ac.set_source_volume_linear(level);
            }
        }
    }
    if let Some(audio) = &mut state.audio {
        match state.audio_target {
            AudioControlTarget::Sink => audio.linear = level,
            AudioControlTarget::Source => audio.source_linear = level,
        }
    }
}

fn toggle_target_mute(state: &mut State, ctx: &EffectCtx<'_>) {
    if let Ok(ac) = ctx.audio() {
        match state.audio_target {
            AudioControlTarget::Sink => {
                let _ = ac.toggle_mute();
            }
            AudioControlTarget::Source => {
                let _ = ac.toggle_source_mute();
            }
        }
    }
    if let Some(audio) = &mut state.audio {
        match state.audio_target {
            AudioControlTarget::Sink => audio.muted = !audio.muted,
            AudioControlTarget::Source => audio.source_muted = !audio.source_muted,
        }
    }
}

fn stream_linear(state: &State, id: u32) -> Option<f32> {
    state
        .audio
        .as_ref()
        .and_then(|a| a.streams.iter().find(|s| s.id == id))
        .map(|s| s.linear)
}

fn capture_linear(state: &State, id: u32) -> Option<f32> {
    state
        .audio
        .as_ref()
        .and_then(|a| a.captures.iter().find(|c| c.id == id))
        .map(|c| c.linear)
}

fn adjust_step(current: f32, step: i32) -> f32 {
    (current + (step as f32) * VOLUME_SCROLL_STEP).clamp(0.0, 1.5)
}

/// Resolves desktop icons for every playback/capture stream, keyed by id.
/// Falls back to the binary name, then the stream name (case-insensitively), then the generic
/// executable glyph.
fn refresh_stream_icons(state: &mut State, ctx: &mut EffectCtx<'_>) {
    let Some(audio) = state.audio.clone() else {
        return;
    };

    let mut streams = HashMap::new();
    for s in &audio.streams {
        let icon = resolve_app_icon(state, s.binary.as_deref(), &s.name, ctx);
        streams.insert(s.id, icon);
    }
    state.audio_ui.stream_icons = streams;

    let mut captures = HashMap::new();
    for c in &audio.captures {
        let icon = resolve_app_icon(state, c.binary.as_deref(), &c.name, ctx);
        captures.insert(c.id, icon);
    }
    state.audio_ui.capture_icons = captures;
}

/// Plain waywi path: XDG icon theme lookup over `[binary, stream name]` case variants, falling
/// back to the generic executable glyph; resolved files go through the path-asset cache.
fn resolve_app_icon(
    state: &mut State,
    binary: Option<&str>,
    stream_name: &str,
    ctx: &mut EffectCtx<'_>,
) -> Option<ImageAsset> {
    const GENERIC: &str = "application-x-executable";

    let mut candidates: Vec<String> = Vec::new();
    for candidate in [binary, Some(stream_name)] {
        let Some(c) = candidate.map(str::trim).filter(|s| !s.is_empty()) else {
            continue;
        };
        if !candidates.iter().any(|seen| seen == c) {
            candidates.push(c.to_owned());
        }
        // Icon names are case-sensitive; `Firefox` must not hide a `firefox.png`.
        let lower = c.to_lowercase();
        if lower != c && !candidates.contains(&lower) {
            candidates.push(lower);
        }
    }
    candidates.push(GENERIC.to_owned());

    for candidate in &candidates {
        if let Some(path) = state.icon_lookup.resolve(candidate, 24)
            && let Some(asset) = load_app_icon_asset(state, &path, ctx)
        {
            return Some(asset);
        }
    }
    None
}

/// Loads a resolved icon path through the path-asset cache keyed by its signature.
fn load_app_icon_asset(
    state: &mut State,
    path: &Path,
    ctx: &mut EffectCtx<'_>,
) -> Option<ImageAsset> {
    let sig = format!("audio-app:{}", path.display());
    if let Some(&cached) = state.path_asset_cache.get(&sig) {
        return Some(cached);
    }
    if let Ok(asset) = ctx.load_image_sized(path, 24) {
        state.path_asset_cache.insert(sig, asset);
        return Some(asset);
    }
    None
}
