//! External Unix socket & POSIX signal dispatch handlers.

use waywi::prelude::*;

use crate::app::ipc::{IpcAction, IpcTarget};
use crate::app::state::{SoundCue, State};

use super::drun::resolve_visible_drun_icons;
use super::quick_menu::refresh_power_capabilities_if_empty;

fn play_overlay_transition(state: &mut State, was_open: bool, now_open: bool, ctx: &EffectCtx<'_, crate::app::msg::Msg>) {
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
            state.clock_menu_anim.set_open(false);
        }
    }
}

pub fn handle_ipc_control(
    state: &mut State,
    action: IpcAction,
    target: IpcTarget,
    ctx: &mut EffectCtx<'_, crate::app::msg::Msg>,
) {
    match target {
        IpcTarget::All => {
            if action != IpcAction::Open {
                close_other_overlays(state, None);
            }
        }
        IpcTarget::Drun => {
            let was_open = state.drun.is_open();
            let next = resolve_next(was_open, action);
            if next {
                close_other_overlays(state, Some("drun"));
                state.drun.open(ctx);
                resolve_visible_drun_icons(state, ctx);
            } else {
                state.drun.close();
            }
            play_overlay_transition(state, was_open, state.drun.is_open(), ctx);
        }
        IpcTarget::Notifications => {
            let was_open = state.notifications.is_center_open();
            let next = resolve_next(was_open, action);
            if next {
                close_other_overlays(state, Some("notifications"));
                state.notifications.open_center();
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
            play_overlay_transition(state, was_open, state.notifications.is_center_open(), ctx);
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
            let was_open = state.audio_ui.is_popup_open();
            let next = resolve_next(was_open, action);
            if next {
                close_other_overlays(state, Some("volume"));
                if state.audio_ui.popup_left_x <= 0.0 {
                    state.audio_ui.popup_left_x = (state.bar_width - 340.0).max(8.0);
                }
                state.audio_ui.open_popup();
            } else {
                state.audio_ui.close_popup();
            }
            play_overlay_transition(state, was_open, state.audio_ui.is_popup_open(), ctx);
        }
        IpcTarget::Bluetooth => {
            let was_open = state.bt_ui.is_popup_open();
            let next = resolve_next(was_open, action);
            if next {
                close_other_overlays(state, Some("bluetooth"));
                if state.bt_ui.popup_left_x <= 0.0 {
                    state.bt_ui.popup_left_x = (state.bar_width - 378.0).max(8.0);
                }
                state.bt_ui.open_popup();
            } else {
                state.bt_ui.close_popup();
            }
            play_overlay_transition(state, was_open, state.bt_ui.is_popup_open(), ctx);
        }
        IpcTarget::Disk => {
            let was_open = state.disk_ui.is_popup_open();
            let next = resolve_next(was_open, action);
            if next {
                close_other_overlays(state, Some("disk"));
                if state.disk_ui.popup_left_x <= 0.0 {
                    state.disk_ui.popup_left_x = (state.bar_width - 420.0).max(8.0);
                }
                if state.udisks.block_devices.is_empty()
                    && let Ok(dbus) = ctx.dbus()
                    && let Ok(snap) = dbus.udisks().cached_snapshot().map_or_else(|| dbus.udisks().snapshot_blocking(), Ok)
                {
                    state.udisks = snap;
                }
                state.disk_ui.open_popup();
            } else {
                state.disk_ui.close_popup();
            }
            play_overlay_transition(state, was_open, state.disk_ui.is_popup_open(), ctx);
        }
        IpcTarget::Mpris => {
            let was_open = state.mpris.is_popup_open();
            let next = resolve_next(was_open, action);
            if next {
                close_other_overlays(state, Some("mpris"));
                state.mpris.open_popup();
            } else {
                state.mpris.close_popup();
            }
            play_overlay_transition(state, was_open, state.mpris.is_popup_open(), ctx);
        }
        IpcTarget::QuickMenu => {
            let was_open = state.quick_menu.is_popup_open();
            let next = resolve_next(was_open, action);
            if next {
                close_other_overlays(state, Some("quick_menu"));
                refresh_power_capabilities_if_empty(state, ctx);
                state.quick_menu.open_popup(ctx);
                state.clock_menu_anim.set_open(true);
                state.clock_menu_leave_deadline = None;
            } else {
                state.quick_menu.close_popup();
                if !state.hovered.is_clock_group() {
                    state.clock_menu_anim.set_open(false);
                }
            }
            play_overlay_transition(state, was_open, state.quick_menu.is_popup_open(), ctx);
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
