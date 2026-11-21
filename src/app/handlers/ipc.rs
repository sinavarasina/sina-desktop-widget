//! External Unix socket & POSIX signal dispatch handlers.

use waywi::prelude::*;

use crate::app::ipc::{IpcAction, IpcTarget};
use crate::app::state::State;

use super::drun::resolve_visible_drun_icons;

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
            let next = resolve_next(state.drun.target, action);
            if next {
                close_other_overlays(state, Some("drun"));
                state.drun.open(ctx);
                resolve_visible_drun_icons(state, ctx);
            } else {
                state.drun.close();
            }
        }
        IpcTarget::Notifications => {
            let next = resolve_next(state.notifications.center_target, action);
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
        }
        IpcTarget::Dnd => {
            let want_on = resolve_next(state.notifications.dnd, action);
            if want_on != state.notifications.dnd {
                state.notifications.toggle_dnd();
            }
        }
        IpcTarget::Volume => {
            let next = resolve_next(state.audio_ui.popup_target, action);
            if next {
                close_other_overlays(state, Some("volume"));
                if state.audio_ui.popup_left_x <= 0.0 {
                    state.audio_ui.popup_left_x = (state.bar_width - 340.0).max(8.0);
                }
                state.audio_ui.toggle_popup();
            } else {
                state.audio_ui.close_popup();
            }
        }
        IpcTarget::Bluetooth => {
            let next = resolve_next(state.bt_ui.popup_target, action);
            if next {
                close_other_overlays(state, Some("bluetooth"));
                if state.bt_ui.popup_left_x <= 0.0 {
                    state.bt_ui.popup_left_x = (state.bar_width - 378.0).max(8.0);
                }
                state.bt_ui.toggle_popup();
            } else {
                state.bt_ui.close_popup();
            }
        }
        IpcTarget::Disk => {
            let next = resolve_next(state.disk_ui.popup_target, action);
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
                state.disk_ui.toggle_popup();
            } else {
                state.disk_ui.close_popup();
            }
        }
        IpcTarget::Mpris => {
            let next = resolve_next(state.mpris.popup_target, action);
            if next {
                close_other_overlays(state, Some("mpris"));
                state.mpris.toggle_popup();
            } else {
                state.mpris.close_popup();
            }
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
