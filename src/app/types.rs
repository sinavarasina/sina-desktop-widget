//! Shared UI and tray data types for application state.

use waywi::prelude::*;
use waywi::dbus::TrayMenuItem;

use super::annotate::AnnotateTool;
use super::config::RightMetricId;
use super::quick_menu::PowerAction;
use super::screenshot::ScreenshotMode;
use super::telemetry_graph::TelemetryGraphKind;
use super::toolbox::ToolboxTool;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AudioControlTarget {
    #[default]
    Sink,
    Source,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoverTarget {
    None,
    Logo,
    SysInfo,
    ActiveWs,
    InactiveWs(i32),
    NamedWs(usize),
    SpecialWs(usize),
    WindowScroll,
    WindowInfo,
    Notifications,
    NotifCenterItem(u32),
    NotifAction { id: u32, action_idx: usize },
    NotifCenterDnd,
    NotifCenterClear,
    PrivacyStatus,
    VisualizerBar,
    ClipboardBar,
    CaffeineBadge,
    ActiveToolBadge,
    ClipboardItem(u64),
    ClipboardOpen(u64),
    ClipboardPin(u64),
    ClipboardDelete(u64),
    ClipboardCaffeineToggle,
    ClipboardClear,
    MprisBar,
    MprisPrev,
    MprisPlayPause,
    MprisStop,
    MprisNext,
    MprisShuffle,
    MprisLoop,
    MprisSeek,
    BtPower,
    BtScan,
    BtDiscoverable,
    BtDevice(usize),
    BtMenuConnect,
    BtMenuTrust,
    BtMenuForget,
    DiskRefresh,
    DiskSystemToggle,
    DiskUnlockSubmit,
    DiskUnlockCancel,
    DiskDevice(usize),
    DiskMenuMount,
    DiskMenuForceUnmount,
    DiskMenuCrypt,
    DiskMenuEject,
    DiskMenuPowerOff,
    DrunEntry(usize),
    MixerOutput,
    MixerInput,
    MixerMaster,
    MixerRoutingToggle,
    MixerBalanceBar,
    MixerDevice(u32),
    MixerPort(usize),
    MixerProfileToggle,
    MixerProfile(usize),
    MixerStreamRouteToggle(u32),
    MixerStreamRouteDevice(u32, u32),
    TrayItem(usize),
    TrayMenuItem(i32),
    TrayMenuClose,
    Wireplumber,
    Cpu,
    Memory,
    Network,
    ExpandSign,
    Gpu,
    Disk,
    Bluetooth,
    TelemetryTab(TelemetryGraphKind),
    ClockMenu,
    Clock,
    QuickActionIndicator,
    QuickSettingsToggle,
    QuickPowerToggle,
    QuickToolboxToggle,
    ToolboxItem(ToolboxTool),
    QuickPowerAction(PowerAction),
    QuickPowerCancel,
    QuickSprite,
    ZoomHudMinus,
    ZoomHudPlus,
    ZoomHudFilter,
    ZoomHudLive,
    ZoomHudReset,
    ZoomHudRecapture,
    ZoomHudClose,
    ScreenshotModeBtn(ScreenshotMode),
    ScreenshotCopyToggle,
    ScreenshotSaveToggle,
    ScreenshotClose,
    ScreenshotPreviewOpen,
    ScreenshotPreviewFolder,
    ScreenshotPreviewCopy,
    ScreenshotPreviewDelete,
    ScreenshotPreviewClose,
    AnnotateTool(AnnotateTool),
    AnnotateColor(usize),
    AnnotateSize(usize),
    AnnotateUndo,
    AnnotateRedo,
    AnnotateClearWs,
    AnnotateClearAll,
    AnnotatePin,
    AnnotateClose,
    SettingsTab(usize),
    SettingsRow(usize),
    SettingsDropdownOpt(usize),
    SettingsStepperMinus(usize),
    SettingsStepperPlus(usize),
    SettingsAction(usize),
    SettingsPathApply,
    SettingsPathClear,
    SettingsMetricPlace(usize),
    SettingsMetricUp(usize),
    SettingsMetricDown(usize),
}

impl HoverTarget {
    #[must_use]
    pub const fn right_metric_id(self) -> Option<RightMetricId> {
        match self {
            Self::Wireplumber => Some(RightMetricId::Audio),
            Self::Cpu => Some(RightMetricId::Cpu),
            Self::Memory => Some(RightMetricId::Memory),
            Self::Network => Some(RightMetricId::Network),
            Self::Gpu => Some(RightMetricId::Gpu),
            Self::Disk => Some(RightMetricId::Disk),
            Self::Bluetooth => Some(RightMetricId::Bluetooth),
            _ => None,
        }
    }

    #[must_use]
    pub const fn is_clock_group(self) -> bool {
        matches!(self, Self::Clock | Self::ClockMenu)
    }

    #[must_use]
    pub fn right_margin_hint(self, clock_alt: bool, tray_count: usize) -> f32 {
        let clock_shift = if clock_alt { 148.0 } else { 0.0 };
        match self {
            Self::Clock => 8.0,
            Self::Gpu | Self::Disk | Self::Bluetooth | Self::ExpandSign => 78.0 + clock_shift,
            Self::Network => 145.0 + clock_shift,
            Self::Memory => 205.0 + clock_shift,
            Self::Cpu => 255.0 + clock_shift,
            Self::Wireplumber => 305.0 + clock_shift,
            Self::TrayItem(idx) => {
                let tail_items = tray_count.saturating_sub(idx + 1) as f32;
                355.0 + clock_shift + tail_items * 26.0
            }
            Self::ClipboardBar | Self::CaffeineBadge => {
                395.0 + clock_shift + (tray_count as f32) * 26.0
            }
            Self::PrivacyStatus | Self::ActiveToolBadge => {
                475.0 + clock_shift + (tray_count as f32) * 26.0
            }
            Self::VisualizerBar => 540.0 + clock_shift + (tray_count as f32) * 26.0,
            _ => 8.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayEntry {
    pub snapshot: TrayItemSnapshot,
    pub icon: Option<ImageAsset>,
    pub icon_signature: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrayMenuPopup {
    pub service: String,
    pub bus_name: String,
    pub menu_path: String,
    pub anchor_x: f32,
    pub items: Vec<TrayMenuItem>,
}
