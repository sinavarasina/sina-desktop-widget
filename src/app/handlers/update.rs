//! Message dispatch and state transitions.

use waywi::desktop::spawn_detached;
use waywi::prelude::*;

use crate::app::audio::AudioSnapshotExt;

use crate::app::config::DesktopPoseMode;
use crate::app::state::{AudioControlTarget, Msg, SoundCue, State};

use super::audio::{
    handle_audio_updated, handle_mixer_balance_scroll, handle_mixer_balance_seek,
    handle_mixer_bar_pointer, handle_mixer_capture_scroll, handle_mixer_capture_seek_click,
    handle_mixer_master_seek_click, handle_mixer_routing_scroll, handle_mixer_select_device,
    handle_mixer_select_port, handle_mixer_select_profile, handle_mixer_set_target,
    handle_mixer_stream_scroll, handle_mixer_stream_seek_click, handle_mixer_toggle_profiles,
    handle_mixer_toggle_routing,
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
use super::ipc::{close_other_overlays, handle_ipc_control};
use super::mpris::{
    handle_mpris_action, handle_mpris_bar_pointer, handle_mpris_event, handle_mpris_scroll,
    handle_mpris_seek_click,
};
use super::notifications::{
    handle_notif_action_click, handle_notif_action_scroll, handle_notif_bar_pointer,
    handle_notif_card_pointer, handle_notif_center_scroll, handle_notification_event,
};
use super::quick_menu::{
    handle_clock_menu_click, handle_logind_event, handle_power_action, handle_power_countdown_tick,
    handle_quick_menu_toggle_power,
};
use super::settings::{
    handle_cycle_metric_placement, handle_list_scroll, handle_move_metric, handle_preview_sound,
    handle_rail_scroll, handle_reset_defaults, handle_select_dropdown, handle_select_section,
    handle_settings_ime, handle_settings_key, handle_step_setting, handle_toggle_dropdown,
    handle_toggle_setting, handle_toggle_settings, handle_wallpaper_path_apply,
    handle_wallpaper_path_clear, handle_wallpaper_path_focus,
};

/// Keeps `bar_width` and `screen_height` synchronized with the primary Wayland output across
/// monitor hotplug, OSD HDR/mode changes, and scale switches (O(1), zero allocations).
fn sync_primary_output_size(state: &mut State, ctx: &EffectCtx<'_, crate::app::msg::Msg>) {
    if let Some(primary) = ctx.outputs().first() {
        let w = primary.logical_size.width.get();
        let h = primary.logical_size.height.get();
        if w >= 320.0 && (state.bar_width - w).abs() > 0.5 {
            state.bar_width = w;
        }
        if h >= 240.0 && (state.screen_height - h).abs() > 0.5 {
            state.screen_height = h;
        }
    }
}

pub fn update_state(state: &mut State, msg: Msg, ctx: &mut EffectCtx<'_, crate::app::msg::Msg>) -> Result<()> {
    sync_primary_output_size(state, ctx);
    match msg {
        Msg::AnimationFrame(now) => state.advance_ui_animations(now, ctx),
        Msg::SpawnBtop => {
            let _ = spawn_detached("footclient", ["btop"]);
        }
        Msg::SwitchWorkspace(id) => {
            let prev_ws = state.active_ws_id;
            if let Ok(hc) = ctx.hyprland() {
                let _ = hc.switch_workspace(id);
                if let Ok(list) = hc.workspaces() {
                    state.workspaces = list;
                }
                state.active_ws_id = id.get();
            } else {
                for w in &mut state.workspaces {
                    w.focused = !w.is_special() && !w.is_named() && w.id == id;
                }
                state.active_ws_id = id.get();
            }
            if prev_ws != state.active_ws_id {
                state.prev_ws_id = prev_ws;
                if state.config.sprite.rotate_on_workspace
                    && state.config.sprite.desktop_pose == DesktopPoseMode::Auto
                {
                    state.desktop_sprite.rotate_instant(ctx);
                }
                state.play_sound(SoundCue::TabChanged, ctx);
            }
        }
        Msg::SwitchWorkspaceNamed(target) => {
            let prev_ws = state.active_ws_id;
            if let Ok(hc) = ctx.hyprland() {
                let _ = hc.switch_workspace(waywi::compositor::WorkspaceTarget::Name(&target));
                if let Ok(list) = hc.workspaces() {
                    state.workspaces = list;
                }
            } else {
                for w in &mut state.workspaces {
                    w.focused = w.is_named() && w.target_spec() == target;
                }
            }
            if prev_ws > 0 {
                state.prev_ws_id = prev_ws;
            }
            if state.config.sprite.rotate_on_workspace
                && state.config.sprite.desktop_pose == DesktopPoseMode::Auto
            {
                state.desktop_sprite.rotate_instant(ctx);
            }
            state.play_sound(SoundCue::TabChanged, ctx);
        }
        Msg::SwitchWorkspacePrevious => {
            let prev_ws = state.active_ws_id;
            if let Ok(hc) = ctx.hyprland() {
                let _ = hc.switch_workspace(waywi::compositor::WorkspaceTarget::Previous);
                if let Ok(list) = hc.workspaces() {
                    if let Some(focused) = list
                        .iter()
                        .find(|w| w.focused && !w.is_special() && w.id.get() > 0)
                    {
                        state.active_ws_id = focused.id.get();
                    }
                    state.workspaces = list;
                }
            } else if state.prev_ws_id > 0 && state.prev_ws_id != state.active_ws_id {
                let target_id = state.prev_ws_id;
                for w in &mut state.workspaces {
                    w.focused = !w.is_special() && !w.is_named() && w.id.get() == target_id;
                }
                state.active_ws_id = target_id;
            }
            if prev_ws != state.active_ws_id {
                state.prev_ws_id = prev_ws;
                if state.config.sprite.rotate_on_workspace
                    && state.config.sprite.desktop_pose == DesktopPoseMode::Auto
                {
                    state.desktop_sprite.rotate_instant(ctx);
                }
                state.play_sound(SoundCue::TabChanged, ctx);
            }
        }
        Msg::WorkspaceScroll(sc) => {
            let dir = super::events::scroll_direction_y(sc);
            if dir != 0 {
                let step = if dir > 0 { 1 } else { -1 };
                let prev_ws = state.active_ws_id;
                if let Ok(hc) = ctx.hyprland() {
                    let _ = hc.switch_workspace(waywi::compositor::WorkspaceTarget::Relative { delta: step, existing_only: true });
                    if let Ok(list) = hc.workspaces() {
                        if let Some(focused) = list
                            .iter()
                            .find(|w| w.focused && !w.is_special() && w.id.get() > 0)
                        {
                            state.active_ws_id = focused.id.get();
                        }
                        state.workspaces = list;
                    }
                } else {
                    let mut ids: Vec<i32> = state
                        .workspaces
                        .iter()
                        .filter(|w| !w.is_special() && !w.is_named() && w.id.get() > 0)
                        .map(|w| w.id.get())
                        .collect();
                    ids.sort_unstable();
                    ids.dedup();
                    if ids.len() > 1
                        && let Some(pos) = ids.iter().position(|&id| id == state.active_ws_id)
                    {
                        let next_pos = if step > 0 {
                            (pos + 1) % ids.len()
                        } else {
                            (pos + ids.len() - 1) % ids.len()
                        };
                        let next_id = ids[next_pos];
                        for w in &mut state.workspaces {
                            w.focused = !w.is_special() && !w.is_named() && w.id.get() == next_id;
                        }
                        state.active_ws_id = next_id;
                    }
                }
                if prev_ws != state.active_ws_id {
                    state.prev_ws_id = prev_ws;
                    if state.config.sprite.rotate_on_workspace
                        && state.config.sprite.desktop_pose == DesktopPoseMode::Auto
                    {
                        state.desktop_sprite.rotate_instant(ctx);
                    }
                    state.play_sound(SoundCue::TabChanged, ctx);
                }
            }
        }
        Msg::ToggleSpecialWorkspace(name) => {
            let short = name.strip_prefix("special:").unwrap_or(&name);
            let target = (!short.is_empty()).then_some(short);
            if let Ok(hc) = ctx.hyprland() {
                let _ = hc.toggle_special_workspace(target);
                if let Ok(list) = hc.workspaces() {
                    state.workspaces = list;
                }
            } else if state.active_special.as_deref() == Some(name.as_str()) {
                state.active_special = None;
            } else {
                state.active_special = Some(name);
            }
            state.play_sound(SoundCue::TabChanged, ctx);
        }
        Msg::HyprEvent(ev) => handle_hypr_event(state, ev, ctx),
        Msg::ScrollTick => state.scroller.advance(),
        Msg::ToggleDrun => handle_toggle_drun(state, ctx),
        Msg::CloseDrun => {
            if state.drun.is_open() {
                state.drun.close();
                state.play_sound(SoundCue::MenuPopdown, ctx);
            }
        }
        Msg::DrunKey(ev) => handle_drun_key(state, ev, ctx),
        Msg::DrunIme(ev) => handle_drun_ime(state, ev, ctx),
        Msg::DrunLaunchIndex(idx) => handle_drun_launch_index(state, idx),
        Msg::DrunScroll(sc) => handle_drun_scroll(state, sc, ctx),
        Msg::DrunMarqueeTick => {
            state.drun.marquee_tick = state.drun.marquee_tick.wrapping_add(1);
        }
        Msg::IpcControl { action, target } => handle_ipc_control(state, action, target, ctx),
        Msg::DbusActionOutcome(res) => handle_dbus_action_result(state, res, ctx),
        Msg::NotificationEvent(ev) => handle_notification_event(state, *ev, ctx),
        Msg::NotifBarPointerDown(ev) => handle_notif_bar_pointer(state, ev, ctx),
        Msg::NotifTickerTick => state.notifications.step_ticker(),
        Msg::NotifCenterMarqueeTick => state.notifications.advance_marquee(),
        Msg::CloseNotificationCenter => {
            if state.notifications.is_center_open() {
                state.notifications.close_center();
                state.play_sound(SoundCue::MenuPopdown, ctx);
            }
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
            let had_items = state.notifications.total_count > 0;
            state.notifications.clear_all();
            if had_items {
                state.play_sound(SoundCue::TrashEmpty, ctx);
            }
        }
        Msg::NotifCenterToggleDnd => {
            state.notifications.toggle_dnd(ctx);
            if !state.notifications.dnd {
                state.play_sound(SoundCue::ToggleOn, ctx);
            }
        }
        Msg::MprisEvent(ev) => handle_mpris_event(state, *ev, ctx),
        Msg::MprisBarPointerDown(ev) => handle_mpris_bar_pointer(state, ev, ctx),
        Msg::MprisBarScroll(sc) => handle_mpris_scroll(state, sc, ctx),
        Msg::MprisAction(action) => handle_mpris_action(state, action, ctx),
        Msg::MprisSeekClick(ev) => handle_mpris_seek_click(state, ev, ctx),
        Msg::MprisTick => state.mpris.advance_tick(),
        Msg::CloseMprisPopup => {
            if state.mpris.is_popup_open() {
                state.mpris.close_popup();
                state.play_sound(SoundCue::MenuPopdown, ctx);
            }
        }
        Msg::BluetoothAgentEvent(ev) => handle_bluetooth_agent_event(state, *ev, ctx),
        Msg::BtBarPointerDown(ev) => handle_bt_bar_pointer(state, ev, ctx),
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
        Msg::BtPendingTick => state.bt_ui.advance_pending_tick(),
        Msg::CloseBtPopup => {
            if state.bt_ui.is_popup_open() {
                state.bt_ui.close_popup();
                state.play_sound(SoundCue::MenuPopdown, ctx);
            }
        }
        Msg::UDisksUpdated(snap) => handle_udisks_updated(state, snap, ctx),
        Msg::DiskBarPointerDown(ev) => handle_disk_bar_pointer(state, ev, ctx),
        Msg::DiskRefresh => handle_disk_refresh(state, ctx),
        Msg::DiskToggleSystem => {
            state.disk_ui.toggle_show_system();
            let cue = if state.disk_ui.show_system {
                SoundCue::ToggleOn
            } else {
                SoundCue::ToggleOff
            };
            state.play_sound(cue, ctx);
        }
        Msg::DiskDevicePointerDown { path, ev } => {
            handle_disk_device_pointer(state, path, ev, ctx);
        }
        Msg::DiskDeviceMenuAction { path, action } => {
            handle_disk_device_menu_action(state, path, action, ctx);
        }
        Msg::DiskUnlockKey(ev) => handle_disk_unlock_key(state, ev, ctx),
        Msg::DiskUnlockIme(ev) => handle_disk_unlock_ime(state, ev, ctx),
        Msg::DiskUnlockSubmit => handle_disk_unlock_submit(state, ctx),
        Msg::DiskUnlockCancel => {
            state.disk_ui.close_unlock_prompt();
            state.play_sound(SoundCue::DialogCancel, ctx);
        }
        Msg::DiskPopupScroll(sc) => handle_disk_popup_scroll(state, sc),
        Msg::DiskPendingTick => state.disk_ui.advance_pending_tick(),
        Msg::DiskMarqueeTick => state.disk_ui.advance_marquee(),
        Msg::CloseDiskPopup => {
            if state.disk_ui.is_popup_open() {
                state.disk_ui.close_popup();
                state.play_sound(SoundCue::MenuPopdown, ctx);
            }
        }
        Msg::UserIdle(IdleEvent::Idled(_)) => ctx.trim_memory(),
        Msg::UserIdle(IdleEvent::Resumed(_)) => {}
        Msg::Noop => {}
        Msg::TrayEvent(ev) => handle_tray_event(state, *ev, ctx),
        Msg::TrayClick { index, ev } => handle_tray_click(state, index, ev, ctx),
        Msg::TrayMenuClick { item_id } => {
            if let Some(menu) = state.tray_menu.take()
                && let Ok(dbus) = ctx.dbus()
            {
                let _ = dbus.tray().trigger_menu_item(&menu.bus_name, &menu.menu_path, item_id);
            }
        }
        Msg::CloseTrayMenu => {
            if state.tray_menu.take().is_some() {
                state.play_sound(SoundCue::MenuPopdown, ctx);
            }
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
                state.play_sound(SoundCue::TabChanged, ctx);
            }
            Some(MouseButton::Left) => {
                if let Ok(ac) = ctx.audio() {
                    match state.audio_target {
                        AudioControlTarget::Sink => {
                            let _ = ac.toggle_mute(AudioTarget::DefaultSink);
                        }
                        AudioControlTarget::Source => {
                            let _ = ac.toggle_mute(AudioTarget::DefaultSource);
                        }
                    }
                }
                let was_muted = state
                    .audio
                    .as_ref()
                    .and_then(|a| {
                        a.target_volume(state.audio_target == AudioControlTarget::Source)
                    })
                    .is_some_and(|v| v.muted);
                let cue = if was_muted {
                    SoundCue::ToggleOn
                } else {
                    SoundCue::ToggleOff
                };
                state.play_sound(cue, ctx);
            }
            _ => {}
        },
        Msg::AudioScroll(sc) => handle_audio_scroll(state, sc, ctx),
        Msg::MixerBarPointerDown(ev) => handle_mixer_bar_pointer(state, ev, ctx),
        Msg::MixerMarqueeTick => state.audio_ui.advance_marquee(),
        Msg::CloseMixerPopup => {
            if state.audio_ui.is_popup_open() {
                state.audio_ui.close_popup();
                state.play_sound(SoundCue::MenuPopdown, ctx);
            }
        }
        Msg::MixerSetTarget(target) => handle_mixer_set_target(state, target, ctx),
        Msg::MixerToggleRouting => handle_mixer_toggle_routing(state, ctx),
        Msg::MixerToggleProfiles => handle_mixer_toggle_profiles(state, ctx),
        Msg::MixerRoutingScroll(sc) => handle_mixer_routing_scroll(state, sc),
        Msg::MixerSelectDevice(id) => handle_mixer_select_device(state, id, ctx),
        Msg::MixerSelectPort(port) => handle_mixer_select_port(state, port, ctx),
        Msg::MixerSelectProfile {
            card_id,
            profile_name,
        } => handle_mixer_select_profile(state, card_id, profile_name, ctx),
        Msg::MixerBalanceSeekClick(ev) => handle_mixer_balance_seek(state, ev, ctx),
        Msg::MixerBalanceScroll(sc) => handle_mixer_balance_scroll(state, sc, ctx),
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
        Msg::StartupHydrate => {
            if !state.startup_hydrated {
                state.startup_hydrated = true;
                #[cfg(not(test))]
                if state.config.sprite.desktop_enabled {
                    state.desktop_sprite.ensure_loaded(ctx);
                }
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
                if state.config.general.foreground_booster {
                    state.booster.on_focus_change(
                        state.window_info.cgroup.as_ref(),
                        state.memory.as_ref(),
                        ctx,
                    );
                }
                state.play_sound(SoundCue::SystemReady, ctx);
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
            state
                .sounds
                .observe_network(&snap, state.notifications.dnd, ctx);
            state.nm = Some(snap);
        }
        Msg::BluetoothUpdated(snap) => {
            state.bt_ui.clear_settled_pending(&snap);
            state
                .sounds
                .observe_bluetooth(&snap, state.notifications.dnd, ctx);
            state.bluetooth = snap;
        }
        Msg::TimeTick(snap) => {
            state.update_clock_from_system(snap.wall_clock);
            state
                .sysinfo
                .tick_second(state.cpu.as_ref(), state.memory.as_ref());
            state.window_info.tick_second(state.booster.short_badge());
            let auto_rotate = state.config.sprite.desktop_pose == DesktopPoseMode::Auto;
            state
                .desktop_sprite
                .tick_second_with(state.config.sprite.rotate_interval_secs, auto_rotate);
        }
        Msg::ClockPointerDown(ev) => match ev.button {
            Some(MouseButton::Left) => state.clock_alt = !state.clock_alt,
            Some(MouseButton::Middle) => {
                if state.calendar_open {
                    state.calendar_open = false;
                    state.play_sound(SoundCue::MenuPopdown, ctx);
                } else {
                    close_other_overlays(state, Some("calendar"));
                    state.calendar_open = true;
                    state.play_sound(SoundCue::MenuPopup, ctx);
                }
            }
            Some(MouseButton::Right) => state.cal_year_mode = !state.cal_year_mode,
            _ => {}
        },
        Msg::ClockScroll(sc) => {
            let step = scroll_direction_y(sc);
            if step != 0 {
                state.cal_month_offset = (state.cal_month_offset - step).clamp(-120, 120);
            }
        }
        Msg::CloseCalendar => {
            if state.calendar_open {
                state.calendar_open = false;
                state.play_sound(SoundCue::MenuPopdown, ctx);
            }
        }
        Msg::ClockMenuClick => handle_clock_menu_click(state, ctx),
        Msg::ClockMenuTimeoutCheck => state.expire_clock_menu_leave(),
        Msg::LogindEvent(ev) => handle_logind_event(state, ev),
        Msg::CloseQuickMenu => {
            let was_open = state.quick_menu.is_popup_open();
            state.quick_menu.close_popup();
            state.settings_ui.unfocus_inputs();
            state.settings_ui.open_dropdown = None;
            if !state.hovered.is_clock_group() {
                state.clock_menu_anim.set_open(false);
            }
            if was_open {
                state.play_sound(SoundCue::MenuPopdown, ctx);
            }
        }
        Msg::QuickMenuTogglePower => handle_quick_menu_toggle_power(state, ctx),
        Msg::QuickMenuToggleSettings => handle_toggle_settings(state, ctx),
        Msg::QuickMenuPowerAction(action) => handle_power_action(state, action, ctx),
        Msg::QuickMenuPowerTick => handle_power_countdown_tick(state, ctx),
        Msg::QuickMenuCancelPower => {
            if state.quick_menu.pending_power.is_some() {
                state.quick_menu.cancel_power_countdown();
                state.play_sound(SoundCue::DialogCancel, ctx);
            }
        }
        Msg::QuickMenuSpriteClick => state.quick_menu.interact_sprite(),
        Msg::QuickMenuSpriteScroll(sc) => {
            let delta = super::events::scroll_direction_y(sc);
            let prev = state.quick_menu.sprite_pose_idx;
            state.quick_menu.scroll_sprite_pose(delta, ctx);
            if state.quick_menu.sprite_pose_idx != prev {
                state.config.sprite.quick_menu_pose = state.quick_menu.sprite_pose_idx;
                state.save_config();
                state.play_sound(SoundCue::TabChanged, ctx);
            }
        }
        Msg::QuickMenuSpriteReactEnd => {
            state.quick_menu.mouth_reacting = false;
        }
        Msg::SettingsSelectSection(section) => handle_select_section(state, section, ctx),
        Msg::SettingsRailScroll(sc) => handle_rail_scroll(state, sc),
        Msg::SettingsListScroll(sc) => handle_list_scroll(state, sc),
        Msg::SettingsToggle(id) => handle_toggle_setting(state, id, ctx),
        Msg::SettingsStep { id, delta } => handle_step_setting(state, id, delta, ctx),
        Msg::SettingsToggleDropdown(dd) => handle_toggle_dropdown(state, dd, ctx),
        Msg::SettingsSelectDropdown { dropdown, index } => {
            handle_select_dropdown(state, dropdown, index, ctx);
        }
        Msg::SettingsWallpaperPathFocus => handle_wallpaper_path_focus(state, ctx),
        Msg::SettingsWallpaperPathApply => handle_wallpaper_path_apply(state, ctx),
        Msg::SettingsWallpaperPathClear => handle_wallpaper_path_clear(state, ctx),
        Msg::SettingsKey(ev) => handle_settings_key(state, ev, ctx),
        Msg::SettingsIme(ev) => handle_settings_ime(state, ev, ctx),
        Msg::SettingsPreviewSound => handle_preview_sound(state, ctx),
        Msg::SettingsResetDefaults => handle_reset_defaults(state, ctx),
        Msg::SettingsCycleMetricPlacement(id) => handle_cycle_metric_placement(state, id, ctx),
        Msg::SettingsMoveMetric { id, delta } => handle_move_metric(state, id, delta, ctx),
        Msg::DesktopSpriteBlinkTick => state.desktop_sprite.step_blink(),
        Msg::DesktopSpriteMouthTick => state.desktop_sprite.step_mouth(),
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
        #[cfg(test)]
        Msg::TestLoadWallpaper => {
            if state.wallpaper_asset.is_none() {
                state.wallpaper_asset = super::init::load_embedded_wallpaper(ctx);
            }
        }
        #[cfg(test)]
        Msg::TestLoadDesktopSprite => {
            state.desktop_sprite.ensure_loaded(ctx);
        }
    }
    Ok(())
}
