//! Application message types.

use waywi::prelude::*;
use waywi::system::{
    CpuSnapshot, DiskSnapshot, GpuSnapshot, MemorySnapshot, NetworkSnapshot, TimeSnapshot,
};

use super::bluetooth::BtDeviceMenuAction;
use super::disk::DiskMenuAction;
use super::ipc::{IpcAction, IpcTarget};
use super::mpris::MprisControlAction;
#[cfg(test)]
use super::types::TrayMenuPopup;
use super::types::{AudioControlTarget, HoverTarget};

#[derive(Clone)]
pub enum Msg {
    SwitchWorkspace(WorkspaceId),
    HyprEvent(HyprlandEvent),
    ScrollTick,
    ToggleDrun,
    CloseDrun,
    DrunKey(KeyEvent),
    DrunIme(ImeEvent),
    DrunLaunchIndex(usize),
    DrunScroll(ScrollEvent),
    DrunAnimTick,
    DrunMarqueeTick,
    IpcControl {
        action: IpcAction,
        target: IpcTarget,
    },
    DbusActionOutcome(DbusActionResult),
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
    NotifCenterActionClick {
        id: u32,
        action_key: String,
    },
    NotifActionScroll {
        id: u32,
        sc: ScrollEvent,
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
    BtToggleDiscoverable,
    BtDevicePointerDown {
        path: String,
        ev: PointerEvent,
    },
    BtDeviceMenuAction {
        path: String,
        action: BtDeviceMenuAction,
    },
    BtAgentDecision {
        request_id: u64,
        accept: bool,
    },
    BtPopupScroll(ScrollEvent),
    BtPopupAnimTick,
    BtPendingTick,
    CloseBtPopup,
    UDisksUpdated(UDisksSnapshot),
    DiskBarPointerDown(PointerEvent),
    DiskRefresh,
    DiskToggleSystem,
    DiskDevicePointerDown {
        path: String,
        ev: PointerEvent,
    },
    DiskDeviceMenuAction {
        path: String,
        action: DiskMenuAction,
    },
    DiskUnlockKey(KeyEvent),
    DiskUnlockIme(ImeEvent),
    DiskUnlockSubmit,
    DiskUnlockCancel,
    DiskPopupScroll(ScrollEvent),
    DiskPopupAnimTick,
    DiskPendingTick,
    DiskMarqueeTick,
    CloseDiskPopup,
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
    MixerBarPointerDown(PointerEvent),
    MixerPopupAnimTick,
    MixerMarqueeTick,
    CloseMixerPopup,
    MixerSetTarget(AudioControlTarget),
    MixerMasterScroll(ScrollEvent),
    MixerMasterSeekClick(PointerEvent),
    MixerStreamScroll {
        id: u32,
        sc: ScrollEvent,
    },
    MixerStreamSeekClick {
        id: u32,
        ev: PointerEvent,
    },
    MixerCaptureScroll {
        id: u32,
        sc: ScrollEvent,
    },
    MixerCaptureSeekClick {
        id: u32,
        ev: PointerEvent,
    },
    CpuSampled(CpuSnapshot),
    GpuSampled(GpuSnapshot),
    SensorsSampled(Vec<waywi::system::hardware::ThermalSensor>),
    UptimeSampled(waywi::system::UptimeSnapshot),
    ReelAnimTick,
    StartupHydrate,
    MemorySampled(MemorySnapshot),
    DiskSampled(DiskSnapshot),
    NetworkSampled(NetworkSnapshot),
    NetworkManagerUpdated(NetworkManagerSnapshot),
    BluetoothUpdated(BluetoothSnapshot),
    TimeTick(TimeSnapshot),
    ClockPointerDown(PointerEvent),
    ClockScroll(ScrollEvent),
    Hover(HoverTarget),
    HoverAnchorX(f32),
    Leave(HoverTarget),
    DrawerTick,
    DrawerTimeoutCheck,
    SpawnBtop,
    #[cfg(test)]
    TestSetWorkspaces(Vec<WorkspaceInfo>, i32),
    #[cfg(test)]
    TestSetTrayMenu(TrayMenuPopup),
    #[cfg(test)]
    TestSetDrunEntries(Vec<DesktopEntry>),
}
