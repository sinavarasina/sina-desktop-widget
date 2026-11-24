//! Core application state, hover tracking, and drawer animation.

use std::sync::Arc;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

use waywi::desktop::icons::XdgIconLookup;
use waywi::prelude::*;
use waywi::dbus::TrayStatus;
use waywi::system::{
    CpuSnapshot, DiskMount, DiskSnapshot, GpuSnapshot, LocalDateTime, MemorySnapshot,
    NetworkSnapshot, TimeFormat, shift_year_month,
};

use super::asset_lru::PathAssetCache;
pub use super::audio::AudioUiState;
pub use super::bluetooth::BluetoothUiState;
pub use super::booster::ForegroundBooster;
pub use super::config::{SettingsUiState, WidgetConfig};
pub use super::desktop_sprite::DesktopSpriteState;
pub use super::disk::DiskUiState;
pub use super::drun::DrunState;
pub use super::mpris::MprisState;
pub use super::msg::Msg;
pub use super::notifications::NotificationStore;
pub use super::quick_menu::QuickMenuState;
pub use super::scroller::WindowScroller;
pub use super::sounds::{SoundCue, SoundEngine};
pub use super::sysinfo::SysInfoState;
pub use super::telemetry::CpuPowerTracker;
pub use super::tray_cache::resolve_tray_icon;
use super::tray_cache::upsert_tray_entry_into;
pub use super::types::{AudioControlTarget, HoverTarget, TrayEntry, TrayMenuPopup};
pub use super::window_info::WindowInfoState;

pub const DRAWER_ANIM_STEP: f32 = 16.0 / 300.0;

pub struct State {
    pub config: WidgetConfig,
    pub config_path: PathBuf,
    pub settings_ui: SettingsUiState,
    pub logo_asset: Option<ImageAsset>,
    pub wallpaper_asset: Option<ImageAsset>,
    pub desktop_sprite: DesktopSpriteState,
    pub sounds: SoundEngine,
    pub icon_lookup: XdgIconLookup,
    pub path_asset_cache: PathAssetCache,
    pub bar_width: f32,
    pub screen_height: f32,

    pub workspaces: Vec<WorkspaceInfo>,
    pub active_ws_id: i32,
    pub prev_ws_id: i32,
    pub active_special: Option<String>,
    pub sysinfo: SysInfoState,
    pub scroller: WindowScroller,
    pub window_info: WindowInfoState,
    pub booster: ForegroundBooster,
    pub drun: DrunState,
    pub notifications: NotificationStore,
    pub mpris: MprisState,

    pub tray_items: Vec<TrayEntry>,
    pub tray_menu: Option<TrayMenuPopup>,
    pub blink_phase: bool,

    pub audio: Option<Arc<AudioSnapshot>>,
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
    pub disk_mounts: Vec<DiskMount>,
    pub udisks: UDisksSnapshot,
    pub disk_ui: DiskUiState,

    pub bluetooth: BluetoothSnapshot,
    pub bt_ui: BluetoothUiState,

    pub now: LocalDateTime,
    pub clock_alt: bool,
    pub calendar_open: bool,
    pub cal_year_mode: bool,
    pub cal_month_offset: i32,
    pub clock_menu_target: bool,
    pub clock_menu_progress: f32,
    pub clock_menu_leave_deadline: Option<Instant>,
    pub quick_menu: QuickMenuState,

    pub hovered: HoverTarget,
    pub hover_anchor_x: Option<f32>,
    pub startup_hydrated: bool,
    pub drawer_target: bool,
    pub drawer_progress: f32,
    pub drawer_leave_deadline: Option<Instant>,
}

impl State {
    /// Plays a standard XDG Sound Theme cue via the O(1)-cached [`SoundEngine`], honoring DND.
    pub fn play_sound(&mut self, cue: SoundCue, ctx: &EffectCtx<'_, crate::app::msg::Msg>) {
        self.sounds.play(cue, self.notifications.dnd, ctx);
    }

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

    #[must_use]
    pub fn is_animating_clock_menu(&self) -> bool {
        let target = if self.clock_menu_target { 1.0 } else { 0.0 };
        (self.clock_menu_progress - target).abs() > 0.001
    }

    #[must_use]
    pub const fn is_animating_reels(&self) -> bool {
        self.sysinfo.is_animating() || self.window_info.is_animating()
    }

    #[must_use]
    pub fn is_drawer_target(&self, target: HoverTarget) -> bool {
        if self.config.widgets.drawer_metrics().is_empty() {
            return false;
        }
        target == HoverTarget::ExpandSign
            || target
                .right_metric_id()
                .is_some_and(|id| self.config.widgets.is_in_drawer(id))
    }

    pub fn update_hover(&mut self, target: HoverTarget, entered: bool) {
        let target_in_drawer = self.is_drawer_target(target);
        if entered {
            if self.hovered != target {
                self.hover_anchor_x = None;
            }
            self.hovered = target;
            if target_in_drawer {
                self.drawer_target = true;
                self.drawer_leave_deadline = None;
            } else {
                self.drawer_target = false;
                self.drawer_leave_deadline = None;
            }
            if target.is_clock_group() {
                self.clock_menu_target = true;
                self.clock_menu_leave_deadline = None;
            } else if !self.quick_menu.popup_target {
                self.clock_menu_target = false;
                self.clock_menu_leave_deadline = None;
            }
        } else if self.hovered == target {
            self.hovered = HoverTarget::None;
            self.hover_anchor_x = None;
            if target_in_drawer {
                // Brief grace avoids flicker when sliding between `expand_sign` and drawer modules.
                self.drawer_leave_deadline = Some(Instant::now() + Duration::from_millis(120));
            }
            if target.is_clock_group() && !self.quick_menu.popup_target {
                // Brief grace avoids flicker when sliding from `Clock` into `ClockMenu`.
                self.clock_menu_leave_deadline = Some(Instant::now() + Duration::from_millis(140));
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
        if !self.is_drawer_target(self.hovered) {
            self.drawer_target = false;
        }
    }

    pub fn step_clock_menu_animation(&mut self) {
        const CLOCK_MENU_STEP: f32 = 16.0 / 180.0;
        let target = if self.clock_menu_target {
            1.0_f32
        } else {
            0.0_f32
        };
        if (self.clock_menu_progress - target).abs() <= CLOCK_MENU_STEP {
            self.clock_menu_progress = target;
        } else if self.clock_menu_progress < target {
            self.clock_menu_progress = (self.clock_menu_progress + CLOCK_MENU_STEP).min(1.0);
        } else {
            self.clock_menu_progress = (self.clock_menu_progress - CLOCK_MENU_STEP).max(0.0);
        }
    }

    pub fn expire_clock_menu_leave(&mut self) {
        self.clock_menu_leave_deadline = None;
        if !self.hovered.is_clock_group() && !self.quick_menu.popup_target {
            self.clock_menu_target = false;
        }
    }

    pub fn apply_disk_snapshot(&mut self, snap: &DiskSnapshot) {
        self.apply_disk_mounts(snap.mounts.clone());
    }

    pub fn apply_disk_mounts(&mut self, mounts: Vec<DiskMount>) {
        self.disk_mounts = mounts;
        if self.disk_mounts.is_empty() {
            self.disk_ui.selected_mount_idx = 0;
            return;
        }
        if self.disk_ui.selected_mount_idx >= self.disk_mounts.len() {
            self.disk_ui.selected_mount_idx = 0;
        }
        self.sync_selected_mount_metrics();
    }

    pub fn cycle_selected_mount(&mut self) {
        if self.disk_mounts.len() <= 1 {
            return;
        }
        self.disk_ui.selected_mount_idx =
            (self.disk_ui.selected_mount_idx + 1) % self.disk_mounts.len();
        self.sync_selected_mount_metrics();
    }

    pub fn sync_selected_mount_metrics(&mut self) {
        let selected = self
            .disk_mounts
            .get(self.disk_ui.selected_mount_idx)
            .or_else(|| {
                self.disk_mounts
                    .iter()
                    .find(|m| m.mount_point.as_os_str() == "/")
            })
            .or_else(|| self.disk_mounts.first());

        if let Some(m) = selected {
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
    pub const fn time_format(&self) -> TimeFormat {
        if self.config.general.clock_24h {
            TimeFormat::H24
        } else {
            TimeFormat::H12
        }
    }

    #[must_use]
    pub fn clock_display(&self) -> String {
        if self.clock_alt {
            self.now.format_long_date()
        } else {
            self.now.format_hm(self.time_format())
        }
    }

    pub fn save_config(&self) {
        let _ = self.config.save_to_path(&self.config_path);
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
