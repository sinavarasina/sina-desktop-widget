//! Application message types.

use waywi::prelude::*;
use waywi::system::{
    CpuSnapshot, DiskSnapshot, GpuSnapshot, MemorySnapshot, NetworkSnapshot, TimeSnapshot,
};

use super::mpris::MprisControlAction;
use super::types::HoverTarget;
#[cfg(test)]
use super::types::TrayMenuPopup;

#[derive(Clone)]
pub enum Msg {
    LaunchApp {
        prog: &'static str,
        args: &'static [&'static str],
    },
    SwitchWorkspace(WorkspaceId),
    HyprEvent(HyprlandEvent),
    ScrollTick,
    NotificationEvent(Box<NotificationEvent>),
    NotifBarPointerDown(PointerEvent),
    NotifTickerTick,
    NotifCenterAnimTick,
    NotifCenterMarqueeTick,
    CloseNotificationCenter,
    NotifCenterScroll(ScrollEvent),
    NotifCenterCardPointerDown {
        id: u32,
        ev: PointerEvent,
    },
    NotifCenterClearAll,
    NotifCenterToggleDnd,
    MprisEvent(Box<MprisEvent>),
    MprisBarPointerDown(PointerEvent),
    MprisBarScroll(ScrollEvent),
    MprisAction(MprisControlAction),
    MprisSeekClick(PointerEvent),
    MprisTick,
    MprisPopupAnimTick,
    CloseMprisPopup,
    BluetoothAgentEvent(Box<BluetoothAgentEvent>),
    BtBarPointerDown(PointerEvent),
    BtTogglePower,
    BtToggleScan,
    BtDevicePointerDown {
        path: String,
        ev: PointerEvent,
    },
    BtAgentDecision {
        request_id: u64,
        accept: bool,
    },
    BtPopupScroll(ScrollEvent),
    BtPopupAnimTick,
    CloseBtPopup,
    Noop,
    TrayEvent(Box<TrayEvent>),
    TrayClick {
        index: usize,
        ev: PointerEvent,
    },
    TrayMenuClick {
        item_id: i32,
    },
    CloseTrayMenu,
    TrayBlinkTick,
    AudioUpdated(AudioSnapshot),
    AudioPointerDown(PointerEvent),
    AudioScroll(ScrollEvent),
    CpuSampled(CpuSnapshot),
    GpuSampled(GpuSnapshot),
    MemorySampled(MemorySnapshot),
    DiskSampled(DiskSnapshot),
    NetworkSampled(NetworkSnapshot),
    NetworkManagerUpdated(NetworkManagerSnapshot),
    BluetoothUpdated(BluetoothSnapshot),
    TimeTick(TimeSnapshot),
    ClockPointerDown(PointerEvent),
    ClockScroll(ScrollEvent),
    Hover(HoverTarget),
    Leave(HoverTarget),
    DrawerTick,
    DrawerTimeoutCheck,
    SpawnBtop,
    #[cfg(test)]
    TestSetWorkspaces(Vec<WorkspaceInfo>, i32),
    #[cfg(test)]
    TestSetTrayMenu(TrayMenuPopup),
}
