//! External Unix socket & POSIX signal dispatch handlers.

use waywi::prelude::*;

use crate::app::ipc::{IpcAction, IpcTarget};
use crate::app::state::{SoundCue, State};

use super::drun::resolve_visible_drun_icons;
use super::quick_menu::refresh_power_capabilities_if_empty;

fn play_overlay_transition(state: &mut State, was_open: bool, now_open: bool, ctx: &EffectCtx<'_>) {
    if was_open != now_open {
        state.play_sound(
            if now_open {
                SoundCue::MenuPopup
            } else {
                SoundCue::MenuPopdown
            },
            ctx,
        );
    }
}

/// Closes all overlay popups except the named `keep` slot so overlays never collide.
pub fn close_other_overlays(state: &mut State, keep: Option<&str>) {
    if keep != Some("drun") {
        state.drun.close();
    }
    if keep != Some("notifications") {
        state.notifications.close_center();
    }
    if keep != Some("mpris") {
        state.mpris.close_popup();
    }
    if keep != Some("bluetooth") {
        state.bt_ui.close_popup();
    }
    if keep != Some("disk") {
        state.disk_ui.close_popup();
    }
    if keep != Some("volume") {
        state.audio_ui.close_popup();
    }
    if keep != Some("calendar") {
        state.calendar_open = false;
    }
    if keep != Some("quick_menu") {
        state.quick_menu.close_popup();
        state.settings_ui.unfocus_inputs();
        state.settings_ui.open_dropdown = None;
        if !state.hovered.is_clock_group() {
            state.clock_menu_target = false;
        }
    }
}

pub fn handle_ipc_control(
    state: &mut State,
    action: IpcAction,
    target: IpcTarget,
    ctx: &mut EffectCtx<'_>,
) {
    match target {
        IpcTarget::All => {
            if action != IpcAction::Open {
                close_other_overlays(state, None);
            }
        }
        IpcTarget::Drun => {
            let was_open = state.drun.target;
            let next = resolve_next(was_open, action);
            if next {
                close_other_overlays(state, Some("drun"));
                state.drun.open(ctx);
                resolve_visible_drun_icons(state, ctx);
            } else {
                state.drun.close();
            }
            play_overlay_transition(state, was_open, state.drun.target, ctx);
        }
        IpcTarget::Notifications => {
            let was_open = state.notifications.center_target;
            let next = resolve_next(was_open, action);
            if next {
                close_other_overlays(state, Some("notifications"));
                state.notifications.center_target = true;
                state.notifications.marquee_tick = 0;
                state.notifications.ensure_visible_icons(
                    state.notifications.scroll_index,
                    12,
                    &mut state.icon_lookup,
                    &mut state.path_asset_cache,
                    ctx,
                );
            } else {
                state.notifications.close_center();
            }
            play_overlay_transition(state, was_open, state.notifications.center_target, ctx);
        }
        IpcTarget::Dnd => {
            let want_on = resolve_next(state.notifications.dnd, action);
            if want_on != state.notifications.dnd {
                state.notifications.toggle_dnd(ctx);
                if !state.notifications.dnd {
                    state.play_sound(SoundCue::ToggleOn, ctx);
                }
            }
        }
        IpcTarget::Volume => {
            let was_open = state.audio_ui.popup_target;
            let next = resolve_next(was_open, action);
            if next {
                close_other_overlays(state, Some("volume"));
                if state.audio_ui.popup_left_x <= 0.0 {
                    state.audio_ui.popup_left_x = (state.bar_width - 340.0).max(8.0);
                }
                if !state.audio_ui.popup_target {
                    state.audio_ui.toggle_popup();
                }
            } else {
                state.audio_ui.close_popup();
            }
            play_overlay_transition(state, was_open, state.audio_ui.popup_target, ctx);
        }
        IpcTarget::Bluetooth => {
            let was_open = state.bt_ui.popup_target;
            let next = resolve_next(was_open, action);
            if next {
                close_other_overlays(state, Some("bluetooth"));
                if state.bt_ui.popup_left_x <= 0.0 {
                    state.bt_ui.popup_left_x = (state.bar_width - 378.0).max(8.0);
                }
                if !state.bt_ui.popup_target {
                    state.bt_ui.toggle_popup();
                }
            } else {
                state.bt_ui.close_popup();
            }
            play_overlay_transition(state, was_open, state.bt_ui.popup_target, ctx);
        }
        IpcTarget::Disk => {
            let was_open = state.disk_ui.popup_target;
            let next = resolve_next(was_open, action);
            if next {
                close_other_overlays(state, Some("disk"));
                if state.disk_ui.popup_left_x <= 0.0 {
                    state.disk_ui.popup_left_x = (state.bar_width - 420.0).max(8.0);
                }
                if state.udisks.block_devices.is_empty()
                    && let Ok(dbus) = ctx.dbus()
                    && let Ok(snap) = dbus.udisks_snapshot()
                {
                    state.udisks = snap;
                }
                if !state.disk_ui.popup_target {
                    state.disk_ui.toggle_popup();
                }
            } else {
                state.disk_ui.close_popup();
            }
            play_overlay_transition(state, was_open, state.disk_ui.popup_target, ctx);
        }
        IpcTarget::Mpris => {
            let was_open = state.mpris.popup_target;
            let next = resolve_next(was_open, action);
            if next {
                close_other_overlays(state, Some("mpris"));
                if !state.mpris.popup_target {
                    state.mpris.toggle_popup();
                }
            } else {
                state.mpris.close_popup();
            }
            play_overlay_transition(state, was_open, state.mpris.popup_target, ctx);
        }
        IpcTarget::QuickMenu => {
            let was_open = state.quick_menu.popup_target;
            let next = resolve_next(was_open, action);
            if next {
                close_other_overlays(state, Some("quick_menu"));
                refresh_power_capabilities_if_empty(state, ctx);
                state.quick_menu.open_popup(ctx);
                state.clock_menu_target = true;
                state.clock_menu_leave_deadline = None;
            } else {
                state.quick_menu.close_popup();
                if !state.hovered.is_clock_group() {
                    state.clock_menu_target = false;
                }
            }
            play_overlay_transition(state, was_open, state.quick_menu.popup_target, ctx);
        }
        IpcTarget::Calendar => {
            let was_open = state.calendar_open;
            let next = resolve_next(was_open, action);
            if next {
                close_other_overlays(state, Some("calendar"));
                state.calendar_open = true;
            } else {
                state.calendar_open = false;
            }
            play_overlay_transition(state, was_open, state.calendar_open, ctx);
        }
    }
}

#[must_use]
const fn resolve_next(current: bool, action: IpcAction) -> bool {
    match action {
        IpcAction::Toggle => !current,
        IpcAction::Open => true,
        IpcAction::Close => false,
    }
}
