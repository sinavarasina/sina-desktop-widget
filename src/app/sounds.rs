//! Centralized, O(1)-cached XDG Sound Theme engine with per-cue cooldowns, GTK event-sound
//! preference detection, DND silence, and startup-baseline hardware/network transition tracking.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use waywi::prelude::*;

use super::config::AudioConfig;

#[cfg(not(test))]
const MAX_SOUND_CACHE: usize = 64;

/// Standard Freedesktop XDG Sound Naming Spec event cues used across the widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SoundCue {
    SystemReady,
    SystemShutdown,
    DesktopLogout,
    SuspendStart,
    DeviceAdded,
    DeviceRemoved,
    NetworkConnected,
    NetworkDisconnected,
    Bell,
    WindowUrgent,
    VolumeChange,
    MenuPopup,
    MenuPopdown,
    ToggleOn,
    ToggleOff,
    TabChanged,
    DialogCancel,
    DialogError,
    TrashEmpty,
    SearchEmpty,
}

impl SoundCue {
    /// Ordered XDG Sound Theme event names (`primary` then optional `fallback`).
    #[must_use]
    pub const fn xdg_names(self) -> &'static [&'static str] {
        match self {
            Self::SystemReady => &["system-ready"],
            Self::SystemShutdown => &["system-shutdown"],
            Self::DesktopLogout => &["desktop-logout"],
            Self::SuspendStart => &["suspend-start"],
            Self::DeviceAdded => &["device-added"],
            Self::DeviceRemoved => &["device-removed"],
            Self::NetworkConnected => &["network-connectivity-established"],
            Self::NetworkDisconnected => &["network-connectivity-lost"],
            Self::Bell => &["bell-window-system", "bell"],
            Self::WindowUrgent => &["window-attention-active", "bell"],
            Self::VolumeChange => &["audio-volume-change"],
            Self::MenuPopup => &["menu-popup"],
            Self::MenuPopdown => &["menu-popdown"],
            Self::ToggleOn => &["button-toggle-on", "button-pressed"],
            Self::ToggleOff => &["button-toggle-off", "button-released"],
            Self::TabChanged => &["notebook-tab-changed"],
            Self::DialogCancel => &["dialog-cancel"],
            Self::DialogError => &["dialog-error", "error"],
            Self::TrashEmpty => &["trash-empty", "item-deleted"],
            Self::SearchEmpty => &["search-results-empty"],
        }
    }

    /// Balanced playback volume (`0.0..=1.0`): subtle for UI feedback, clearer for system alerts.
    #[must_use]
    pub const fn volume(self) -> f32 {
        match self {
            Self::SystemShutdown | Self::DesktopLogout | Self::SuspendStart => 0.75,
            Self::DeviceAdded | Self::DeviceRemoved | Self::DialogError => 0.65,
            Self::SystemReady
            | Self::NetworkConnected
            | Self::NetworkDisconnected
            | Self::WindowUrgent => 0.60,
            Self::Bell | Self::TrashEmpty => 0.55,
            Self::VolumeChange | Self::DialogCancel => 0.50,
            Self::ToggleOn => 0.45,
            Self::MenuPopup | Self::ToggleOff => 0.42,
            Self::SearchEmpty => 0.40,
            Self::MenuPopdown => 0.38,
            Self::TabChanged => 0.36,
        }
    }

    /// Whether this cue is interactive UI feedback (can be muted separately from system/device cues).
    #[must_use]
    pub const fn is_ui_feedback(self) -> bool {
        matches!(
            self,
            Self::VolumeChange
                | Self::MenuPopup
                | Self::MenuPopdown
                | Self::ToggleOn
                | Self::ToggleOff
                | Self::TabChanged
                | Self::DialogCancel
                | Self::SearchEmpty
        )
    }

    /// Minimum interval between consecutive plays of the same cue to prevent audio spam.
    #[must_use]
    pub const fn cooldown(self) -> Duration {
        match self {
            Self::SystemReady => Duration::ZERO,
            Self::VolumeChange => Duration::from_millis(75),
            Self::MenuPopup | Self::MenuPopdown | Self::ToggleOn | Self::ToggleOff => {
                Duration::from_millis(80)
            }
            Self::TabChanged => Duration::from_millis(90),
            Self::DialogCancel => Duration::from_millis(120),
            Self::Bell => Duration::from_millis(150),
            Self::DeviceAdded | Self::DeviceRemoved | Self::DialogError | Self::TrashEmpty => {
                Duration::from_millis(250)
            }
            Self::WindowUrgent => Duration::from_millis(300),
            Self::SearchEmpty => Duration::from_millis(350),
            Self::SystemShutdown | Self::DesktopLogout | Self::SuspendStart => {
                Duration::from_millis(500)
            }
            Self::NetworkConnected | Self::NetworkDisconnected => Duration::from_millis(1000),
        }
    }
}

/// Stateful XDG sound manager owned by [`State`](super::state::State).
pub struct SoundEngine {
    pub event_sounds_enabled: bool,
    pub ui_feedback_enabled: bool,
    pub notification_sounds_enabled: bool,
    pub volume_scale: f32,
    pub detected_theme: String,
    #[cfg(not(test))]
    cache: HashMap<String, Option<SoundAsset>>,
    last_played: HashMap<SoundCue, Instant>,
    udisks_seeded: bool,
    known_udisks_drives: HashSet<String>,
    bt_seeded: bool,
    connected_bt_devices: HashSet<String>,
    nm_online: Option<bool>,
    #[cfg(test)]
    pub played_cues: Vec<SoundCue>,
}

impl Default for SoundEngine {
    fn default() -> Self {
        Self::new(read_gtk_event_sounds_enabled())
    }
}

impl SoundEngine {
    #[must_use]
    pub fn new(event_sounds_enabled: bool) -> Self {
        Self {
            event_sounds_enabled,
            ui_feedback_enabled: true,
            notification_sounds_enabled: true,
            volume_scale: 1.0,
            detected_theme: read_gtk_sound_theme_name(),
            #[cfg(not(test))]
            cache: HashMap::new(),
            last_played: HashMap::new(),
            udisks_seeded: false,
            known_udisks_drives: HashSet::new(),
            bt_seeded: false,
            connected_bt_devices: HashSet::new(),
            nm_online: None,
            #[cfg(test)]
            played_cues: Vec::new(),
        }
    }

    /// Applies user audio preferences from `config.toml`.
    pub fn apply_config(&mut self, cfg: &AudioConfig) {
        self.event_sounds_enabled = cfg.xdg_sounds_enabled;
        self.ui_feedback_enabled = cfg.ui_feedback_sounds;
        self.notification_sounds_enabled = cfg.notification_sounds;
        self.volume_scale = (f32::from(cfg.sound_volume_pct) / 100.0).clamp(0.1, 1.5);
    }

    /// Plays a standard XDG sound cue if event sounds are enabled, DND is off, and cooldown allows.
    pub fn play(&mut self, cue: SoundCue, dnd: bool, ctx: &EffectCtx<'_>) {
        if !self.event_sounds_enabled || dnd {
            return;
        }
        if cue.is_ui_feedback() && !self.ui_feedback_enabled {
            return;
        }
        let cooldown = cue.cooldown();
        let now = Instant::now();
        if !cooldown.is_zero()
            && self
                .last_played
                .get(&cue)
                .is_some_and(|prev| now.saturating_duration_since(*prev) < cooldown)
        {
            return;
        }
        self.last_played.insert(cue, now);

        #[cfg(test)]
        {
            let _ = ctx;
            self.played_cues.push(cue);
        }
        #[cfg(not(test))]
        {
            let vol = (cue.volume() * self.volume_scale).clamp(0.0, 1.0);
            for &name in cue.xdg_names() {
                if self.play_theme_name_cached(name, vol, ctx) {
                    return;
                }
            }
        }
    }

    /// Resolves `name` once via `ctx.load_system_sound` (caching hits and misses in O(1)) and
    /// plays the cached `SoundAsset` at `volume`. Returns `true` if a theme file was found.
    #[cfg(not(test))]
    pub fn play_theme_name_cached(
        &mut self,
        name: &str,
        volume: f32,
        ctx: &EffectCtx<'_>,
    ) -> bool {
        if !self.event_sounds_enabled {
            return false;
        }
        let asset = if let Some(&cached) = self.cache.get(name) {
            cached
        } else {
            let resolved = ctx.load_system_sound(name).ok();
            if self.cache.len() >= MAX_SOUND_CACHE {
                self.cache.clear();
            }
            self.cache.insert(name.to_owned(), resolved);
            resolved
        };
        if let Some(sound) = asset {
            ctx.play_sound_volume(sound, volume);
            true
        } else {
            false
        }
    }

    /// Tracks physical UDisks2 drives and plays `DeviceAdded` / `DeviceRemoved` on hotplug transitions
    /// (suppressing initial startup baseline).
    pub fn observe_udisks(&mut self, snap: &UDisksSnapshot, dnd: bool, ctx: &EffectCtx<'_>) {
        let current: HashSet<String> = snap
            .drives
            .iter()
            .map(|d| d.object_path.clone())
            .collect();

        if !self.udisks_seeded {
            self.known_udisks_drives = current;
            self.udisks_seeded = true;
            return;
        }

        let added = current.iter().any(|p| !self.known_udisks_drives.contains(p));
        let removed = self.known_udisks_drives.iter().any(|p| !current.contains(p));
        self.known_udisks_drives = current;

        if added {
            self.play(SoundCue::DeviceAdded, dnd, ctx);
        } else if removed {
            self.play(SoundCue::DeviceRemoved, dnd, ctx);
        }
    }

    /// Tracks connected BlueZ devices and plays `DeviceAdded` / `DeviceRemoved` on connection
    /// transitions (suppressing initial startup baseline).
    pub fn observe_bluetooth(&mut self, snap: &BluetoothSnapshot, dnd: bool, ctx: &EffectCtx<'_>) {
        let current: HashSet<String> = snap
            .devices
            .iter()
            .filter(|d| d.connected)
            .map(|d| d.object_path.clone())
            .collect();

        if !self.bt_seeded {
            self.connected_bt_devices = current;
            self.bt_seeded = true;
            return;
        }

        let added = current.iter().any(|p| !self.connected_bt_devices.contains(p));
        let removed = self.connected_bt_devices.iter().any(|p| !current.contains(p));
        self.connected_bt_devices = current;

        if added {
            self.play(SoundCue::DeviceAdded, dnd, ctx);
        } else if removed {
            self.play(SoundCue::DeviceRemoved, dnd, ctx);
        }
    }

    /// Tracks NetworkManager online/offline transitions and plays `NetworkConnected` /
    /// `NetworkDisconnected` (suppressing initial startup baseline).
    pub fn observe_network(
        &mut self,
        snap: &NetworkManagerSnapshot,
        dnd: bool,
        ctx: &EffectCtx<'_>,
    ) {
        let online = match snap.state {
            NmState::ConnectedLocal | NmState::ConnectedSite | NmState::ConnectedGlobal => {
                Some(true)
            }
            NmState::Disconnected | NmState::Asleep => Some(false),
            NmState::Unknown | NmState::Connecting | NmState::Disconnecting => None,
        };
        let Some(now_online) = online else {
            return;
        };
        let prev = self.nm_online.replace(now_online);
        if let Some(was_online) = prev
            && was_online != now_online
        {
            let cue = if now_online {
                SoundCue::NetworkConnected
            } else {
                SoundCue::NetworkDisconnected
            };
            self.play(cue, dnd, ctx);
        }
    }
}

/// Reads `gtk-sound-theme-name` from GTK4/GTK3 `settings.ini` (falls back to `"freedesktop"`).
#[must_use]
fn read_gtk_sound_theme_name() -> String {
    let config_root = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")));
    if let Some(root) = config_root {
        for rel in ["gtk-4.0/settings.ini", "gtk-3.0/settings.ini"] {
            let Ok(text) = fs::read_to_string(root.join(rel)) else {
                continue;
            };
            for raw_line in text.lines() {
                let line = raw_line.trim();
                if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
                    continue;
                }
                if let Some((k, v)) = line.split_once('=')
                    && k.trim().eq_ignore_ascii_case("gtk-sound-theme-name")
                {
                    let val = v.trim();
                    if !val.is_empty() {
                        return val.to_owned();
                    }
                }
            }
        }
    }
    String::from("freedesktop")
}

/// Reads `gtk-enable-event-sounds` once from GTK3/GTK4 `settings.ini` (defaults to `true`).
#[must_use]
fn read_gtk_event_sounds_enabled() -> bool {
    let config_root = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")));
    let Some(root) = config_root else {
        return true;
    };
    for rel in ["gtk-4.0/settings.ini", "gtk-3.0/settings.ini"] {
        let Ok(text) = fs::read_to_string(root.join(rel)) else {
            continue;
        };
        if let Some(val) = parse_gtk_bool_setting(&text, "gtk-enable-event-sounds") {
            return val;
        }
    }
    true
}

#[must_use]
fn parse_gtk_bool_setting(contents: &str, key: &str) -> Option<bool> {
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        if k.trim().eq_ignore_ascii_case(key) {
            let val = v.trim();
            if val == "0" || val.eq_ignore_ascii_case("false") {
                return Some(false);
            }
            if val == "1" || val.eq_ignore_ascii_case("true") {
                return Some(true);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_gtk_event_sounds_setting_cleanly() {
        assert_eq!(
            parse_gtk_bool_setting("[Settings]\ngtk-enable-event-sounds=0\n", "gtk-enable-event-sounds"),
            Some(false)
        );
        assert_eq!(
            parse_gtk_bool_setting("[Settings]\ngtk-enable-event-sounds = true\n", "gtk-enable-event-sounds"),
            Some(true)
        );
        assert_eq!(
            parse_gtk_bool_setting("[Settings]\ngtk-sound-theme-name=yuki-nagato\n", "gtk-enable-event-sounds"),
            None
        );
    }

    #[test]
    fn sound_cues_have_non_empty_xdg_names_and_sane_volumes() {
        let all = [
            SoundCue::SystemReady,
            SoundCue::SystemShutdown,
            SoundCue::DesktopLogout,
            SoundCue::SuspendStart,
            SoundCue::DeviceAdded,
            SoundCue::DeviceRemoved,
            SoundCue::NetworkConnected,
            SoundCue::NetworkDisconnected,
            SoundCue::Bell,
            SoundCue::WindowUrgent,
            SoundCue::VolumeChange,
            SoundCue::MenuPopup,
            SoundCue::MenuPopdown,
            SoundCue::ToggleOn,
            SoundCue::ToggleOff,
            SoundCue::TabChanged,
            SoundCue::DialogCancel,
            SoundCue::DialogError,
            SoundCue::TrashEmpty,
            SoundCue::SearchEmpty,
        ];
        for cue in all {
            assert!(!cue.xdg_names().is_empty());
            assert!(cue.volume() > 0.0 && cue.volume() <= 1.0);
        }
    }
}
