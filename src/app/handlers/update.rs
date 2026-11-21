//! Message dispatch and state transitions.

use waywi::desktop::spawn_detached;
use waywi::prelude::*;

use crate::app::state::{AudioControlTarget, Msg, State};

use super::audio::{
    handle_audio_updated, handle_mixer_bar_pointer, handle_mixer_capture_scroll,
    handle_mixer_capture_seek_click, handle_mixer_master_seek_click, handle_mixer_set_target,
    handle_mixer_stream_scroll, handle_mixer_stream_seek_click,
};
use super::bluetooth::{
    handle_bluetooth_agent_event, handle_bt_agent_decision, handle_bt_bar_pointer,
    handle_bt_device_menu_action, handle_bt_device_pointer, handle_bt_popup_scroll,
    handle_bt_toggle_discoverable, handle_bt_toggle_power, handle_bt_toggle_scan,
    handle_dbus_action_result,
};
use super::disk::{
    handle_disk_bar_pointer, handle_disk_device_menu_action, handle_disk_device_pointer,
    handle_disk_popup_scroll, handle_disk_refresh, handle_disk_unlock_ime, handle_disk_unlock_key,
    handle_disk_unlock_submit, handle_udisks_updated,
};
use super::drun::{
    handle_drun_ime, handle_drun_key, handle_drun_launch_index, handle_drun_scroll,
    handle_toggle_drun,
};
use super::events::{
    handle_audio_scroll, handle_hypr_event, handle_tray_click, handle_tray_event,
    scroll_direction_y,
};
use super::ipc::handle_ipc_control;
use super::mpris::{
    handle_mpris_action, handle_mpris_bar_pointer, handle_mpris_event, handle_mpris_scroll,
    handle_mpris_seek_click,
};
use super::notifications::{
    handle_notif_action_click, handle_notif_action_scroll, handle_notif_bar_pointer,
    handle_notif_card_pointer, handle_notif_center_scroll, handle_notification_event,
};

pub fn update_state(state: &mut State, msg: Msg, ctx: &mut EffectCtx<'_>) -> Result<()> {
    match msg {
        Msg::SpawnBtop => {
            let _ = spawn_detached("footclient", &["btop"]);
        }
        Msg::SwitchWorkspace(id) => {
            if let Ok(hc) = ctx.hyprland() {
                let _ = hc.switch_workspace(id);
                if let Ok(list) = hc.workspaces() {
                    state.workspaces = list;
                }
                state.active_ws_id = id.get();
            } else {
                for w in &mut state.workspaces {
                    w.focused = w.id == id;
                }
                state.active_ws_id = id.get();
            }
        }
        Msg::HyprEvent(ev) => handle_hypr_event(state, ev, ctx),
        Msg::ScrollTick => state.scroller.advance(),
        Msg::ToggleDrun => handle_toggle_drun(state, ctx),
        Msg::CloseDrun => state.drun.close(),
        Msg::DrunKey(ev) => handle_drun_key(state, ev, ctx),
        Msg::DrunIme(ev) => handle_drun_ime(state, ev, ctx),
        Msg::DrunLaunchIndex(idx) => handle_drun_launch_index(state, idx),
        Msg::DrunScroll(sc) => handle_drun_scroll(state, sc, ctx),
        Msg::DrunAnimTick => state.drun.step_animation(),
        Msg::DrunMarqueeTick => {
            state.drun.marquee_tick = state.drun.marquee_tick.wrapping_add(1);
        }
        Msg::IpcControl { action, target } => handle_ipc_control(state, action, target, ctx),
        Msg::DbusActionOutcome(res) => handle_dbus_action_result(state, res),
        Msg::NotificationEvent(ev) => handle_notification_event(state, *ev, ctx),
        Msg::NotifBarPointerDown(ev) => handle_notif_bar_pointer(state, ev, ctx),
        Msg::NotifTickerTick => state.notifications.step_ticker(),
        Msg::NotifCenterAnimTick => state.notifications.step_center_animation(),
        Msg::NotifCenterMarqueeTick => state.notifications.advance_marquee(),
        Msg::CloseNotificationCenter => {
            state.notifications.close_center();
        }
        Msg::NotifCenterScroll(sc) => handle_notif_center_scroll(state, sc, ctx),
        Msg::NotifCenterCardPointerDown { id, ev } => {
            handle_notif_card_pointer(state, id, ev, ctx);
        }
        Msg::NotifCenterActionClick { id, action_key } => {
            handle_notif_action_click(state, id, &action_key, ctx);
        }
        Msg::NotifActionScroll { id, sc } => {
            handle_notif_action_scroll(state, id, sc);
        }
        Msg::NotifCenterClearAll => {
            state.notifications.clear_all();
        }
        Msg::NotifCenterToggleDnd => {
            state.notifications.toggle_dnd(ctx);
        }
        Msg::MprisEvent(ev) => handle_mpris_event(state, *ev, ctx),
        Msg::MprisBarPointerDown(ev) => handle_mpris_bar_pointer(state, ev, ctx),
        Msg::MprisBarScroll(sc) => handle_mpris_scroll(state, sc, ctx),
        Msg::MprisAction(action) => handle_mpris_action(state, action, ctx),
        Msg::MprisSeekClick(ev) => handle_mpris_seek_click(state, ev, ctx),
        Msg::MprisTick => state.mpris.advance_tick(),
        Msg::MprisPopupAnimTick => state.mpris.step_popup_animation(),
        Msg::CloseMprisPopup => {
            state.mpris.close_popup();
        }
        Msg::BluetoothAgentEvent(ev) => handle_bluetooth_agent_event(state, *ev, ctx),
        Msg::BtBarPointerDown(ev) => handle_bt_bar_pointer(state, ev),
        Msg::BtTogglePower => handle_bt_toggle_power(state, ctx),
        Msg::BtToggleScan => handle_bt_toggle_scan(state, ctx),
        Msg::BtToggleDiscoverable => handle_bt_toggle_discoverable(state, ctx),
        Msg::BtDevicePointerDown { path, ev } => handle_bt_device_pointer(state, path, ev, ctx),
        Msg::BtDeviceMenuAction { path, action } => {
            handle_bt_device_menu_action(state, path, action, ctx);
        }
        Msg::BtAgentDecision { request_id, accept } => {
            handle_bt_agent_decision(state, request_id, accept, ctx);
        }
        Msg::BtPopupScroll(sc) => handle_bt_popup_scroll(state, sc),
        Msg::BtPopupAnimTick => state.bt_ui.step_popup_animation(),
        Msg::BtPendingTick => state.bt_ui.advance_pending_tick(),
        Msg::CloseBtPopup => state.bt_ui.close_popup(),
        Msg::UDisksUpdated(snap) => handle_udisks_updated(state, snap),
        Msg::DiskBarPointerDown(ev) => handle_disk_bar_pointer(state, ev, ctx),
        Msg::DiskRefresh => handle_disk_refresh(state, ctx),
        Msg::DiskToggleSystem => state.disk_ui.toggle_show_system(),
        Msg::DiskDevicePointerDown { path, ev } => {
            handle_disk_device_pointer(state, path, ev, ctx);
        }
        Msg::DiskDeviceMenuAction { path, action } => {
            handle_disk_device_menu_action(state, path, action, ctx);
        }
        Msg::DiskUnlockKey(ev) => handle_disk_unlock_key(state, ev, ctx),
        Msg::DiskUnlockIme(ev) => handle_disk_unlock_ime(state, ev, ctx),
        Msg::DiskUnlockSubmit => handle_disk_unlock_submit(state, ctx),
        Msg::DiskUnlockCancel => state.disk_ui.close_unlock_prompt(),
        Msg::DiskPopupScroll(sc) => handle_disk_popup_scroll(state, sc),
        Msg::DiskPopupAnimTick => state.disk_ui.step_popup_animation(),
        Msg::DiskPendingTick => state.disk_ui.advance_pending_tick(),
        Msg::DiskMarqueeTick => state.disk_ui.advance_marquee(),
        Msg::CloseDiskPopup => state.disk_ui.close_popup(),
        Msg::Noop => {}
        Msg::TrayEvent(ev) => handle_tray_event(state, *ev, ctx),
        Msg::TrayClick { index, ev } => handle_tray_click(state, index, ev, ctx),
        Msg::TrayMenuClick { item_id } => {
            if let Some(menu) = state.tray_menu.take()
                && let Ok(dbus) = ctx.dbus()
            {
                let _ = dbus.tray_menu_event(&menu.bus_name, &menu.menu_path, item_id, "clicked");
            }
        }
        Msg::CloseTrayMenu => {
            state.tray_menu = None;
        }
        Msg::TrayBlinkTick => {
            state.blink_phase = !state.blink_phase;
        }
        Msg::AudioUpdated(snap) => handle_audio_updated(state, snap, ctx),
        Msg::AudioPointerDown(ev) => match ev.button {
            Some(MouseButton::Middle) => {
                state.audio_target = match state.audio_target {
                    AudioControlTarget::Sink => AudioControlTarget::Source,
                    AudioControlTarget::Source => AudioControlTarget::Sink,
                };
            }
            Some(MouseButton::Left) => {
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
            }
            _ => {}
        },
        Msg::AudioScroll(sc) => handle_audio_scroll(state, sc, ctx),
        Msg::MixerBarPointerDown(ev) => handle_mixer_bar_pointer(state, ev, ctx),
        Msg::MixerPopupAnimTick => state.audio_ui.step_popup_animation(),
        Msg::MixerMarqueeTick => state.audio_ui.advance_marquee(),
        Msg::CloseMixerPopup => state.audio_ui.close_popup(),
        Msg::MixerSetTarget(target) => handle_mixer_set_target(state, target),
        Msg::MixerMasterScroll(sc) => handle_audio_scroll(state, sc, ctx),
        Msg::MixerMasterSeekClick(ev) => handle_mixer_master_seek_click(state, ev, ctx),
        Msg::MixerStreamScroll { id, sc } => handle_mixer_stream_scroll(state, id, sc, ctx),
        Msg::MixerStreamSeekClick { id, ev } => {
            handle_mixer_stream_seek_click(state, id, ev, ctx);
        }
        Msg::MixerCaptureScroll { id, sc } => handle_mixer_capture_scroll(state, id, sc, ctx),
        Msg::MixerCaptureSeekClick { id, ev } => {
            handle_mixer_capture_seek_click(state, id, ev, ctx);
        }
        Msg::CpuSampled(snap) => {
            let now = snap.timestamp;
            state.cpu = Some(snap);
            if let Some(watts) = state.cpu_power_tracker.sample_now(now) {
                state.cpu_power_watts = Some(watts);
            }
        }
        Msg::GpuSampled(snap) => {
            state.gpu = Some(snap);
        }
        Msg::SensorsSampled(sensors) => {
            state.cpu_temp_celsius = crate::app::telemetry::pick_cpu_temperature(&sensors);
        }
        Msg::UptimeSampled(snap) => {
            state.sysinfo.uptime = Some(snap);
        }
        Msg::ReelAnimTick => {
            state
                .sysinfo
                .step_animation(state.cpu.as_ref(), state.memory.as_ref());
            state
                .window_info
                .step_animation(state.booster.short_badge());
        }
        Msg::StartupHydrate => {
            if !state.startup_hydrated {
                state.startup_hydrated = true;
                state.notifications.ensure_visible_icons(
                    0,
                    crate::theme::NOTIF_MAX_BAR_ICONS,
                    &mut state.icon_lookup,
                    &mut state.path_asset_cache,
                    ctx,
                );
                if state
                    .window_info
                    .client
                    .as_ref()
                    .is_some_and(|c| c.content_type.is_none())
                {
                    state.drun.ensure_scanned();
                    state.window_info.refresh_category(&state.drun.entries);
                }
                state.booster.on_focus_change(
                    state.window_info.cgroup.as_ref(),
                    state.memory.as_ref(),
                    ctx,
                );
            }
        }
        Msg::MemorySampled(snap) => {
            state.memory = Some(snap);
        }
        Msg::DiskSampled(snap) => {
            state.apply_disk_snapshot(&snap);
        }
        Msg::NetworkSampled(snap) => {
            state.network = Some(snap);
        }
        Msg::NetworkManagerUpdated(snap) => {
            state.nm = Some(snap);
        }
        Msg::BluetoothUpdated(snap) => {
            state.bt_ui.clear_settled_pending(&snap);
            state.bluetooth = snap;
        }
        Msg::TimeTick(snap) => {
            state.update_clock_from_system(snap.wall_clock);
            state
                .sysinfo
                .tick_second(state.cpu.as_ref(), state.memory.as_ref());
            state.window_info.tick_second(state.booster.short_badge());
        }
        Msg::ClockPointerDown(ev) => match ev.button {
            Some(MouseButton::Left) => state.clock_alt = !state.clock_alt,
            Some(MouseButton::Right) => state.cal_year_mode = !state.cal_year_mode,
            _ => {}
        },
        Msg::ClockScroll(sc) => {
            let step = scroll_direction_y(sc);
            if step != 0 {
                state.cal_month_offset = (state.cal_month_offset - step).clamp(-120, 120);
            }
        }
        Msg::Hover(target) => state.update_hover(target, true),
        Msg::HoverAnchorX(x) => {
            if state
                .hover_anchor_x
                .is_none_or(|prev| (prev - x).abs() > 0.5)
            {
                state.hover_anchor_x = Some(x);
            }
        }
        Msg::Leave(target) => state.update_hover(target, false),
        Msg::DrawerTick => state.step_drawer_animation(),
        Msg::DrawerTimeoutCheck => state.expire_drawer_leave(),
        #[cfg(test)]
        Msg::TestSetWorkspaces(ws, active_id) => {
            state.workspaces = ws;
            state.active_ws_id = active_id;
        }
        #[cfg(test)]
        Msg::TestSetTrayMenu(menu) => {
            state.tray_menu = Some(menu);
        }
        #[cfg(test)]
        Msg::TestSetDrunEntries(entries) => {
            state.drun.entries = entries;
            state.drun.scanned = true;
            state.drun.refresh_matches();
        }
    }
    Ok(())
}
