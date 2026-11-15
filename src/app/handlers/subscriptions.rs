//! Runtime IPC, system telemetry, and timer subscriptions.

use std::time::Duration;

use waywi::prelude::*;

use crate::app::state::{Msg, State};

pub fn build_subscriptions(state: &State) -> Subscriptions<Msg> {
    let mut subs = Subscriptions::new();
    subs.push(Hyprland::subscribe(Msg::HyprEvent));
    subs.push(Dbus::tray(|ev| Msg::TrayEvent(Box::new(ev))));
    subs.push(Dbus::notifications(|ev| Msg::NotificationEvent(Box::new(ev))));
    subs.push(Audio::subscribe(Msg::AudioUpdated));
    subs.push(System::cpu().every(Duration::from_secs(1), Msg::CpuSampled));
    subs.push(System::gpu().every(Duration::from_secs(2), Msg::GpuSampled));
    subs.push(System::memory().every(Duration::from_secs(1), Msg::MemorySampled));
    subs.push(System::network().every(Duration::from_secs(1), Msg::NetworkSampled));
    subs.push(System::disk().every(Duration::from_secs(30), Msg::DiskSampled));
    subs.push(System::time().every(Duration::from_secs(1), Msg::TimeTick));
    subs.push(Dbus::network_manager(Msg::NetworkManagerUpdated));
    subs.push(Dbus::bluetooth(Msg::BluetoothUpdated));

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
    subs
}
