//! Audio input handlers: mixer popup toggle, master volume, and per-app streams.

mod icons;

use waywi::prelude::*;

use crate::app::state::{AudioControlTarget, SoundCue, State};
use crate::theme::{MIXER_TRACK_WIDTH, SEP_WIDTH};

use super::events::scroll_direction_y;
use icons::refresh_stream_icons;

/// Per-app scroll step, matched to the master track and bar module step.
const VOLUME_SCROLL_STEP: f32 = 0.01;

pub fn handle_mixer_bar_pointer(state: &mut State, ev: PointerEvent, ctx: &EffectCtx<'_>) {
    match ev.button {
        Some(MouseButton::Left) => {
            let module_left = ev.position.x.get() - ev.local_position.x.get();
            state.audio_ui.popup_left_x = (module_left + SEP_WIDTH).max(8.0);
            state.audio_ui.toggle_popup();
            let cue = if state.audio_ui.popup_target {
                SoundCue::MenuPopup
            } else {
                SoundCue::MenuPopdown
            };
            state.play_sound(cue, ctx);
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
    state.play_sound(SoundCue::VolumeChange, ctx);
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
    state.play_sound(SoundCue::VolumeChange, ctx);
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
    state.play_sound(SoundCue::VolumeChange, ctx);
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
    let mut now_muted = false;
    if let Some(audio) = &mut state.audio {
        match state.audio_target {
            AudioControlTarget::Sink => {
                audio.muted = !audio.muted;
                now_muted = audio.muted;
            }
            AudioControlTarget::Source => {
                audio.source_muted = !audio.source_muted;
                now_muted = audio.source_muted;
            }
        }
    }
    let cue = if now_muted {
        SoundCue::ToggleOff
    } else {
        SoundCue::ToggleOn
    };
    state.play_sound(cue, ctx);
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
