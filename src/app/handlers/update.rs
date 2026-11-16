//! Message dispatch and state transitions.

use waywi::desktop::spawn_detached;
use waywi::prelude::*;

use crate::app::state::{AudioControlTarget, Msg, State};

use super::bluetooth::{
    handle_bluetooth_agent_event, handle_bt_agent_decision, handle_bt_bar_pointer,
    handle_bt_device_pointer, handle_bt_popup_scroll, handle_bt_toggle_power, handle_bt_toggle_scan,
};
use super::events::{
    handle_audio_scroll, handle_hypr_event, handle_tray_click, handle_tray_event,
    scroll_direction_y,
};
use super::mpris::{
    handle_mpris_action, handle_mpris_bar_pointer, handle_mpris_event, handle_mpris_scroll,
    handle_mpris_seek_click,
};
use super::notifications::{
    handle_notif_bar_pointer, handle_notif_card_pointer, handle_notif_center_scroll,
    handle_notification_event,
};

pub fn update_state(state: &mut State, msg: Msg, ctx: &mut EffectCtx<'_>) -> Result<()> {
    match msg {
        Msg::LaunchApp { prog, args } => {
            let _ = spawn_detached(prog, args);
        }
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
        Msg::NotifCenterClearAll => {
            state.notifications.clear_all();
        }
        Msg::NotifCenterToggleDnd => {
            state.notifications.toggle_dnd();
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
        Msg::BtDevicePointerDown { path, ev } => handle_bt_device_pointer(state, path, ev, ctx),
        Msg::BtAgentDecision { request_id, accept } => {
            handle_bt_agent_decision(state, request_id, accept, ctx);
        }
        Msg::BtPopupScroll(sc) => handle_bt_popup_scroll(state, sc),
        Msg::BtPopupAnimTick => state.bt_ui.step_popup_animation(),
        Msg::CloseBtPopup => state.bt_ui.close_popup(),
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
        Msg::AudioUpdated(snap) => {
            state.audio = Some(snap);
        }
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
        Msg::CpuSampled(snap) => {
            state.cpu = Some(snap);
        }
        Msg::GpuSampled(snap) => {
            state.gpu = Some(snap);
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
            state.bluetooth = snap;
        }
        Msg::TimeTick(snap) => {
            state.update_clock_from_system(snap.wall_clock);
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
    }
    Ok(())
}
