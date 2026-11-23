//! Audio input handlers: mixer popup toggle, master volume, and per-app streams.

mod icons;

use waywi::prelude::*;

use crate::app::state::{AudioControlTarget, SoundCue, State};
use crate::theme::{MIXER_TRACK_WIDTH, SEP_WIDTH};
use crate::widgets::mixer_popup::routing::{
    BALANCE_TRACK_W, routing_content_height, routing_view_height,
};

use super::events::scroll_direction_y;
use icons::refresh_stream_icons;

/// Per-app scroll step, matched to the master track and bar module step.
const VOLUME_SCROLL_STEP: f32 = 0.01;
const BALANCE_SCROLL_STEP: f32 = 0.05;
const ROUTING_SCROLL_STEP_PX: f32 = 24.0;

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

pub fn handle_mixer_set_target(
    state: &mut State,
    target: AudioControlTarget,
    ctx: &EffectCtx<'_>,
) {
    if state.audio_target != target {
        state.audio_target = target;
        state.audio_ui.profiles_open = false;
        state.audio_ui.routing_scroll_y = 0.0;
        state.play_sound(SoundCue::TabChanged, ctx);
    }
}

pub fn handle_mixer_toggle_routing(state: &mut State, ctx: &EffectCtx<'_>) {
    state.audio_ui.routing_open = !state.audio_ui.routing_open;
    state.audio_ui.profiles_open = false;
    state.audio_ui.routing_scroll_y = 0.0;
    let cue = if state.audio_ui.routing_open {
        SoundCue::MenuPopup
    } else {
        SoundCue::MenuPopdown
    };
    state.play_sound(cue, ctx);
}

pub fn handle_mixer_toggle_profiles(state: &mut State, ctx: &EffectCtx<'_>) {
    state.audio_ui.profiles_open = !state.audio_ui.profiles_open;
    clamp_routing_scroll(state);
    let cue = if state.audio_ui.profiles_open {
        SoundCue::MenuPopup
    } else {
        SoundCue::MenuPopdown
    };
    state.play_sound(cue, ctx);
}

pub fn handle_mixer_routing_scroll(state: &mut State, sc: ScrollEvent) {
    let step = scroll_direction_y(sc);
    if step == 0 {
        return;
    }
    let max_scroll = (routing_content_height(state) - routing_view_height(state)).max(0.0);
    state.audio_ui.routing_scroll_y = (state.audio_ui.routing_scroll_y
        + (step as f32) * ROUTING_SCROLL_STEP_PX)
        .clamp(0.0, max_scroll);
}

pub fn handle_mixer_select_device(state: &mut State, id: u32, ctx: &EffectCtx<'_>) {
    if let Ok(ac) = ctx.audio() {
        match state.audio_target {
            AudioControlTarget::Sink => {
                let _ = ac.select_device(id);
            }
            AudioControlTarget::Source => {
                let _ = ac.select_source(id);
            }
        }
    }
    if let Some(audio) = &mut state.audio {
        match state.audio_target {
            AudioControlTarget::Sink => {
                if let Some(dev) = audio.devices.iter().find(|d| d.id == id).cloned() {
                    audio.default_sink = Some(dev);
                }
            }
            AudioControlTarget::Source => {
                if let Some(src) = audio.sources.iter().find(|d| d.id == id).cloned() {
                    audio.default_source = Some(src);
                }
            }
        }
    }
    state.audio_ui.profiles_open = false;
    clamp_routing_scroll(state);
    state.play_sound(SoundCue::TabChanged, ctx);
}

pub fn handle_mixer_select_port(state: &mut State, port_name: String, ctx: &EffectCtx<'_>) {
    if let Ok(ac) = ctx.audio() {
        match state.audio_target {
            AudioControlTarget::Sink => {
                let _ = ac.select_port(&port_name);
            }
            AudioControlTarget::Source => {
                let _ = ac.select_source_port(&port_name);
            }
        }
    }
    if let Some(audio) = &mut state.audio {
        match state.audio_target {
            AudioControlTarget::Sink => {
                if let Some(sink) = &mut audio.default_sink {
                    sink.active_port = Some(port_name.clone());
                    let sink_id = sink.id;
                    if let Some(dev) = audio.devices.iter_mut().find(|d| d.id == sink_id) {
                        dev.active_port = Some(port_name);
                    }
                }
            }
            AudioControlTarget::Source => {
                if let Some(src) = &mut audio.default_source {
                    src.active_port = Some(port_name.clone());
                    let src_id = src.id;
                    if let Some(dev) = audio.sources.iter_mut().find(|d| d.id == src_id) {
                        dev.active_port = Some(port_name);
                    }
                }
            }
        }
    }
    state.play_sound(SoundCue::TabChanged, ctx);
}

pub fn handle_mixer_select_profile(
    state: &mut State,
    card_id: u32,
    profile_name: String,
    ctx: &EffectCtx<'_>,
) {
    if let Ok(ac) = ctx.audio() {
        let _ = ac.select_card_profile(card_id, &profile_name);
    }
    if let Some(audio) = &mut state.audio
        && let Some(card) = audio.cards.iter_mut().find(|c| c.id == card_id)
    {
        card.active_profile = Some(profile_name);
    }
    state.audio_ui.profiles_open = false;
    clamp_routing_scroll(state);
    state.play_sound(SoundCue::TabChanged, ctx);
}

pub fn handle_mixer_balance_seek(state: &mut State, ev: PointerEvent, ctx: &EffectCtx<'_>) {
    match ev.button {
        Some(MouseButton::Right | MouseButton::Middle) => {
            apply_target_balance(state, 0.0, ctx);
        }
        Some(MouseButton::Left) => {
            let ratio = (ev.local_position.x.get() / BALANCE_TRACK_W).clamp(0.0, 1.0);
            let raw = ratio * 2.0 - 1.0;
            let snapped = if raw.abs() < 0.05 { 0.0 } else { raw };
            apply_target_balance(state, snapped, ctx);
        }
        _ => {}
    }
}

pub fn handle_mixer_balance_scroll(state: &mut State, sc: ScrollEvent, ctx: &EffectCtx<'_>) {
    let step = -scroll_direction_y(sc);
    if step == 0 {
        return;
    }
    let current = match (state.audio.as_ref(), state.audio_target) {
        (Some(a), AudioControlTarget::Sink) => a.balance(),
        (Some(a), AudioControlTarget::Source) => a.source_balance(),
        (None, _) => 0.0,
    };
    let raw = (current + (step as f32) * BALANCE_SCROLL_STEP).clamp(-1.0, 1.0);
    let snapped = if raw.abs() < 0.025 { 0.0 } else { raw };
    apply_target_balance(state, snapped, ctx);
}

fn apply_target_balance(state: &mut State, balance: f32, ctx: &EffectCtx<'_>) {
    let bal = balance.clamp(-1.0, 1.0);
    if let Ok(ac) = ctx.audio() {
        match state.audio_target {
            AudioControlTarget::Sink => {
                let _ = ac.set_balance(bal);
            }
            AudioControlTarget::Source => {
                let _ = ac.set_source_balance(bal);
            }
        }
    }
    if let Some(audio) = &mut state.audio {
        match state.audio_target {
            AudioControlTarget::Sink => {
                let peak = audio.linear.max(0.01);
                update_optimistic_channels(&mut audio.channels, &audio.channel_map, peak, bal);
            }
            AudioControlTarget::Source => {
                let peak = audio.source_linear.max(0.01);
                update_optimistic_channels(
                    &mut audio.source_channels,
                    &audio.source_channel_map,
                    peak,
                    bal,
                );
            }
        }
    }
    state.play_sound(SoundCue::VolumeChange, ctx);
}

fn update_optimistic_channels(
    channels: &mut Vec<f32>,
    channel_map: &[AudioChannelPosition],
    peak: f32,
    balance: f32,
) {
    let left_gain = if balance > 0.0 { 1.0 - balance } else { 1.0 };
    let right_gain = if balance < 0.0 { 1.0 + balance } else { 1.0 };
    if channels.len() < 2 {
        *channels = vec![peak * left_gain, peak * right_gain];
        return;
    }
    if channel_map.len() == channels.len() {
        for (vol, &pos) in channels.iter_mut().zip(channel_map.iter()) {
            if pos.is_left() {
                *vol = peak * left_gain;
            } else if pos.is_right() {
                *vol = peak * right_gain;
            }
        }
    } else {
        channels[0] = peak * left_gain;
        channels[1] = peak * right_gain;
    }
}

fn clamp_routing_scroll(state: &mut State) {
    let max_scroll = (routing_content_height(state) - routing_view_height(state)).max(0.0);
    state.audio_ui.routing_scroll_y = state.audio_ui.routing_scroll_y.clamp(0.0, max_scroll);
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
