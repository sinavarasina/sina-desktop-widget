//! Application message types.

use waywi::prelude::*;
use waywi::system::{
    CpuSnapshot, DiskSnapshot, GpuSnapshot, MemorySnapshot, NetworkSnapshot, TimeSnapshot,
};

use super::types::{HoverTarget, TrayMenuPopup};

#[derive(Clone)]
pub enum Msg {
    LaunchApp {
        prog: &'static str,
        args: &'static [&'static str],
    },
    SwitchWorkspace(WorkspaceId),
    HyprEvent(HyprlandEvent),
    ScrollTick,
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
    SpawnBluetui,
    #[cfg(test)]
    TestSetWorkspaces(Vec<WorkspaceInfo>, i32),
    #[cfg(test)]
    TestSetTrayMenu(TrayMenuPopup),
}
