//! Shared UI and tray data types for application state.

use waywi::prelude::*;

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
    ActiveWs,
    InactiveWs(i32),
    WindowScroll,
    Notifications,
    NotifCenterItem(u32),
    NotifAction { id: u32, action_idx: usize },
    NotifCenterDnd,
    NotifCenterClear,
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
    DrunEntry(usize),
    MixerOutput,
    MixerInput,
    MixerMaster,
    TrayItem(usize),
    TrayMenuItem(i32),
    TrayMenuClose,
    Wireplumber,
    Cpu,
    Memory,
    Network,
    ExpandSign,
    Gpu,
    Temp,
    Disk,
    Bluetooth,
    Clock,
}

impl HoverTarget {
    #[must_use]
    pub const fn is_drawer(self) -> bool {
        matches!(
            self,
            Self::ExpandSign | Self::Gpu | Self::Temp | Self::Disk | Self::Bluetooth
        )
    }

    #[must_use]
    pub fn right_margin_hint(self, clock_alt: bool, tray_count: usize) -> f32 {
        let clock_shift = if clock_alt { 148.0 } else { 0.0 };
        match self {
            Self::Clock => 8.0,
            Self::Gpu | Self::Temp | Self::Disk | Self::Bluetooth | Self::ExpandSign => {
                78.0 + clock_shift
            }
            Self::Network => 145.0 + clock_shift,
            Self::Memory => 205.0 + clock_shift,
            Self::Cpu => 255.0 + clock_shift,
            Self::Wireplumber => 305.0 + clock_shift,
            Self::TrayItem(idx) => {
                let tail_items = tray_count.saturating_sub(idx + 1) as f32;
                355.0 + clock_shift + tail_items * 26.0
            }
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
