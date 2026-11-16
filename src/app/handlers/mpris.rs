//! MPRIS D-Bus event and user input handlers.

use waywi::prelude::*;

use crate::app::mpris::MprisControlAction;
use crate::app::state::State;
use crate::theme::{MPRIS_SEEK_TRACK_WIDTH, SEP_WIDTH};

use super::events::scroll_direction_y;

const SEEK_STEP_US: i64 = 5_000_000;

pub fn handle_mpris_event(state: &mut State, ev: MprisEvent, ctx: &mut EffectCtx<'_>) {
    match ev {
        MprisEvent::PlayersUpdated(list) => {
            state
                .mpris
                .set_players(list, &mut state.path_asset_cache, ctx);
        }
        MprisEvent::PlayerChanged(snap) => {
            state
                .mpris
                .upsert_player(*snap, &mut state.path_asset_cache, ctx);
        }
        MprisEvent::PlayerAdded { service_name } => {
            if let Ok(dbus) = ctx.dbus()
                && let Ok(snap) = dbus.mpris_snapshot(&service_name)
            {
                state
                    .mpris
                    .upsert_player(snap, &mut state.path_asset_cache, ctx);
            }
        }
        MprisEvent::PlayerRemoved { service_name } => {
            state
                .mpris
                .remove_player(&service_name, &mut state.path_asset_cache, ctx);
        }
        MprisEvent::Seeked {
            service_name,
            position_us,
        } => {
            state.mpris.apply_seeked(&service_name, position_us);
        }
        _ => {}
    }
}

pub fn handle_mpris_bar_pointer(state: &mut State, ev: PointerEvent, ctx: &EffectCtx<'_>) {
    match ev.button {
        Some(MouseButton::Left) => {
            let module_left = ev.position.x.get() - ev.local_position.x.get();
            state.mpris.popup_left_x = (module_left + SEP_WIDTH).max(8.0);
            state.mpris.toggle_popup();
        }
        Some(MouseButton::Right) => {
            if let Some(svc) = state.mpris.active_player().map(|p| p.service_name.clone()) {
                if let Ok(dbus) = ctx.dbus() {
                    let _ = dbus.mpris_play_pause(&svc);
                }
                if let Some(p) = state.mpris.active_player_mut() {
                    p.status = match p.status {
                        MprisPlaybackStatus::Playing => MprisPlaybackStatus::Paused,
                        _ => MprisPlaybackStatus::Playing,
                    };
                }
            }
        }
        _ => {}
    }
}

pub fn handle_mpris_scroll(state: &mut State, sc: ScrollEvent, ctx: &EffectCtx<'_>) {
    let step = -scroll_direction_y(sc);
    if step == 0 {
        return;
    }
    let Some(svc) = state.mpris.active_player().map(|p| p.service_name.clone()) else {
        return;
    };
    let delta_us = i64::from(step) * SEEK_STEP_US;
    if let Ok(dbus) = ctx.dbus() {
        let _ = dbus.mpris_seek(&svc, delta_us);
    }
    if let Some(p) = state.mpris.active_player_mut() {
        let cur = p.position_us.unwrap_or(0);
        let next = (cur + delta_us).max(0);
        p.position_us = Some(p.length_us.map_or(next, |len| next.min(len.max(0))));
    }
}

pub fn handle_mpris_action(state: &mut State, action: MprisControlAction, ctx: &EffectCtx<'_>) {
    let Some(snap) = state.mpris.active_player().cloned() else {
        return;
    };
    let svc = &snap.service_name;
    match action {
        MprisControlAction::PlayPause => {
            if let Ok(dbus) = ctx.dbus() {
                let _ = dbus.mpris_play_pause(svc);
            }
            if let Some(p) = state.mpris.active_player_mut() {
                p.status = match p.status {
                    MprisPlaybackStatus::Playing => MprisPlaybackStatus::Paused,
                    _ => MprisPlaybackStatus::Playing,
                };
            }
        }
        MprisControlAction::Stop => {
            if let Ok(dbus) = ctx.dbus() {
                let _ = dbus.mpris_stop(svc);
            }
            if let Some(p) = state.mpris.active_player_mut() {
                p.status = MprisPlaybackStatus::Stopped;
                p.position_us = Some(0);
            }
        }
        MprisControlAction::Previous => {
            if let Ok(dbus) = ctx.dbus() {
                let _ = dbus.mpris_previous(svc);
            }
        }
        MprisControlAction::Next => {
            if let Ok(dbus) = ctx.dbus() {
                let _ = dbus.mpris_next(svc);
            }
        }
        MprisControlAction::ToggleShuffle => {
            let next = !snap.shuffle;
            if let Ok(dbus) = ctx.dbus() {
                let _ = dbus.mpris_set_shuffle(svc, next);
            }
            if let Some(p) = state.mpris.active_player_mut() {
                p.shuffle = next;
            }
        }
        MprisControlAction::CycleLoop => {
            let next = match snap.loop_status {
                MprisLoopStatus::None => MprisLoopStatus::Playlist,
                MprisLoopStatus::Playlist => MprisLoopStatus::Track,
                MprisLoopStatus::Track => MprisLoopStatus::None,
            };
            if let Ok(dbus) = ctx.dbus() {
                let _ = dbus.mpris_set_loop_status(svc, next);
            }
            if let Some(p) = state.mpris.active_player_mut() {
                p.loop_status = next;
            }
        }
    }
}

pub fn handle_mpris_seek_click(state: &mut State, ev: PointerEvent, ctx: &EffectCtx<'_>) {
    if ev.button != Some(MouseButton::Left) {
        return;
    }
    let Some(snap) = state.mpris.active_player().cloned() else {
        return;
    };
    let Some(len_us) = snap.length_us.filter(|&l| l > 0) else {
        return;
    };
    let ratio = (ev.local_position.x.get() / MPRIS_SEEK_TRACK_WIDTH).clamp(0.0, 1.0);
    let target_us = ((len_us as f64) * f64::from(ratio)).round() as i64;
    if let Ok(dbus) = ctx.dbus() {
        if let Some(track_id) = &snap.track_id {
            let _ = dbus.mpris_set_position(&snap.service_name, track_id, target_us);
        } else if let Some(cur) = snap.position_us {
            let _ = dbus.mpris_seek(&snap.service_name, target_us - cur);
        }
    }
    if let Some(p) = state.mpris.active_player_mut() {
        p.position_us = Some(target_us);
    }
}
