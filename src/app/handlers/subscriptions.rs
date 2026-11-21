//! Runtime IPC, system telemetry, and timer subscriptions.

use std::time::Duration;

use waywi::prelude::*;

use crate::app::ipc::{CONTROL_SOCKET_NAME, parse_ipc_command};
use crate::app::state::{Msg, State};

pub fn build_subscriptions(state: &State) -> Subscriptions<Msg> {
    let mut subs = Subscriptions::new();
    subs.push(Control::socket(CONTROL_SOCKET_NAME, |cmd| {
        parse_ipc_command(&cmd)
    }));
    subs.push(Hyprland::subscribe(Msg::HyprEvent));
    subs.push(Dbus::tray(|ev| Msg::TrayEvent(Box::new(ev))));
    subs.push(Dbus::notifications_with_capabilities(
        NotificationCapabilities::default()
            | NotificationCapabilities::PERSISTENCE
            | NotificationCapabilities::SOUND,
        |ev| Msg::NotificationEvent(Box::new(ev)),
    ));
    subs.push(Dbus::mpris(|ev| Msg::MprisEvent(Box::new(ev))));
    subs.push(Dbus::action_results(Msg::DbusActionOutcome));
    subs.push(Audio::subscribe(Msg::AudioUpdated));
    subs.push(System::cpu().every(Duration::from_secs(1), Msg::CpuSampled));
    subs.push(System::gpu().every(Duration::from_secs(2), Msg::GpuSampled));
    subs.push(System::sensors().every(Duration::from_secs(2), Msg::SensorsSampled));
    subs.push(System::uptime().every(Duration::from_secs(5), Msg::UptimeSampled));
    subs.push(System::memory().every(Duration::from_secs(1), Msg::MemorySampled));
    subs.push(System::network().every(Duration::from_secs(1), Msg::NetworkSampled));
    subs.push(System::disk().every(Duration::from_secs(30), Msg::DiskSampled));
    subs.push(System::time().every(Duration::from_secs(1), Msg::TimeTick));
    subs.push(Dbus::network_manager(Msg::NetworkManagerUpdated));
    subs.push(Dbus::bluetooth(Msg::BluetoothUpdated));
    subs.push(Dbus::bluetooth_agent(|ev| {
        Msg::BluetoothAgentEvent(Box::new(ev))
    }));
    subs.push(Dbus::udisks(Msg::UDisksUpdated));

    if !state.startup_hydrated {
        subs.push(Timer::once(
            "startup-hydrate",
            Duration::from_millis(1),
            Msg::StartupHydrate,
        ));
    }
    if state.scroller.needs_scroll() {
        subs.push(Timer::every(
            "window-scroll-tick",
            Duration::from_millis(180),
            Msg::ScrollTick,
        ));
    }
    if state.is_animating_drawer() {
        subs.push(Timer::every(
            "drawer-anim",
            Duration::from_millis(16),
            Msg::DrawerTick,
        ));
    }
    if state.is_animating_reels() {
        subs.push(Timer::every(
            "reel-anim",
            Duration::from_millis(16),
            Msg::ReelAnimTick,
        ));
    }
    if state.drawer_leave_deadline.is_some() {
        subs.push(Timer::once(
            "drawer-leave-grace",
            Duration::from_millis(125),
            Msg::DrawerTimeoutCheck,
        ));
    }
    if state.has_attention_tray() {
        subs.push(Timer::every(
            "tray-blink",
            Duration::from_millis(500),
            Msg::TrayBlinkTick,
        ));
    }
    if state.notifications.active_ticker.is_some() {
        subs.push(Timer::every(
            "notif-ticker-tick",
            Duration::from_millis(28),
            Msg::NotifTickerTick,
        ));
    }
    if state.notifications.is_animating_center() {
        subs.push(Timer::every(
            "notif-center-anim",
            Duration::from_millis(16),
            Msg::NotifCenterAnimTick,
        ));
    }
    if state.notifications.center_target && !state.notifications.items.is_empty() {
        subs.push(Timer::every(
            "notif-center-marquee",
            Duration::from_millis(200),
            Msg::NotifCenterMarqueeTick,
        ));
    }
    if state.mpris.has_active_session() || state.mpris.popup_target {
        subs.push(Timer::every(
            "mpris-tick",
            Duration::from_millis(200),
            Msg::MprisTick,
        ));
    }
    if state.mpris.is_animating_popup() {
        subs.push(Timer::every(
            "mpris-popup-anim",
            Duration::from_millis(16),
            Msg::MprisPopupAnimTick,
        ));
    }
    if state.bt_ui.is_animating_popup() {
        subs.push(Timer::every(
            "bt-popup-anim",
            Duration::from_millis(16),
            Msg::BtPopupAnimTick,
        ));
    }
    if state.bt_ui.has_pending_ops() {
        subs.push(Timer::every(
            "bt-pending-tick",
            Duration::from_millis(280),
            Msg::BtPendingTick,
        ));
    }
    if state.disk_ui.is_animating_popup() {
        subs.push(Timer::every(
            "disk-popup-anim",
            Duration::from_millis(16),
            Msg::DiskPopupAnimTick,
        ));
    }
    if state.disk_ui.has_pending_ops() {
        subs.push(Timer::every(
            "disk-pending-tick",
            Duration::from_millis(280),
            Msg::DiskPendingTick,
        ));
    }
    if state.disk_ui.popup_target {
        subs.push(Timer::every(
            "disk-marquee",
            Duration::from_millis(200),
            Msg::DiskMarqueeTick,
        ));
    }
    if state.audio_ui.is_animating_popup() {
        subs.push(Timer::every(
            "mixer-popup-anim",
            Duration::from_millis(16),
            Msg::MixerPopupAnimTick,
        ));
    }
    if state.audio_ui.popup_target {
        subs.push(Timer::every(
            "mixer-marquee",
            Duration::from_millis(200),
            Msg::MixerMarqueeTick,
        ));
    }
    if state.drun.is_animating() {
        subs.push(Timer::every(
            "drun-anim",
            Duration::from_millis(16),
            Msg::DrunAnimTick,
        ));
    }
    if state.drun.target && !state.drun.match_indices.is_empty() {
        subs.push(Timer::every(
            "drun-marquee",
            Duration::from_millis(200),
            Msg::DrunMarqueeTick,
        ));
    }
    subs
}
