//! Core application state, hover tracking, and drawer animation.

use std::time::{Duration, Instant, SystemTime};

use waywi::desktop::icons::XdgIconLookup;
use waywi::prelude::*;
use waywi::system::{
    CpuSnapshot, DiskSnapshot, GpuSnapshot, LocalDateTime, MemorySnapshot, NetworkSnapshot,
    TimeFormat, shift_year_month,
};

use super::asset_lru::PathAssetCache;
pub use super::audio::AudioUiState;
pub use super::bluetooth::BluetoothUiState;
pub use super::drun::DrunState;
pub use super::mpris::MprisState;
pub use super::msg::Msg;
pub use super::notifications::NotificationStore;
pub use super::scroller::WindowScroller;
pub use super::telemetry::CpuPowerTracker;
pub use super::tray_cache::{resolve_tray_icon, upsert_tray_entry_into};
pub use super::types::{AudioControlTarget, HoverTarget, TrayEntry, TrayMenuPopup};

pub const DRAWER_ANIM_STEP: f32 = 16.0 / 300.0;

pub struct State {
    pub logo_asset: Option<ImageAsset>,
    pub icon_lookup: XdgIconLookup,
    pub path_asset_cache: PathAssetCache,
    pub bar_width: f32,
    pub screen_height: f32,

    pub workspaces: Vec<WorkspaceInfo>,
    pub active_ws_id: i32,
    pub scroller: WindowScroller,
    pub drun: DrunState,
    pub notifications: NotificationStore,
    pub mpris: MprisState,

    pub tray_items: Vec<TrayEntry>,
    pub tray_menu: Option<TrayMenuPopup>,
    pub blink_phase: bool,

    pub audio: Option<AudioSnapshot>,
    pub audio_target: AudioControlTarget,
    pub audio_ui: AudioUiState,

    pub cpu: Option<CpuSnapshot>,
    pub cpu_temp_celsius: Option<f32>,
    pub cpu_power_watts: Option<f32>,
    pub cpu_power_tracker: CpuPowerTracker,
    pub gpu: Option<GpuSnapshot>,
    pub memory: Option<MemorySnapshot>,

    pub network: Option<NetworkSnapshot>,
    pub nm: Option<NetworkManagerSnapshot>,

    pub disk_used_gb: f64,
    pub disk_free_gb: f64,
    pub disk_total_gb: f64,
    pub disk_pct_used: f32,
    pub disk_pct_free: f32,

    pub bluetooth: BluetoothSnapshot,
    pub bt_ui: BluetoothUiState,

    pub now: LocalDateTime,
    pub clock_alt: bool,
    pub cal_year_mode: bool,
    pub cal_month_offset: i32,

    pub hovered: HoverTarget,
    pub drawer_target: bool,
    pub drawer_progress: f32,
    pub drawer_leave_deadline: Option<Instant>,
}

impl State {
    #[must_use]
    pub fn has_attention_tray(&self) -> bool {
        self.tray_items
            .iter()
            .any(|e| e.snapshot.status == TrayStatus::NeedsAttention)
    }

    #[must_use]
    pub fn is_animating_drawer(&self) -> bool {
        let target = if self.drawer_target { 1.0 } else { 0.0 };
        (self.drawer_progress - target).abs() > 0.001
    }

    pub fn update_hover(&mut self, target: HoverTarget, entered: bool) {
        if entered {
            self.hovered = target;
            if target.is_drawer() {
                self.drawer_target = true;
                self.drawer_leave_deadline = None;
            } else {
                self.drawer_target = false;
                self.drawer_leave_deadline = None;
            }
        } else if self.hovered == target {
            self.hovered = HoverTarget::None;
            if target.is_drawer() {
                // Brief grace avoids flicker when sliding between `expand_sign`, `disk`, and `bluetooth`.
                self.drawer_leave_deadline = Some(Instant::now() + Duration::from_millis(120));
            }
        }
    }

    pub fn step_drawer_animation(&mut self) {
        let target = if self.drawer_target { 1.0_f32 } else { 0.0_f32 };
        if (self.drawer_progress - target).abs() <= DRAWER_ANIM_STEP {
            self.drawer_progress = target;
        } else if self.drawer_progress < target {
            self.drawer_progress = (self.drawer_progress + DRAWER_ANIM_STEP).min(1.0);
        } else {
            self.drawer_progress = (self.drawer_progress - DRAWER_ANIM_STEP).max(0.0);
        }
    }

    pub fn expire_drawer_leave(&mut self) {
        self.drawer_leave_deadline = None;
        if !self.hovered.is_drawer() {
            self.drawer_target = false;
        }
    }

    pub fn apply_disk_snapshot(&mut self, snap: &DiskSnapshot) {
        let root = snap
            .mounts
            .iter()
            .find(|m| m.mount_point.as_os_str() == "/")
            .or_else(|| snap.mounts.first());
        if let Some(m) = root {
            const GB: f64 = 1_000_000_000.0;
            let total_b = m.total.as_bytes() as f64;
            let used_b = m.used.as_bytes() as f64;
            let free_b = m.free.as_bytes() as f64;
            self.disk_total_gb = total_b / GB;
            self.disk_used_gb = used_b / GB;
            self.disk_free_gb = free_b / GB;
            if total_b > 0.0 {
                self.disk_pct_used = ((used_b / total_b) * 100.0) as f32;
                self.disk_pct_free = ((free_b / total_b) * 100.0) as f32;
            }
        }
    }

    pub fn update_clock_from_system(&mut self, wall: SystemTime) {
        self.now = LocalDateTime::from_system_time(wall);
    }

    #[must_use]
    pub fn clock_display(&self) -> String {
        if self.clock_alt {
            self.now.format_long_date()
        } else {
            self.now.format_hm(TimeFormat::H24)
        }
    }

    #[must_use]
    pub fn calendar_target_ym(&self) -> (i32, u8) {
        shift_year_month(self.now.year, self.now.month, self.cal_month_offset)
    }

    pub fn upsert_tray_entry(&mut self, new_entry: TrayEntry) {
        upsert_tray_entry_into(&mut self.tray_items, new_entry);
    }

    pub fn remove_tray_service(&mut self, service: &str) {
        self.tray_items.retain(|e| {
            e.snapshot.service != service
                && (e.snapshot.bus_name.is_empty() || e.snapshot.bus_name != service)
        });
        if self
            .tray_menu
            .as_ref()
            .is_some_and(|m| m.service == service || m.bus_name == service)
        {
            self.tray_menu = None;
        }
    }
}
