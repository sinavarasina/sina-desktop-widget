//! UDisks2 disk management popup handlers, Polkit mount/unmount/eject/poweroff, and LUKS unlock.

use waywi::prelude::*;

use crate::app::disk::{DiskMenuAction, DiskUiState};
use crate::app::handlers::events::scroll_direction_y;
use crate::app::handlers::ipc::close_other_overlays;
use crate::app::state::{SoundCue, State};
use crate::widgets::disk_popup::DISK_POPUP_WIDTH;

pub fn handle_udisks_updated(state: &mut State, snap: UDisksSnapshot, ctx: &EffectCtx<'_, crate::app::msg::Msg>) {
    state.disk_ui.clear_settled_pending(&snap);
    state
        .sounds
        .observe_udisks(&snap, state.notifications.dnd, ctx);
    state.udisks = snap;
}

pub fn handle_disk_bar_pointer(state: &mut State, ev: PointerEvent, ctx: &EffectCtx<'_, crate::app::msg::Msg>) {
    match ev.button {
        Some(MouseButton::Left) => {
            let click_global_x = ev.position.x.get();
            let raw_left = click_global_x - DISK_POPUP_WIDTH * 0.5;
            let max_left = (state.bar_width - DISK_POPUP_WIDTH - 8.0).max(8.0);
            state.disk_ui.popup_left_x = raw_left.clamp(8.0, max_left);
            if !state.disk_ui.is_popup_open() {
                close_other_overlays(state, Some("disk"));
                if state.udisks.block_devices.is_empty()
                    && let Ok(dbus) = ctx.dbus()
                    && let Ok(snap) = dbus
                        .udisks()
                        .cached_snapshot()
                        .map_or_else(|| dbus.udisks().snapshot_blocking(), Ok)
                {
                    state.udisks = snap;
                }
            }
            state.disk_ui.toggle_popup();
            let cue = if state.disk_ui.is_popup_open() {
                SoundCue::MenuPopup
            } else {
                SoundCue::MenuPopdown
            };
            state.play_sound(cue, ctx);
        }
        Some(MouseButton::Middle) => {
            state.cycle_selected_mount();
        }
        _ => {}
    }
}

pub fn handle_disk_refresh(state: &mut State, ctx: &EffectCtx<'_, crate::app::msg::Msg>) {
    state.disk_ui.last_error = None;
    if let Ok(dbus) = ctx.dbus()
        && let Ok(snap) = dbus
            .udisks()
            .cached_snapshot()
            .map_or_else(|| dbus.udisks().snapshot_blocking(), Ok)
    {
        state.disk_ui.clear_settled_pending(&snap);
        state.udisks = snap;
    }
    if let Ok(mounts) = waywi::system::disk::read_disk_mounts() {
        state.apply_disk_mounts(mounts);
    }
}

pub fn handle_disk_device_pointer(
    state: &mut State,
    path: String,
    ev: PointerEvent,
    ctx: &EffectCtx<'_, crate::app::msg::Msg>,
) {
    let Some(block) = state
        .udisks
        .block_devices
        .iter()
        .find(|b| b.object_path == path)
        .cloned()
    else {
        return;
    };
    let menu_open_for_this = state.disk_ui.menu_path.as_deref() == Some(path.as_str());

    match ev.button {
        Some(MouseButton::Left) => {
            // Quick primary action:
            // - Locked LUKS container -> open inline passphrase prompt
            // - Unmounted filesystem -> mount immediately (invoking Polkit if required)
            // - Mounted filesystem / unlocked LUKS container -> toggle context menu so the user
            //   can deliberately pick Unmount / Lock / Eject / Power Off.
            if block.is_encrypted && !block.is_unlocked() {
                state.disk_ui.menu_path = None;
                let label = block.display_label().to_owned();
                state.disk_ui.open_unlock_prompt(path, label);
            } else if block.has_filesystem && !block.is_mounted() {
                state.disk_ui.menu_path = None;
                state.disk_ui.last_error = None;
                if let Ok(dbus) = ctx.dbus() {
                    state.disk_ui.set_pending(path.clone(), "mounting");
                    if let Err(e) = dbus.udisks().mount(&path, &UDisksMountOptions::new()) {
                        state.disk_ui.pending_ops.remove(&path);
                        state.disk_ui.last_error = Some(e.to_string());
                    }
                }
            } else {
                state.disk_ui.menu_path = if menu_open_for_this { None } else { Some(path) };
            }
        }
        Some(MouseButton::Right) => {
            state.disk_ui.menu_path = if menu_open_for_this { None } else { Some(path) };
        }
        _ => {}
    }
}

pub fn handle_disk_device_menu_action(
    state: &mut State,
    path: String,
    action: DiskMenuAction,
    ctx: &EffectCtx<'_, crate::app::msg::Msg>,
) {
    let Some(block) = state
        .udisks
        .block_devices
        .iter()
        .find(|b| b.object_path == path)
        .cloned()
    else {
        return;
    };
    state.disk_ui.menu_path = None;
    state.disk_ui.last_error = None;

    if action == DiskMenuAction::UnlockPrompt {
        let label = block.display_label().to_owned();
        state.disk_ui.open_unlock_prompt(path, label);
        return;
    }

    let Ok(dbus) = ctx.dbus() else {
        return;
    };

    match action {
        DiskMenuAction::Mount => {
            state.disk_ui.set_pending(path.clone(), "mounting");
            if let Err(e) = dbus.udisks().mount(&path, &waywi::dbus::UDisksMountOptions::default()) {
                state.disk_ui.pending_ops.remove(&path);
                state.disk_ui.last_error = Some(e.to_string());
            }
        }
        DiskMenuAction::Unmount { force } => {
            state.disk_ui.set_pending(path.clone(), "unmounting");
            if let Err(e) = dbus.udisks().unmount(&path, force, true) {
                state.disk_ui.pending_ops.remove(&path);
                state.disk_ui.last_error = Some(e.to_string());
            }
        }
        DiskMenuAction::UnlockPrompt => {}
        DiskMenuAction::Lock => {
            let lock_path = if block.is_encrypted {
                Some(block.object_path.clone())
            } else {
                block.crypto_backing_device.clone()
            };
            if let Some(target_path) = lock_path {
                state.disk_ui.set_pending(target_path.clone(), "locking");
                if let Err(e) = dbus.udisks().lock(&target_path, true) {
                    state.disk_ui.pending_ops.remove(&target_path);
                    state.disk_ui.last_error = Some(e.to_string());
                }
            }
        }
        DiskMenuAction::Eject => {
            if let Some(drive) = state.udisks.drive_for_block(&block) {
                let drive_path = drive.object_path.clone();
                state.disk_ui.set_pending(drive_path.clone(), "ejecting");
                if let Err(e) = dbus.udisks().eject(&drive_path, true) {
                    state.disk_ui.pending_ops.remove(&drive_path);
                    state.disk_ui.last_error = Some(e.to_string());
                }
            }
        }
        DiskMenuAction::PowerOff => {
            if let Some(drive) = state.udisks.drive_for_block(&block) {
                let drive_path = drive.object_path.clone();
                state
                    .disk_ui
                    .set_pending(drive_path.clone(), "powering off");
                if let Err(e) = dbus.udisks().power_off(&drive_path, true) {
                    state.disk_ui.pending_ops.remove(&drive_path);
                    state.disk_ui.last_error = Some(e.to_string());
                }
            }
        }
    }
}

pub fn handle_disk_unlock_key(state: &mut State, ev: KeyEvent, ctx: &mut EffectCtx<'_, crate::app::msg::Msg>) {
    if state.disk_ui.unlock_target.is_none() {
        if ev.key == Key::Escape {
            state.disk_ui.close_popup();
        }
        return;
    }

    let outcome = state.disk_ui.unlock_input.handle_key(&ev);
    match outcome {
        TextInputOutcome::Cancelled => state.disk_ui.close_unlock_prompt(),
        TextInputOutcome::Submitted => handle_disk_unlock_submit(state, ctx),
        TextInputOutcome::Edited | TextInputOutcome::CursorMoved => {
            state.disk_ui.remeasure_unlock_input(ctx);
        }
        TextInputOutcome::Unchanged => {}
    }
}

pub fn handle_disk_unlock_ime(state: &mut State, ev: ImeEvent, ctx: &mut EffectCtx<'_, crate::app::msg::Msg>) {
    if state.disk_ui.unlock_target.is_none() {
        return;
    }
    let outcome = state.disk_ui.unlock_input.handle_ime(&ev);
    if outcome.needs_redraw() {
        state.disk_ui.remeasure_unlock_input(ctx);
    }
}

pub fn handle_disk_unlock_submit(state: &mut State, ctx: &EffectCtx<'_, crate::app::msg::Msg>) {
    let Some(target_path) = state.disk_ui.unlock_target.take() else {
        return;
    };
    let passphrase = state.disk_ui.unlock_input.text().to_owned();
    state.disk_ui.close_unlock_prompt();

    let Ok(dbus) = ctx.dbus() else {
        return;
    };
    state.disk_ui.last_error = None;
    state.disk_ui.set_pending(target_path.clone(), "unlocking");
    if let Err(e) = dbus.udisks().unlock(&target_path, &passphrase, true) {
        state.disk_ui.pending_ops.remove(&target_path);
        state.disk_ui.last_error = Some(e.to_string());
    }
}

pub fn handle_disk_popup_scroll(state: &mut State, sc: ScrollEvent) {
    let step = scroll_direction_y(sc);
    if step == 0 {
        return;
    }
    let next = if step > 0 {
        state.disk_ui.scroll_index.saturating_add(step as usize)
    } else {
        state
            .disk_ui
            .scroll_index
            .saturating_sub(step.unsigned_abs() as usize)
    };
    let total = DiskUiState::user_blocks(&state.udisks).len();
    let max_idx = total.saturating_sub(1);
    state.disk_ui.scroll_index = next.min(max_idx);
}
