//! Hyprland, tray, and audio input event handlers.

use std::sync::Arc;

use waywi::prelude::*;

use crate::app::audio::AudioSnapshotExt;

use crate::app::config::DesktopPoseMode;
use crate::app::state::{
    AudioControlTarget, SoundCue, State, TrayEntry, TrayMenuPopup, resolve_tray_icon,
};

/// Returns Wayland vertical scroll step (`< 0` = scroll up, `> 0` = scroll down).
#[must_use]
pub fn scroll_direction_y(sc: ScrollEvent) -> i32 {
    if sc.discrete_y != 0 {
        sc.discrete_y
    } else if sc.delta_y > 0.0 {
        1
    } else if sc.delta_y < 0.0 {
        -1
    } else {
        0
    }
}

fn refresh_active_client(state: &mut State, ctx: &EffectCtx<'_, crate::app::msg::Msg>) {
    let Ok(hc) = ctx.hyprland() else {
        return;
    };
    match hc.active_client() {
        Ok(Some(c)) => {
            state.scroller.set_window(c.class.clone(), c.title.clone());
            if c.content_type.is_none() {
                state.drun.ensure_scanned();
            }
            state.window_info.set_client(Some(c), &state.drun.entries);
            if state.config.general.foreground_booster {
                state.booster.on_focus_change(
                    state.window_info.cgroup.as_ref(),
                    state.memory.as_ref(),
                    ctx,
                );
            } else {
                state.booster.clear_active(ctx);
            }
        }
        Ok(None) => {
            state.scroller.set_window(String::new(), String::new());
            state.window_info.set_client(None, &state.drun.entries);
            state.booster.clear_active(ctx);
        }
        Err(_) => {}
    }
}

fn refresh_workspaces(state: &mut State, ctx: &EffectCtx<'_, crate::app::msg::Msg>) {
    if let Ok(hc) = ctx.hyprland()
        && let Ok(list) = hc.workspaces()
    {
        if let Some(focused) = list.iter().find(|w| w.focused && !w.is_special() && w.id.get() > 0) {
            state.active_ws_id = focused.id.get();
        }
        state.workspaces = list;
    }
}

pub fn handle_hypr_event(state: &mut State, ev: HyprlandEvent, ctx: &mut EffectCtx<'_, crate::app::msg::Msg>) {
    match ev {
        HyprlandEvent::ActiveWindow { class, title } => {
            state.scroller.set_window(class, title);
            refresh_active_client(state, ctx);
        }
        HyprlandEvent::ActiveWindowV2 { .. } | HyprlandEvent::WindowTitle { .. } => {
            refresh_active_client(state, ctx);
        }
        HyprlandEvent::OpenWindow { .. }
        | HyprlandEvent::CloseWindow { .. }
        | HyprlandEvent::MoveWindowV2 { .. }
        | HyprlandEvent::Fullscreen(_) => {
            refresh_workspaces(state, ctx);
            refresh_active_client(state, ctx);
        }
        HyprlandEvent::WindowTitleV2 { address, title } => {
            state.window_info.update_title(&address, &title);
            if let Some(c) = &state.window_info.client {
                state.scroller.set_window(c.class.clone(), c.title.clone());
            } else {
                refresh_active_client(state, ctx);
            }
        }
        HyprlandEvent::ChangeFloatingMode { address, floating } => {
            state.window_info.update_floating(&address, floating);
        }
        HyprlandEvent::Pin { address, pinned } => {
            state.window_info.update_pinned(&address, pinned);
        }
        HyprlandEvent::WorkspaceV2 { id, .. }
        | HyprlandEvent::FocusedMonitorV2 {
            workspace_id: id, ..
        } => {
            let prev_ws = state.active_ws_id;
            if id.get() > 0 {
                state.active_ws_id = id.get();
            }
            if prev_ws != state.active_ws_id {
                state.prev_ws_id = prev_ws;
                if state.config.sprite.rotate_on_workspace
                    && state.config.sprite.desktop_pose == DesktopPoseMode::Auto
                {
                    state.desktop_sprite.rotate_instant(ctx);
                }
            }
            if ctx.hyprland().is_ok() {
                refresh_workspaces(state, ctx);
                refresh_active_client(state, ctx);
            } else if id.get() > 0 {
                for w in &mut state.workspaces {
                    w.focused = !w.is_special() && w.id == id;
                }
            }
        }
        HyprlandEvent::ActiveSpecial { workspace_name, .. }
        | HyprlandEvent::ActiveSpecialV2 { workspace_name, .. } => {
            let trimmed = workspace_name.trim();
            state.active_special = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_owned())
            };
            refresh_workspaces(state, ctx);
        }
        HyprlandEvent::CreateWorkspaceV2 { .. }
        | HyprlandEvent::DestroyWorkspaceV2 { .. }
        | HyprlandEvent::MoveWorkspaceV2 { .. }
        | HyprlandEvent::RenameWorkspace { .. }
        | HyprlandEvent::ConfigReloaded
        | HyprlandEvent::Reconnected => {
            refresh_workspaces(state, ctx);
        }
        HyprlandEvent::Bell { .. } => {
            state.play_sound(SoundCue::Bell, ctx);
        }
        HyprlandEvent::Urgent { .. } => {
            state.play_sound(SoundCue::WindowUrgent, ctx);
        }
        HyprlandEvent::Unknown { name, .. } if name.to_ascii_lowercase().contains("layout") => {
            refresh_workspaces(state, ctx);
        }
        _ => {}
    }
}

pub fn handle_tray_event(state: &mut State, ev: TrayEvent, ctx: &mut EffectCtx<'_, crate::app::msg::Msg>) {
    match ev {
        TrayEvent::ItemRegistered(snap) | TrayEvent::ItemUpdated(snap) => {
            let (icon, sig) = resolve_tray_icon(
                &snap,
                &mut state.icon_lookup,
                &mut state.path_asset_cache,
                ctx,
            );
            state.upsert_tray_entry(TrayEntry {
                snapshot: snap,
                icon,
                icon_signature: sig,
            });
        }
        TrayEvent::ItemUnregistered { service } => {
            state.remove_tray_service(&service);
        }
        _ => {}
    }
}

pub fn handle_tray_click(state: &mut State, index: usize, ev: PointerEvent, ctx: &EffectCtx<'_, crate::app::msg::Msg>) {
    let Some(entry) = state.tray_items.get(index) else {
        return;
    };
    let service = entry.snapshot.service.clone();
    let bus_name = entry.snapshot.bus_name.clone();
    let object_path = entry.snapshot.object_path.clone();
    let menu_path = entry.snapshot.menu_path.clone();
    let anchor_x = ev.position.x.get();
    let Ok(dbus) = ctx.dbus() else {
        return;
    };

    match ev.button {
        Some(MouseButton::Middle) => {
            let _ = dbus.tray().secondary_activate(&bus_name, &object_path, 0, 0);
        }
        Some(MouseButton::Right) => {
            if let Some(path) = menu_path
                && let Ok(items) = dbus.tray().menu_layout_blocking(&bus_name, &path)
                && !items.is_empty()
            {
                let toggle_same = state
                    .tray_menu
                    .as_ref()
                    .is_some_and(|m| m.service == service);
                state.tray_menu = if toggle_same {
                    None
                } else {
                    Some(TrayMenuPopup {
                        service,
                        bus_name,
                        menu_path: path,
                        anchor_x,
                        items,
                    })
                };
                return;
            }
            let _ = dbus.tray().context_menu(&bus_name, &object_path, 0, 0);
        }
        _ => {
            state.tray_menu = None;
            let _ = dbus.tray().activate(&bus_name, &object_path, 0, 0);
        }
    }
}

pub fn handle_audio_scroll(state: &mut State, sc: ScrollEvent, ctx: &EffectCtx<'_, crate::app::msg::Msg>) {
    // Wayland vertical scroll UP produces negative delta_y / discrete_y; negate to raise volume.
    let step = -scroll_direction_y(sc);
    if step == 0 {
        return;
    }
    match state.audio_target {
        AudioControlTarget::Sink => {
            let current = state
                .audio
                .as_ref()
                .and_then(|a| a.target_volume(false))
                .map_or(0.5, |v| v.linear);
            let next = (current + (step as f32) * 0.01).clamp(0.0, 1.5);
            if let Ok(ac) = ctx.audio() {
                let _ = ac.set_volume(AudioTarget::DefaultSink, next);
            }
            if let Some(audio) = &mut state.audio
                && let Some(dev) = Arc::make_mut(audio).default_device_mut(false)
            {
                dev.volume.linear = next;
            }
        }
        AudioControlTarget::Source => {
            let current = state
                .audio
                .as_ref()
                .and_then(|a| a.target_volume(true))
                .map_or(0.5, |v| v.linear);
            let next = (current + (step as f32) * 0.01).clamp(0.0, 1.5);
            if let Ok(ac) = ctx.audio() {
                let _ = ac.set_volume(AudioTarget::DefaultSource, next);
            }
            if let Some(audio) = &mut state.audio
                && let Some(dev) = Arc::make_mut(audio).default_device_mut(true)
            {
                dev.volume.linear = next;
            }
        }
    }
    let muted = state
        .audio
        .as_ref()
        .and_then(|a| a.target_volume(state.audio_target == AudioControlTarget::Source))
        .is_some_and(|v| v.muted);
    if !muted {
        state.play_sound(SoundCue::VolumeChange, ctx);
    }
}
