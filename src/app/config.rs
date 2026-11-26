//! Persistent zero-dependency TOML configuration (`~/.config/sina-desktop-widget/config.toml`)
//! and Quick Menu Settings UI state.

use std::fs;
use std::path::{Path, PathBuf};

use waywi::prelude::*;

use crate::theme::font_ui;

pub const CONFIG_FILE_NAME: &str = "config.toml";

/// Wallpaper scaling mode mapped directly to `waywi::ImageFit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum WallpaperFit {
    #[default]
    Cover,
    Contain,
    Fill,
}

impl WallpaperFit {
    pub const ALL: [Self; 3] = [Self::Cover, Self::Contain, Self::Fill];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cover => "cover",
            Self::Contain => "contain",
            Self::Fill => "fill",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Cover => "Cover",
            Self::Contain => "Contain",
            Self::Fill => "Fill",
        }
    }

    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "cover" => Some(Self::Cover),
            "contain" => Some(Self::Contain),
            "fill" => Some(Self::Fill),
            _ => None,
        }
    }

    #[must_use]
    pub const fn to_image_fit(self) -> ImageFit {
        match self {
            Self::Cover => ImageFit::Cover,
            Self::Contain => ImageFit::Contain,
            Self::Fill => ImageFit::Fill,
        }
    }

    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Cover => 0,
            Self::Contain => 1,
            Self::Fill => 2,
        }
    }

    #[must_use]
    pub const fn from_index(idx: usize) -> Self {
        match idx {
            1 => Self::Contain,
            2 => Self::Fill,
            _ => Self::Cover,
        }
    }
}

/// Desktop Yuki pose mode: automatic rotation or pinned to one of the 6 embedded poses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DesktopPoseMode {
    #[default]
    Auto,
    Pinned(usize),
}

impl DesktopPoseMode {
    pub const LABELS: [&'static str; 7] = [
        "Auto Rotate",
        "Standing",
        "Glasses",
        "Side Look",
        "Reading Book",
        "Gloom",
        "Gloom Glasses",
    ];

    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Auto => 0,
            Self::Pinned(i) => (i % 6) + 1,
        }
    }

    #[must_use]
    pub const fn from_index(idx: usize) -> Self {
        match idx {
            0 => Self::Auto,
            n => Self::Pinned((n - 1) % 6),
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        Self::LABELS[self.index()]
    }

    #[must_use]
    pub const fn as_toml_value(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Pinned(0) => "standing",
            Self::Pinned(1) => "glasses",
            Self::Pinned(2) => "side_look",
            Self::Pinned(3) => "reading_book",
            Self::Pinned(4) => "gloom",
            Self::Pinned(_) => "gloom_glasses",
        }
    }

    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "standing" | "0" => Some(Self::Pinned(0)),
            "glasses" | "1" => Some(Self::Pinned(1)),
            "side_look" | "2" => Some(Self::Pinned(2)),
            "reading_book" | "3" => Some(Self::Pinned(3)),
            "gloom" | "4" => Some(Self::Pinned(4)),
            "gloom_glasses" | "5" => Some(Self::Pinned(5)),
            _ => None,
        }
    }
}

pub const QUICK_MENU_POSE_LABELS: [&str; 3] = ["Profile Right", "3/4 Right", "Front View"];

/// WayWi rendering backend preference persisted in `[general] renderer` (applied on startup).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RendererMode {
    #[default]
    Software,
    Auto,
    Gpu,
}

impl RendererMode {
    pub const ALL: [Self; 3] = [Self::Software, Self::Auto, Self::Gpu];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Software => "software",
            Self::Auto => "auto",
            Self::Gpu => "gpu",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Software => "CPU (Software)",
            Self::Auto => "Auto (GPU)",
            Self::Gpu => "GPU",
        }
    }

    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "software" | "cpu" => Some(Self::Software),
            "auto" => Some(Self::Auto),
            "gpu" | "wgpu" | "vulkan" => Some(Self::Gpu),
            _ => None,
        }
    }

    #[must_use]
    pub const fn to_preference(self) -> RendererPreference {
        match self {
            Self::Software => RendererPreference::Software,
            Self::Auto => RendererPreference::Auto,
            Self::Gpu => RendererPreference::Gpu,
        }
    }

    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Software => 0,
            Self::Auto => 1,
            Self::Gpu => 2,
        }
    }

    #[must_use]
    pub const fn from_index(idx: usize) -> Self {
        match idx {
            1 => Self::Auto,
            2 => Self::Gpu,
            _ => Self::Software,
        }
    }
}

/// WayWi runtime memory/cache preset persisted in `[general] memory_profile` (applied on startup).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MemoryProfile {
    #[default]
    Balanced,
    LowMemory,
    HighThroughput,
}

impl MemoryProfile {
    pub const ALL: [Self; 3] = [Self::Balanced, Self::LowMemory, Self::HighThroughput];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Balanced => "balanced",
            Self::LowMemory => "low_memory",
            Self::HighThroughput => "high_throughput",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Balanced => "Balanced",
            Self::LowMemory => "Low Memory",
            Self::HighThroughput => "High Throughput",
        }
    }

    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "balanced" | "default" => Some(Self::Balanced),
            "low_memory" | "low-memory" | "low" => Some(Self::LowMemory),
            "high_throughput" | "high-throughput" | "high" => Some(Self::HighThroughput),
            _ => None,
        }
    }

    #[must_use]
    pub const fn to_memory_config(self) -> MemoryConfig {
        match self {
            Self::Balanced => MemoryConfig::balanced(),
            Self::LowMemory => MemoryConfig::low_memory(),
            Self::HighThroughput => MemoryConfig::high_throughput(),
        }
    }

    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Balanced => 0,
            Self::LowMemory => 1,
            Self::HighThroughput => 2,
        }
    }

    #[must_use]
    pub const fn from_index(idx: usize) -> Self {
        match idx {
            1 => Self::LowMemory,
            2 => Self::HighThroughput,
            _ => Self::Balanced,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneralConfig {
    pub clock_24h: bool,
    pub clock_alt_default: bool,
    pub foreground_booster: bool,
    pub power_countdown_secs: u8,
    pub renderer: RendererMode,
    pub memory_profile: MemoryProfile,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            clock_24h: true,
            clock_alt_default: false,
            foreground_booster: true,
            power_countdown_secs: crate::app::quick_menu::POWER_COUNTDOWN_SECS,
            renderer: RendererMode::Software,
            memory_profile: MemoryProfile::Balanced,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioConfig {
    pub xdg_sounds_enabled: bool,
    pub ui_feedback_sounds: bool,
    pub notification_sounds: bool,
    /// Master sound cue volume percentage (`10..=150`).
    pub sound_volume_pct: u16,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            xdg_sounds_enabled: true,
            ui_feedback_sounds: true,
            notification_sounds: true,
            sound_volume_pct: 100,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WallpaperConfig {
    pub enabled: bool,
    /// Empty string uses the embedded Gentoo dark wallpaper fallback; non-empty loads dynamically
    /// from disk with `AssetRetention::ReclaimAfterPresent` (0 B RAM when idle).
    pub path: String,
    pub fit: WallpaperFit,
}

impl Default for WallpaperConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            path: String::new(),
            fit: WallpaperFit::Cover,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpriteConfig {
    pub desktop_enabled: bool,
    pub rotate_on_workspace: bool,
    pub mouth_on_notification: bool,
    pub desktop_pose: DesktopPoseMode,
    /// Desktop Yuki scale multiplier percentage (`70..=115`).
    pub desktop_scale_pct: u16,
    /// Idle rotation interval in seconds (`30..=600`).
    pub rotate_interval_secs: u16,
    /// Quick Menu witch portrait pose (`0..=2`).
    pub quick_menu_pose: usize,
}

impl Default for SpriteConfig {
    fn default() -> Self {
        Self {
            desktop_enabled: true,
            rotate_on_workspace: true,
            mouth_on_notification: true,
            desktop_pose: DesktopPoseMode::Auto,
            desktop_scale_pct: 100,
            rotate_interval_secs: 180,
            quick_menu_pose: 0,
        }
    }
}

/// Unified right-side monitoring widgets (Volume/Audio through Bluetooth) that can be freely
/// reordered and assigned to the main Bar, the expandable Drawer, or Hidden.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RightMetricId {
    Audio,
    Cpu,
    Memory,
    Network,
    Gpu,
    Temp,
    Disk,
    Bluetooth,
}

impl RightMetricId {
    pub const ALL: [Self; 8] = [
        Self::Audio,
        Self::Cpu,
        Self::Memory,
        Self::Network,
        Self::Gpu,
        Self::Temp,
        Self::Disk,
        Self::Bluetooth,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Audio => "audio",
            Self::Cpu => "cpu",
            Self::Memory => "memory",
            Self::Network => "network",
            Self::Gpu => "gpu",
            Self::Temp => "temp",
            Self::Disk => "disk",
            Self::Bluetooth => "bluetooth",
        }
    }

    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "audio" | "volume" | "wireplumber" => Some(Self::Audio),
            "cpu" => Some(Self::Cpu),
            "memory" | "mem" => Some(Self::Memory),
            "network" | "net" => Some(Self::Network),
            "gpu" => Some(Self::Gpu),
            "temp" | "thermal" => Some(Self::Temp),
            "disk" | "storage" => Some(Self::Disk),
            "bluetooth" | "bt" => Some(Self::Bluetooth),
            _ => None,
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Audio => "Audio",
            Self::Cpu => "CPU",
            Self::Memory => "Memory",
            Self::Network => "Network",
            Self::Gpu => "GPU",
            Self::Temp => "Temp",
            Self::Disk => "Disk",
            Self::Bluetooth => "Bluetooth",
        }
    }

    #[must_use]
    pub const fn default_placement(self) -> MetricPlacement {
        match self {
            Self::Audio | Self::Cpu | Self::Memory | Self::Network => MetricPlacement::Bar,
            Self::Gpu | Self::Temp | Self::Disk | Self::Bluetooth => MetricPlacement::Drawer,
        }
    }

    /// Fixed logical width used when this metric lives inside the animated expandable drawer.
    #[must_use]
    pub const fn drawer_width(self) -> f32 {
        match self {
            Self::Audio => 56.0,
            Self::Cpu => 56.0,
            Self::Memory => 58.0,
            Self::Network => 68.0,
            Self::Gpu => crate::theme::GPU_EXPANDED_WIDTH,
            Self::Temp => crate::theme::TEMP_EXPANDED_WIDTH,
            Self::Disk => crate::theme::DISK_EXPANDED_WIDTH,
            Self::Bluetooth => crate::theme::BT_EXPANDED_WIDTH,
        }
    }
}

/// Where a right-side monitoring metric appears on the bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetricPlacement {
    Bar,
    Drawer,
    Hidden,
}

impl MetricPlacement {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bar => "bar",
            Self::Drawer => "drawer",
            Self::Hidden => "off",
        }
    }

    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "bar" => Some(Self::Bar),
            "drawer" => Some(Self::Drawer),
            "off" | "hidden" | "none" => Some(Self::Hidden),
            _ => None,
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Bar => "Bar",
            Self::Drawer => "Drawer",
            Self::Hidden => "Off",
        }
    }

    #[must_use]
    pub const fn cycle_next(self) -> Self {
        match self {
            Self::Bar => Self::Drawer,
            Self::Drawer => Self::Hidden,
            Self::Hidden => Self::Bar,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RightMetricSlot {
    pub id: RightMetricId,
    pub placement: MetricPlacement,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WidgetsConfig {
    pub show_sysinfo: bool,
    pub show_workspaces: bool,
    pub show_window_info: bool,
    pub show_notifications: bool,
    pub show_mpris: bool,
    pub show_tray: bool,
    pub right_metrics: Vec<RightMetricSlot>,
}

impl Default for WidgetsConfig {
    fn default() -> Self {
        Self {
            show_sysinfo: true,
            show_workspaces: true,
            show_window_info: true,
            show_notifications: true,
            show_mpris: true,
            show_tray: true,
            right_metrics: RightMetricId::ALL
                .into_iter()
                .map(|id| RightMetricSlot {
                    id,
                    placement: id.default_placement(),
                })
                .collect(),
        }
    }
}

impl WidgetsConfig {
    /// Ensures every [`RightMetricId`] appears exactly once in `right_metrics`.
    pub fn normalize_right_metrics(&mut self) {
        let mut seen = [false; RightMetricId::ALL.len()];
        let mut out = Vec::with_capacity(RightMetricId::ALL.len());
        for slot in self.right_metrics.drain(..) {
            let idx = slot.id as usize;
            if !seen[idx] {
                seen[idx] = true;
                out.push(slot);
            }
        }
        for id in RightMetricId::ALL {
            if !seen[id as usize] {
                out.push(RightMetricSlot {
                    id,
                    placement: id.default_placement(),
                });
            }
        }
        self.right_metrics = out;
    }

    #[must_use]
    pub fn bar_metrics(&self) -> Vec<RightMetricId> {
        self.right_metrics
            .iter()
            .filter(|s| s.placement == MetricPlacement::Bar)
            .map(|s| s.id)
            .collect()
    }

    #[must_use]
    pub fn drawer_metrics(&self) -> Vec<RightMetricId> {
        self.right_metrics
            .iter()
            .filter(|s| s.placement == MetricPlacement::Drawer)
            .map(|s| s.id)
            .collect()
    }

    #[must_use]
    pub fn has_any_right_mantle(&self) -> bool {
        self.show_tray
            || self
                .right_metrics
                .iter()
                .any(|s| s.placement != MetricPlacement::Hidden)
    }

    #[must_use]
    pub fn is_in_drawer(&self, id: RightMetricId) -> bool {
        self.placement_of(id) == MetricPlacement::Drawer
    }

    #[must_use]
    pub fn placement_of(&self, id: RightMetricId) -> MetricPlacement {
        self.right_metrics
            .iter()
            .find(|s| s.id == id)
            .map_or_else(|| id.default_placement(), |s| s.placement)
    }

    #[cfg(test)]
    pub fn set_metric_placement(&mut self, id: RightMetricId, placement: MetricPlacement) {
        if let Some(slot) = self.right_metrics.iter_mut().find(|s| s.id == id) {
            slot.placement = placement;
        }
    }

    pub fn cycle_metric_placement(&mut self, id: RightMetricId) -> MetricPlacement {
        self.normalize_right_metrics();
        if let Some(slot) = self.right_metrics.iter_mut().find(|s| s.id == id) {
            slot.placement = slot.placement.cycle_next();
            return slot.placement;
        }
        id.default_placement()
    }

    /// Moves `id` up (`delta < 0`) or down (`delta > 0`) in `right_metrics`; returns `true` if moved.
    pub fn move_metric(&mut self, id: RightMetricId, delta: i32) -> bool {
        self.normalize_right_metrics();
        let Some(pos) = self.right_metrics.iter().position(|s| s.id == id) else {
            return false;
        };
        if delta < 0 && pos > 0 {
            self.right_metrics.swap(pos, pos - 1);
            true
        } else if delta > 0 && pos + 1 < self.right_metrics.len() {
            self.right_metrics.swap(pos, pos + 1);
            true
        } else {
            false
        }
    }

    #[must_use]
    pub fn format_right_metrics_toml(&self) -> String {
        self.right_metrics
            .iter()
            .map(|s| format!("{}:{}", s.id.as_str(), s.placement.as_str()))
            .collect::<Vec<_>>()
            .join(",")
    }

    #[must_use]
    pub fn parse_right_metrics_toml(spec: &str) -> Vec<RightMetricSlot> {
        let mut out = Vec::new();
        for token in spec.split(',') {
            let trimmed = token.trim();
            if trimmed.is_empty() {
                continue;
            }
            let (raw_id, raw_place) = trimmed
                .split_once(':')
                .map_or((trimmed, None), |(a, b)| (a, Some(b)));
            let Some(id) = RightMetricId::parse(raw_id) else {
                continue;
            };
            let placement = raw_place
                .and_then(MetricPlacement::parse)
                .unwrap_or_else(|| id.default_placement());
            out.push(RightMetricSlot { id, placement });
        }
        out
    }
}

/// Complete user configuration persisted as pure TOML.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WidgetConfig {
    pub general: GeneralConfig,
    pub audio: AudioConfig,
    pub wallpaper: WallpaperConfig,
    pub sprite: SpriteConfig,
    pub widgets: WidgetsConfig,
}

impl WidgetConfig {
    #[must_use]
    pub fn default_path() -> PathBuf {
        let root = XdgBaseDirs::from_env()
            .config_home
            .unwrap_or_else(|| PathBuf::from("/tmp"));
        root.join("sina-desktop-widget").join(CONFIG_FILE_NAME)
    }

    #[must_use]
    pub fn load_from_path(path: &Path) -> Self {
        let Ok(contents) = fs::read_to_string(path) else {
            return Self::default();
        };
        Self::from_toml(&contents)
    }

    pub fn save_to_path(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("toml.tmp");
        fs::write(&tmp, self.to_toml())?;
        fs::rename(&tmp, path)?;
        Ok(())
    }

    #[must_use]
    pub fn to_toml(&self) -> String {
        let escaped_wp = escape_toml_string(&self.wallpaper.path);
        format!(
            "# sina-desktop-widget configuration\n\
             \n\
             [general]\n\
             clock_24h = {}\n\
             clock_alt_default = {}\n\
             foreground_booster = {}\n\
             power_countdown_secs = {}\n\
             renderer = \"{}\"\n\
             memory_profile = \"{}\"\n\
             \n\
             [audio]\n\
             xdg_sounds_enabled = {}\n\
             ui_feedback_sounds = {}\n\
             notification_sounds = {}\n\
             sound_volume_pct = {}\n\
             \n\
             [wallpaper]\n\
             enabled = {}\n\
             path = \"{escaped_wp}\"\n\
             fit = \"{}\"\n\
             \n\
             [sprite]\n\
             desktop_enabled = {}\n\
             rotate_on_workspace = {}\n\
             mouth_on_notification = {}\n\
             desktop_pose = \"{}\"\n\
             desktop_scale_pct = {}\n\
             rotate_interval_secs = {}\n\
             quick_menu_pose = {}\n\
             \n\
             [widgets]\n\
             show_sysinfo = {}\n\
             show_workspaces = {}\n\
             show_window_info = {}\n\
             show_notifications = {}\n\
             show_mpris = {}\n\
             show_tray = {}\n\
             right_metrics = \"{}\"\n",
            self.general.clock_24h,
            self.general.clock_alt_default,
            self.general.foreground_booster,
            self.general.power_countdown_secs.min(10),
            self.general.renderer.as_str(),
            self.general.memory_profile.as_str(),
            self.audio.xdg_sounds_enabled,
            self.audio.ui_feedback_sounds,
            self.audio.notification_sounds,
            self.audio.sound_volume_pct.clamp(10, 150),
            self.wallpaper.enabled,
            self.wallpaper.fit.as_str(),
            self.sprite.desktop_enabled,
            self.sprite.rotate_on_workspace,
            self.sprite.mouth_on_notification,
            self.sprite.desktop_pose.as_toml_value(),
            self.sprite.desktop_scale_pct.clamp(70, 115),
            self.sprite.rotate_interval_secs.clamp(30, 600),
            self.sprite.quick_menu_pose % 3,
            self.widgets.show_sysinfo,
            self.widgets.show_workspaces,
            self.widgets.show_window_info,
            self.widgets.show_notifications,
            self.widgets.show_mpris,
            self.widgets.show_tray,
            self.widgets.format_right_metrics_toml(),
        )
    }

    #[must_use]
    pub fn from_toml(input: &str) -> Self {
        let mut cfg = Self::default();
        let mut section = String::new();

        for raw_line in input.lines() {
            let line = strip_toml_comment(raw_line).trim();
            if line.is_empty() {
                continue;
            }
            if line.starts_with('[') && line.ends_with(']') {
                section = line[1..line.len() - 1].trim().to_ascii_lowercase();
                continue;
            }
            let Some((raw_k, raw_v)) = line.split_once('=') else {
                continue;
            };
            let key = raw_k.trim().to_ascii_lowercase();
            let val = raw_v.trim();

            match (section.as_str(), key.as_str()) {
                ("general", "clock_24h") => {
                    if let Some(b) = parse_toml_bool(val) {
                        cfg.general.clock_24h = b;
                    }
                }
                ("general", "clock_alt_default") => {
                    if let Some(b) = parse_toml_bool(val) {
                        cfg.general.clock_alt_default = b;
                    }
                }
                ("general", "foreground_booster") => {
                    if let Some(b) = parse_toml_bool(val) {
                        cfg.general.foreground_booster = b;
                    }
                }
                ("general", "power_countdown_secs") => {
                    if let Ok(n) = val.parse::<u8>() {
                        cfg.general.power_countdown_secs = n.min(10);
                    }
                }
                ("general", "renderer") => {
                    if let Some(s) = parse_toml_string(val)
                        && let Some(mode) = RendererMode::parse(&s)
                    {
                        cfg.general.renderer = mode;
                    }
                }
                ("general", "memory_profile") => {
                    if let Some(s) = parse_toml_string(val)
                        && let Some(profile) = MemoryProfile::parse(&s)
                    {
                        cfg.general.memory_profile = profile;
                    }
                }
                ("audio", "xdg_sounds_enabled") => {
                    if let Some(b) = parse_toml_bool(val) {
                        cfg.audio.xdg_sounds_enabled = b;
                    }
                }
                ("audio", "ui_feedback_sounds") => {
                    if let Some(b) = parse_toml_bool(val) {
                        cfg.audio.ui_feedback_sounds = b;
                    }
                }
                ("audio", "notification_sounds") => {
                    if let Some(b) = parse_toml_bool(val) {
                        cfg.audio.notification_sounds = b;
                    }
                }
                ("audio", "sound_volume_pct") => {
                    if let Ok(n) = val.parse::<u16>() {
                        cfg.audio.sound_volume_pct = n.clamp(10, 150);
                    }
                }
                ("wallpaper", "enabled") => {
                    if let Some(b) = parse_toml_bool(val) {
                        cfg.wallpaper.enabled = b;
                    }
                }
                ("wallpaper", "path") => {
                    if let Some(s) = parse_toml_string(val) {
                        cfg.wallpaper.path = s;
                    }
                }
                ("wallpaper", "fit") => {
                    if let Some(s) = parse_toml_string(val)
                        && let Some(fit) = WallpaperFit::parse(&s)
                    {
                        cfg.wallpaper.fit = fit;
                    }
                }
                ("sprite", "desktop_enabled") => {
                    if let Some(b) = parse_toml_bool(val) {
                        cfg.sprite.desktop_enabled = b;
                    }
                }
                ("sprite", "rotate_on_workspace") => {
                    if let Some(b) = parse_toml_bool(val) {
                        cfg.sprite.rotate_on_workspace = b;
                    }
                }
                ("sprite", "mouth_on_notification") => {
                    if let Some(b) = parse_toml_bool(val) {
                        cfg.sprite.mouth_on_notification = b;
                    }
                }
                ("sprite", "desktop_pose") => {
                    if let Some(s) = parse_toml_string(val)
                        && let Some(mode) = DesktopPoseMode::parse(&s)
                    {
                        cfg.sprite.desktop_pose = mode;
                    }
                }
                ("sprite", "desktop_scale_pct") => {
                    if let Ok(n) = val.parse::<u16>() {
                        cfg.sprite.desktop_scale_pct = n.clamp(70, 115);
                    }
                }
                ("sprite", "rotate_interval_secs") => {
                    if let Ok(n) = val.parse::<u16>() {
                        cfg.sprite.rotate_interval_secs = n.clamp(30, 600);
                    }
                }
                ("sprite", "quick_menu_pose") => {
                    if let Ok(n) = val.parse::<usize>() {
                        cfg.sprite.quick_menu_pose = n % 3;
                    }
                }
                ("widgets", "show_sysinfo") => {
                    if let Some(b) = parse_toml_bool(val) {
                        cfg.widgets.show_sysinfo = b;
                    }
                }
                ("widgets", "show_workspaces") => {
                    if let Some(b) = parse_toml_bool(val) {
                        cfg.widgets.show_workspaces = b;
                    }
                }
                ("widgets", "show_window_info") => {
                    if let Some(b) = parse_toml_bool(val) {
                        cfg.widgets.show_window_info = b;
                    }
                }
                ("widgets", "show_notifications") => {
                    if let Some(b) = parse_toml_bool(val) {
                        cfg.widgets.show_notifications = b;
                    }
                }
                ("widgets", "show_mpris") => {
                    if let Some(b) = parse_toml_bool(val) {
                        cfg.widgets.show_mpris = b;
                    }
                }
                ("widgets", "show_tray") => {
                    if let Some(b) = parse_toml_bool(val) {
                        cfg.widgets.show_tray = b;
                    }
                }
                ("widgets", "right_metrics") => {
                    if let Some(s) = parse_toml_string(val) {
                        cfg.widgets.right_metrics = WidgetsConfig::parse_right_metrics_toml(&s);
                        cfg.widgets.normalize_right_metrics();
                    }
                }
                _ => {}
            }
        }

        cfg.widgets.normalize_right_metrics();
        cfg
    }
}

/// Expands a leading `~/` to `$HOME/` for user-supplied filesystem paths.
#[must_use]
pub fn expand_tilde_path(raw: &str) -> PathBuf {
    let trimmed = raw.trim();
    if let Some(rest) = trimmed.strip_prefix("~/")
        && let Some(home) = std::env::var_os("HOME")
    {
        return PathBuf::from(home).join(rest);
    }
    PathBuf::from(trimmed)
}

fn strip_toml_comment(line: &str) -> &str {
    let mut in_double = false;
    let mut in_single = false;
    let mut escaped = false;
    for (idx, ch) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' if in_double => escaped = true,
            '"' if !in_single => in_double = !in_double,
            '\'' if !in_double => in_single = !in_single,
            '#' if !in_double && !in_single => return &line[..idx],
            _ => {}
        }
    }
    line
}

fn parse_toml_bool(val: &str) -> Option<bool> {
    match val.trim() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn escape_toml_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

fn parse_toml_string(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.len() >= 2 && trimmed.starts_with('\'') && trimmed.ends_with('\'') {
        return Some(trimmed[1..trimmed.len() - 1].to_owned());
    }
    if trimmed.len() < 2 || !trimmed.starts_with('"') || !trimmed.ends_with('"') {
        return None;
    }
    let inner = &trimmed[1..trimmed.len() - 1];
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next()? {
                '\\' => out.push('\\'),
                '"' => out.push('"'),
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                other => {
                    out.push('\\');
                    out.push(other);
                }
            }
        } else {
            out.push(ch);
        }
    }
    Some(out)
}

/// Universal vertical rail tabs inside the Quick Menu Settings drawer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SettingsSection {
    #[default]
    General,
    Audio,
    Desktop,
    Bar,
}

impl SettingsSection {
    pub const ALL: [Self; 4] = [Self::General, Self::Audio, Self::Desktop, Self::Bar];

    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::General => "Gen",
            Self::Audio => "Audio",
            Self::Desktop => "Desk",
            Self::Bar => "Bar",
        }
    }

    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Audio => "Audio & Sound",
            Self::Desktop => "Desktop & Appearance",
            Self::Bar => "Bar & Modules",
        }
    }

    #[must_use]
    pub const fn icon(self) -> &'static str {
        match self {
            Self::General => "\u{f013}",
            Self::Audio => "\u{f028}",
            Self::Desktop => "\u{f108}",
            Self::Bar => "\u{f2d0}",
        }
    }

    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::General => 0,
            Self::Audio => 1,
            Self::Desktop => 2,
            Self::Bar => 3,
        }
    }
}

/// All boolean toggle controls in the Settings panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SettingToggleId {
    Clock24h,
    ClockAltDefault,
    ForegroundBooster,
    XdgSoundsEnabled,
    UiFeedbackSounds,
    NotificationSounds,
    WallpaperEnabled,
    SpriteDesktopEnabled,
    SpriteRotateOnWorkspace,
    SpriteMouthOnNotification,
    WidgetSysInfo,
    WidgetWorkspaces,
    WidgetWindowInfo,
    WidgetNotifications,
    WidgetMpris,
    WidgetTray,
}

/// All numeric `- value +` stepper controls in the Settings panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SettingStepperId {
    PowerCountdownSecs,
    SoundVolumePct,
    DesktopScalePct,
    SpriteRotateIntervalSecs,
}

/// Inline expandable dropdown controls in the Settings panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SettingsDropdown {
    Renderer,
    MemoryProfile,
    WallpaperFit,
    DesktopPose,
    QuickMenuPose,
}

/// Interactive UI state for the Quick Menu Settings panel.
pub struct SettingsUiState {
    pub active_section: SettingsSection,
    pub rail_scroll_y: f32,
    pub list_scroll_y: f32,
    pub open_dropdown: Option<SettingsDropdown>,
    pub wallpaper_input: TextInputState,
    pub wallpaper_focused: bool,
    pub wallpaper_cursor_x: f32,
    pub wallpaper_error: Option<String>,
}

impl Default for SettingsUiState {
    fn default() -> Self {
        Self {
            active_section: SettingsSection::General,
            rail_scroll_y: 0.0,
            list_scroll_y: 0.0,
            open_dropdown: None,
            wallpaper_input: TextInputState::new(),
            wallpaper_focused: false,
            wallpaper_cursor_x: 0.0,
            wallpaper_error: None,
        }
    }
}

impl SettingsUiState {
    pub fn sync_from_config(&mut self, cfg: &WidgetConfig) {
        self.wallpaper_input = TextInputState::new();
        self.wallpaper_input.set_text(&cfg.wallpaper.path);
        self.wallpaper_focused = false;
        self.wallpaper_cursor_x = 0.0;
        self.wallpaper_error = None;
    }

    pub fn select_section(&mut self, section: SettingsSection) {
        if self.active_section != section {
            self.active_section = section;
            self.list_scroll_y = 0.0;
            self.open_dropdown = None;
            self.wallpaper_focused = false;
        }
    }

    pub fn unfocus_inputs(&mut self) {
        self.wallpaper_focused = false;
    }

    pub fn remeasure_wallpaper_cursor(&mut self, ctx: &mut EffectCtx<'_, crate::app::msg::Msg>) {
        let display = self.wallpaper_input.display_text();
        let cursor_byte = self.wallpaper_input.display_cursor_bytes();
        let metrics = ctx.measure_text_cursor(&display, &font_ui(), 9.5.logical_px(), cursor_byte);
        self.wallpaper_cursor_x = metrics.cursor_x.get();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toml_round_trip_preserves_all_settings_and_escaped_paths() {
        let mut cfg = WidgetConfig::default();
        cfg.general.clock_24h = false;
        cfg.general.clock_alt_default = true;
        cfg.general.foreground_booster = false;
        cfg.general.power_countdown_secs = 5;
        cfg.general.renderer = RendererMode::Gpu;
        cfg.general.memory_profile = MemoryProfile::LowMemory;
        cfg.audio.xdg_sounds_enabled = false;
        cfg.audio.ui_feedback_sounds = false;
        cfg.audio.notification_sounds = false;
        cfg.audio.sound_volume_pct = 130;
        cfg.wallpaper.enabled = false;
        cfg.wallpaper.path = String::from("/home/sina/Pictures/yuki \"night\" #1\\wall.png");
        cfg.wallpaper.fit = WallpaperFit::Contain;
        cfg.sprite.desktop_enabled = false;
        cfg.sprite.rotate_on_workspace = false;
        cfg.sprite.mouth_on_notification = false;
        cfg.sprite.desktop_pose = DesktopPoseMode::Pinned(3);
        cfg.sprite.desktop_scale_pct = 85;
        cfg.sprite.rotate_interval_secs = 300;
        cfg.sprite.quick_menu_pose = 2;
        cfg.widgets.show_sysinfo = false;
        cfg.widgets.show_mpris = false;
        cfg.widgets
            .set_metric_placement(RightMetricId::Bluetooth, MetricPlacement::Bar);
        cfg.widgets
            .set_metric_placement(RightMetricId::Cpu, MetricPlacement::Drawer);
        cfg.widgets.move_metric(RightMetricId::Bluetooth, -1);

        let serialized = cfg.to_toml();
        let parsed = WidgetConfig::from_toml(&serialized);
        assert_eq!(parsed, cfg);
    }

    #[test]
    fn right_metrics_normalizes_partial_and_duplicate_lists() {
        let partial = r#"
[widgets]
right_metrics = "bluetooth:bar,audio:drawer,bluetooth:off"
"#;
        let parsed_partial = WidgetConfig::from_toml(partial);
        assert_eq!(parsed_partial.widgets.right_metrics.len(), 8);
        assert_eq!(
            parsed_partial.widgets.right_metrics[0],
            RightMetricSlot {
                id: RightMetricId::Bluetooth,
                placement: MetricPlacement::Bar
            }
        );
        assert_eq!(
            parsed_partial.widgets.right_metrics[1],
            RightMetricSlot {
                id: RightMetricId::Audio,
                placement: MetricPlacement::Drawer
            }
        );
    }

    #[test]
    fn toml_parser_ignores_comments_unknown_keys_and_clamps_out_of_range_values() {
        let sample = r#"
# Top comment
[general]
clock_24h = false # inline comment
power_countdown_secs = 99
renderer = "auto"
memory_profile = "high_throughput"
unknown_future_flag = true

[audio]
sound_volume_pct = 500

[wallpaper]
path = '/tmp/my # literal path.png'
fit = "fill"

[sprite]
desktop_pose = "glasses"
desktop_scale_pct = 10
rotate_interval_secs = 9999
quick_menu_pose = 5

[unknown_section]
foo = "bar"
"#;
        let parsed = WidgetConfig::from_toml(sample);
        assert!(!parsed.general.clock_24h);
        assert_eq!(parsed.general.power_countdown_secs, 10);
        assert_eq!(parsed.general.renderer, RendererMode::Auto);
        assert_eq!(parsed.general.memory_profile, MemoryProfile::HighThroughput);
        assert_eq!(RendererMode::parse("cpu"), Some(RendererMode::Software));
        assert_eq!(RendererMode::parse("vulkan"), Some(RendererMode::Gpu));
        assert_eq!(RendererMode::parse("invalid"), None);
        assert_eq!(MemoryProfile::parse("low"), Some(MemoryProfile::LowMemory));
        assert_eq!(MemoryProfile::parse("default"), Some(MemoryProfile::Balanced));
        assert_eq!(MemoryProfile::parse("invalid"), None);
        assert_eq!(parsed.audio.sound_volume_pct, 150);
        assert_eq!(parsed.wallpaper.path, "/tmp/my # literal path.png");
        assert_eq!(parsed.wallpaper.fit, WallpaperFit::Fill);
        assert_eq!(parsed.sprite.desktop_pose, DesktopPoseMode::Pinned(1));
        assert_eq!(parsed.sprite.desktop_scale_pct, 70);
        assert_eq!(parsed.sprite.rotate_interval_secs, 600);
        assert_eq!(parsed.sprite.quick_menu_pose, 2);
    }

    #[test]
    fn default_config_path_ends_with_config_toml() {
        let path = WidgetConfig::default_path();
        assert_eq!(
            path.file_name().and_then(|s| s.to_str()),
            Some(CONFIG_FILE_NAME)
        );
    }
}
