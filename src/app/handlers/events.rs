//! Hyprland, tray, and audio input event handlers.

use waywi::prelude::*;

use crate::app::state::{AudioControlTarget, State, TrayEntry, TrayMenuPopup, resolve_tray_icon};

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

fn refresh_active_client(state: &mut State, ctx: &EffectCtx<'_>) {
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
            state.booster.on_focus_change(
                state.window_info.cgroup.as_ref(),
                state.memory.as_ref(),
                ctx,
            );
        }
        Ok(None) => {
            state.scroller.set_window(String::new(), String::new());
            state.window_info.set_client(None, &state.drun.entries);
            state.booster.clear_active(ctx);
        }
        Err(_) => {}
    }
}

pub fn handle_hypr_event(state: &mut State, ev: HyprlandEvent, ctx: &EffectCtx<'_>) {
    match ev {
        HyprlandEvent::ActiveWindow { class, title } => {
            state.scroller.set_window(class, title);
            refresh_active_client(state, ctx);
        }
        HyprlandEvent::ActiveWindowV2 { .. }
        | HyprlandEvent::CloseWindow { .. }
        | HyprlandEvent::WindowTitle { .. }
        | HyprlandEvent::MoveWindowV2 { .. }
        | HyprlandEvent::Fullscreen(_) => {
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
            state.active_ws_id = id.get();
            if let Ok(hc) = ctx.hyprland() {
                if let Ok(list) = hc.workspaces() {
                    state.workspaces = list;
                }
                refresh_active_client(state, ctx);
            } else {
                for w in &mut state.workspaces {
                    w.focused = w.id == id;
                }
            }
        }
        HyprlandEvent::CreateWorkspaceV2 { .. }
        | HyprlandEvent::DestroyWorkspaceV2 { .. }
        | HyprlandEvent::MoveWorkspaceV2 { .. } => {
            if let Ok(hc) = ctx.hyprland()
                && let Ok(list) = hc.workspaces()
            {
                state.workspaces = list;
            }
        }
        _ => {}
    }
}

pub fn handle_tray_event(state: &mut State, ev: TrayEvent, ctx: &mut EffectCtx<'_>) {
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

pub fn handle_tray_click(state: &mut State, index: usize, ev: PointerEvent, ctx: &EffectCtx<'_>) {
    let Some(entry) = state.tray_items.get(index) else {
        return;
    };
    let service = entry.snapshot.service.clone();
    let bus_name = entry.snapshot.bus_name.clone();
    let menu_path = entry.snapshot.menu_path.clone();
    let anchor_x = ev.position.x.get();
    let Ok(dbus) = ctx.dbus() else {
        return;
    };

    match ev.button {
        Some(MouseButton::Middle) => {
            let _ = dbus.tray_secondary_activate(&service, 0, 0);
        }
        Some(MouseButton::Right) => {
            if let Some(path) = menu_path
                && let Ok(items) = dbus.tray_menu_layout(&bus_name, &path)
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
            let _ = dbus.tray_context_menu(&service, 0, 0);
        }
        _ => {
            state.tray_menu = None;
            let _ = dbus.tray_activate(&service, 0, 0);
        }
    }
}

pub fn handle_audio_scroll(state: &mut State, sc: ScrollEvent, ctx: &EffectCtx<'_>) {
    // Wayland vertical scroll UP produces negative delta_y / discrete_y; negate to raise volume.
    let step = -scroll_direction_y(sc);
    if step == 0 {
        return;
    }
    match state.audio_target {
        AudioControlTarget::Sink => {
            let current = state.audio.as_ref().map_or(0.5, |a| a.linear);
            let next = (current + (step as f32) * 0.01).clamp(0.0, 1.5);
            if let Ok(ac) = ctx.audio() {
                let _ = ac.set_volume_linear(next);
            }
            if let Some(audio) = &mut state.audio {
                audio.linear = next;
            }
        }
        AudioControlTarget::Source => {
            let current = state.audio.as_ref().map_or(0.5, |a| a.source_linear);
            let next = (current + (step as f32) * 0.01).clamp(0.0, 1.5);
            if let Ok(ac) = ctx.audio() {
                let _ = ac.set_source_volume_linear(next);
            }
            if let Some(audio) = &mut state.audio {
                audio.source_linear = next;
            }
        }
    }
}
