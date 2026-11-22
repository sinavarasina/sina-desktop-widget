//! Bluetooth popup and BlueZ pairing agent event handlers.

use std::time::SystemTime;

use waywi::prelude::*;

use crate::app::bluetooth::BtDeviceMenuAction;
use crate::app::state::{SoundCue, State};

use super::events::scroll_direction_y;

pub fn handle_bluetooth_agent_event(
    state: &mut State,
    ev: BluetoothAgentEvent,
    ctx: &mut EffectCtx<'_>,
) {
    if let Some(prompt) = state.bt_ui.apply_agent_event(ev, &state.bluetooth) {
        // Also surface pairing prompt into the Notification Center with Accept / Reject actions!
        let notif_id = 900_000 + (prompt.request_id as u32);
        let notif = DesktopNotification {
            id: notif_id,
            app_name: String::from("Bluetooth"),
            replaces_id: notif_id,
            app_icon: String::from("bluetooth"),
            summary: String::from("Bluetooth Pairing"),
            body: prompt.label,
            actions: vec![
                (
                    format!("bt-accept:{}", prompt.request_id),
                    String::from("Accept"),
                ),
                (
                    format!("bt-reject:{}", prompt.request_id),
                    String::from("Reject"),
                ),
            ],
            urgency: NotificationUrgency::Critical,
            expire_timeout_ms: 0,
            received_at: SystemTime::now(),
            ..DesktopNotification::default()
        };
        state.notifications.push_incoming(
            notif,
            &mut state.icon_lookup,
            &mut state.path_asset_cache,
            &mut state.sounds,
            ctx,
        );
    }
}

pub fn handle_bt_bar_pointer(state: &mut State, ev: PointerEvent, ctx: &EffectCtx<'_>) {
    if ev.button == Some(MouseButton::Left) {
        state.bt_ui.popup_left_x = (ev.position.x.get() - 140.0).max(8.0);
        state.bt_ui.toggle_popup();
        let cue = if state.bt_ui.popup_target {
            SoundCue::MenuPopup
        } else {
            SoundCue::MenuPopdown
        };
        state.play_sound(cue, ctx);
    }
}

pub fn handle_bt_toggle_power(state: &mut State, ctx: &EffectCtx<'_>) {
    let Some(adapter) = state.bluetooth.adapters.first() else {
        return;
    };
    let path = adapter.object_path.clone();
    let next = !adapter.powered;
    if let Ok(dbus) = ctx.dbus() {
        let _ = dbus.set_bluetooth_powered(&path, next);
    }
    if let Some(a) = state.bluetooth.adapters.first_mut() {
        a.powered = next;
    }
    state.play_sound(
        if next {
            SoundCue::ToggleOn
        } else {
            SoundCue::ToggleOff
        },
        ctx,
    );
}

pub fn handle_bt_toggle_scan(state: &mut State, ctx: &EffectCtx<'_>) {
    let Some(adapter) = state.bluetooth.adapters.first() else {
        return;
    };
    let path = adapter.object_path.clone();
    let discovering = adapter.discovering;
    if let Ok(dbus) = ctx.dbus() {
        if discovering {
            let _ = dbus.stop_bluetooth_discovery(&path);
        } else {
            let _ = dbus.start_bluetooth_discovery(&path);
        }
    }
    let next = !discovering;
    if let Some(a) = state.bluetooth.adapters.first_mut() {
        a.discovering = next;
    }
    state.play_sound(
        if next {
            SoundCue::ToggleOn
        } else {
            SoundCue::ToggleOff
        },
        ctx,
    );
}

pub fn handle_bt_toggle_discoverable(state: &mut State, ctx: &EffectCtx<'_>) {
    let Some(adapter) = state.bluetooth.adapters.first() else {
        return;
    };
    let path = adapter.object_path.clone();
    let discoverable = adapter.discoverable;
    let next = !discoverable;
    if let Ok(dbus) = ctx.dbus() {
        let _ = dbus.set_bluetooth_discoverable(&path, next);
    }
    if let Some(a) = state.bluetooth.adapters.first_mut() {
        a.discoverable = next;
    }
    state.play_sound(
        if next {
            SoundCue::ToggleOn
        } else {
            SoundCue::ToggleOff
        },
        ctx,
    );
}

pub fn handle_bt_device_pointer(
    state: &mut State,
    path: String,
    ev: PointerEvent,
    ctx: &EffectCtx<'_>,
) {
    let Some(dev) = state
        .bluetooth
        .devices
        .iter()
        .find(|d| d.object_path == path)
        .cloned()
    else {
        return;
    };
    let menu_open_for_this = state.bt_ui.menu_path.as_deref() == Some(path.as_str());

    match ev.button {
        Some(MouseButton::Left) => {
            state.bt_ui.menu_path = None;
            state.bt_ui.last_error = None;
            if let Ok(dbus) = ctx.dbus() {
                if dev.connected {
                    state.bt_ui.set_pending(path.clone(), "disconnecting");
                    let _ = dbus.disconnect_bluetooth_device(&path);
                } else if dev.paired {
                    state.bt_ui.set_pending(path.clone(), "connecting");
                    let _ = dbus.connect_bluetooth_device(&path);
                } else {
                    state.bt_ui.set_pending(path.clone(), "pairing");
                    let _ = dbus.pair_bluetooth_device(&path);
                }
            }
        }
        Some(MouseButton::Right) => {
            // Right-click toggles the device context menu (Connect / Trust / Forget).
            state.bt_ui.menu_path = if menu_open_for_this { None } else { Some(path) };
        }
        _ => {}
    }
}

pub fn handle_bt_device_menu_action(
    state: &mut State,
    path: String,
    action: BtDeviceMenuAction,
    ctx: &EffectCtx<'_>,
) {
    let Some(dev) = state
        .bluetooth
        .devices
        .iter()
        .find(|d| d.object_path == path)
        .cloned()
    else {
        return;
    };
    state.bt_ui.menu_path = None;
    let Ok(dbus) = ctx.dbus() else {
        return;
    };
    match action {
        BtDeviceMenuAction::ToggleConnect => {
            state.bt_ui.last_error = None;
            if dev.connected {
                state.bt_ui.set_pending(path.clone(), "disconnecting");
                let _ = dbus.disconnect_bluetooth_device(&path);
            } else if dev.paired {
                state.bt_ui.set_pending(path.clone(), "connecting");
                let _ = dbus.connect_bluetooth_device(&path);
            } else {
                state.bt_ui.set_pending(path.clone(), "pairing");
                let _ = dbus.pair_bluetooth_device(&path);
            }
        }
        BtDeviceMenuAction::ToggleTrust => {
            let next = !dev.trusted;
            let _ = dbus.trust_bluetooth_device(&path, next);
            if let Some(d) = state
                .bluetooth
                .devices
                .iter_mut()
                .find(|d| d.object_path == path)
            {
                d.trusted = next;
            }
        }
        BtDeviceMenuAction::Forget => {
            state.bt_ui.last_error = None;
            state.bt_ui.set_pending(path.clone(), "removing");
            let _ = dbus.remove_bluetooth_device(&dev.adapter_path, &path);
            state.bluetooth.devices.retain(|d| d.object_path != path);
        }
    }
}

pub fn handle_bt_agent_decision(
    state: &mut State,
    request_id: u64,
    accept: bool,
    ctx: &EffectCtx<'_>,
) {
    let prompt = state.bt_ui.pending_prompt.clone();
    if state
        .bt_ui
        .pending_prompt
        .as_ref()
        .is_some_and(|p| p.request_id == request_id)
    {
        state.bt_ui.pending_prompt = None;
    }
    state.notifications.dismiss(900_000 + (request_id as u32));

    let Ok(dbus) = ctx.dbus() else {
        return;
    };
    if !accept {
        let _ = dbus.bluetooth_agent_reject(request_id);
        return;
    }
    if let Some(p) = prompt {
        if p.needs_pin {
            let _ = dbus.bluetooth_agent_reply_pincode(request_id, "0000");
        } else if p.needs_passkey {
            let _ = dbus.bluetooth_agent_reply_passkey(request_id, 0);
        } else {
            let _ = dbus.bluetooth_agent_confirm(request_id);
        }
    } else {
        let _ = dbus.bluetooth_agent_confirm(request_id);
    }
}

pub fn handle_bt_popup_scroll(state: &mut State, sc: ScrollEvent) {
    let step = scroll_direction_y(sc);
    if step == 0 {
        return;
    }
    let next = if step > 0 {
        state.bt_ui.scroll_index.saturating_add(step as usize)
    } else {
        state
            .bt_ui
            .scroll_index
            .saturating_sub(step.unsigned_abs() as usize)
    };
    let max_idx = state.bluetooth.devices.len().saturating_sub(1);
    state.bt_ui.scroll_index = next.min(max_idx);
}

pub fn handle_dbus_action_result(state: &mut State, res: DbusActionResult, ctx: &EffectCtx<'_>) {
    let had_error = res.error.is_some();
    if matches!(
        res.lane,
        DbusActionLane::BluetoothDevice | DbusActionLane::BluetoothAdapter
    ) {
        state.bt_ui.pending_ops.remove(&res.target);
        state.bt_ui.last_error = res.error.map(|e| clean_dbus_error(&e));
    } else if res.target.starts_with("/org/freedesktop/UDisks2/") {
        state.disk_ui.pending_ops.remove(&res.target);
        state.disk_ui.last_error = res.error.map(|e| clean_dbus_error(&e));
        if state.disk_ui.last_error.is_none()
            && let Ok(mounts) = waywi::system::disk::read_disk_mounts()
        {
            state.apply_disk_mounts(mounts);
        }
    } else if res.target.starts_with("/org/freedesktop/login1") {
        state.quick_menu.power_error = res.error.map(|e| clean_dbus_error(&e));
    }
    if had_error {
        state.play_sound(SoundCue::DialogError, ctx);
    }
}

fn clean_dbus_error(raw: &str) -> String {
    let trimmed = raw
        .trim_start_matches("D-Bus error: ")
        .trim_start_matches("Filesystem.Mount failed: ")
        .trim_start_matches("Filesystem.Unmount failed: ")
        .trim_start_matches("Drive.Eject failed: ")
        .trim_start_matches("Drive.PowerOff failed: ")
        .trim_start_matches("Encrypted.Unlock failed: ")
        .trim_start_matches("Encrypted.Lock failed: ");
    if let Some((_, msg)) = trimmed.split_once(": ") {
        msg.trim().to_owned()
    } else {
        trimmed.trim().to_owned()
    }
}
