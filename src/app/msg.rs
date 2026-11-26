//! Application message types.

use std::sync::Arc;
use std::time::Instant;

use waywi::prelude::*;
use waywi::system::{
    CpuSnapshot, DiskSnapshot, GpuSnapshot, MemorySnapshot, NetworkSnapshot, TimeSnapshot,
};

use super::bluetooth::BtDeviceMenuAction;
use super::config::{
    RightMetricId, SettingStepperId, SettingToggleId, SettingsDropdown, SettingsSection,
};
use super::disk::DiskMenuAction;
use super::ipc::{IpcAction, IpcTarget};
use super::mpris::MprisControlAction;
use super::quick_menu::PowerAction;
#[cfg(test)]
use super::types::TrayMenuPopup;
use super::types::{AudioControlTarget, HoverTarget};

#[derive(Clone)]
pub enum Msg {
    /// Monitor-synchronized VSync frame tick from `waywi::animation::Animation::frames`.
    AnimationFrame(Instant),
    SwitchWorkspace(WorkspaceId),
    SwitchWorkspaceNamed(String),
    SwitchWorkspacePrevious,
    WorkspaceScroll(ScrollEvent),
    ToggleSpecialWorkspace(String),
    HyprEvent(HyprlandEvent),
    ScrollTick,
    ToggleDrun,
    CloseDrun,
    DrunKey(KeyEvent),
    DrunIme(ImeEvent),
    DrunLaunchIndex(usize),
    DrunScroll(ScrollEvent),
    DrunMarqueeTick,
    IpcControl {
        action: IpcAction,
        target: IpcTarget,
    },
    DbusActionOutcome(DbusActionResult),
    NotificationEvent(Box<NotificationEvent>),
    NotifBarPointerDown(PointerEvent),
    NotifTickerTick,
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
    DiskPendingTick,
    DiskMarqueeTick,
    CloseDiskPopup,
    Noop,
    /// Compositor user-idle notification; `Idled` schedules a `trim_memory()` sweep.
    UserIdle(IdleEvent),
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
    AudioUpdated(Arc<AudioSnapshot>),
    AudioPointerDown(PointerEvent),
    AudioScroll(ScrollEvent),
    MixerBarPointerDown(PointerEvent),
    MixerMarqueeTick,
    CloseMixerPopup,
    MixerSetTarget(AudioControlTarget),
    MixerToggleRouting,
    MixerToggleProfiles,
    MixerRoutingScroll(ScrollEvent),
    MixerSelectDevice(u32),
    MixerSelectPort(String),
    MixerSelectProfile {
        card_id: u32,
        profile_name: String,
    },
    MixerBalanceSeekClick(PointerEvent),
    MixerBalanceScroll(ScrollEvent),
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
    StartupHydrate,
    MemorySampled(MemorySnapshot),
    DiskSampled(DiskSnapshot),
    NetworkSampled(NetworkSnapshot),
    NetworkManagerUpdated(NetworkManagerSnapshot),
    BluetoothUpdated(BluetoothSnapshot),
    TimeTick(TimeSnapshot),
    ClockPointerDown(PointerEvent),
    ClockScroll(ScrollEvent),
    CloseCalendar,
    ClockMenuClick,
    ClockMenuTimeoutCheck,
    LogindEvent(LogindEvent),
    CloseQuickMenu,
    QuickMenuTogglePower,
    QuickMenuToggleSettings,
    QuickMenuPowerAction(PowerAction),
    QuickMenuPowerTick,
    QuickMenuCancelPower,
    QuickMenuSpriteClick,
    QuickMenuSpriteScroll(ScrollEvent),
    QuickMenuSpriteReactEnd,
    SettingsSelectSection(SettingsSection),
    SettingsRailScroll(ScrollEvent),
    SettingsListScroll(ScrollEvent),
    SettingsToggle(SettingToggleId),
    SettingsStep {
        id: SettingStepperId,
        delta: i32,
    },
    SettingsToggleDropdown(SettingsDropdown),
    SettingsSelectDropdown {
        dropdown: SettingsDropdown,
        index: usize,
    },
    SettingsWallpaperPathFocus,
    SettingsWallpaperPathApply,
    SettingsWallpaperPathClear,
    SettingsKey(KeyEvent),
    SettingsIme(ImeEvent),
    SettingsPreviewSound,
    SettingsResetDefaults,
    SettingsCycleMetricPlacement(RightMetricId),
    SettingsMoveMetric {
        id: RightMetricId,
        delta: i32,
    },
    DesktopSpriteBlinkTick,
    DesktopSpriteMouthTick,
    Hover(HoverTarget),
    HoverAnchorX(f32),
    Leave(HoverTarget),
    DrawerTimeoutCheck,
    SpawnBtop,
    #[cfg(test)]
    TestSetWorkspaces(Vec<WorkspaceInfo>, i32),
    #[cfg(test)]
    TestSetTrayMenu(TrayMenuPopup),
    #[cfg(test)]
    TestSetDrunEntries(Vec<DesktopEntry>),
    #[cfg(test)]
    TestLoadWallpaper,
    #[cfg(test)]
    TestLoadDesktopSprite,
}
