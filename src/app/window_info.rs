//! Active Hyprland window category and rotating metadata state.

use waywi::compositor::hyprland::ClientInfo;
use waywi::prelude::DesktopEntry;
use waywi::system::os::{ProcessCgroupInfo, read_process_cgroup};

use super::reel::ReelState;

const WINDOW_INFO_HOLD_SECS: u16 = 3;

#[derive(Debug, Clone)]
pub struct WindowInfoState {
    pub client: Option<ClientInfo>,
    pub cgroup: Option<ProcessCgroupInfo>,
    pub category: String,
    pub reel: ReelState,
}

impl Default for WindowInfoState {
    fn default() -> Self {
        Self {
            client: None,
            cgroup: None,
            category: String::from("Desktop"),
            reel: ReelState::new(WINDOW_INFO_HOLD_SECS),
        }
    }
}

impl WindowInfoState {
    pub fn set_client(&mut self, next: Option<ClientInfo>, desktop_entries: &[DesktopEntry]) {
        let prev_addr = self.client.as_ref().map(|c| c.address.as_str());
        let next_addr = next.as_ref().map(|c| c.address.as_str());
        let switched_window = prev_addr != next_addr;

        let prev_pid = self.client.as_ref().and_then(|c| c.pid);
        let next_pid = next.as_ref().and_then(|c| c.pid);
        if prev_pid != next_pid {
            self.cgroup = next_pid.and_then(|pid| read_process_cgroup(pid).ok());
        }

        self.category = match next.as_ref() {
            Some(c) => resolve_window_category(c, desktop_entries),
            None => String::from("Desktop"),
        };
        self.client = next;

        if switched_window {
            self.reel.reset();
        }
    }

    pub fn refresh_category(&mut self, desktop_entries: &[DesktopEntry]) {
        if let Some(c) = &self.client {
            self.category = resolve_window_category(c, desktop_entries);
        }
    }

    pub fn update_floating(&mut self, address: &str, floating: bool) {
        if let Some(c) = &mut self.client
            && address_matches(&c.address, address)
        {
            c.floating = floating;
        }
    }

    pub fn update_pinned(&mut self, address: &str, pinned: bool) {
        if let Some(c) = &mut self.client
            && address_matches(&c.address, address)
        {
            c.pinned = pinned;
        }
    }

    pub fn update_title(&mut self, address: &str, title: &str) {
        if let Some(c) = &mut self.client
            && address_matches(&c.address, address)
        {
            c.title = title.to_owned();
        }
    }

    #[must_use]
    pub fn info_items(&self, boost_badge: Option<&'static str>) -> Vec<String> {
        let Some(c) = &self.client else {
            return vec![String::from("Idle")];
        };

        let mut items = Vec::with_capacity(7);
        items.push(if c.xwayland {
            String::from("XWayland")
        } else {
            String::from("Wayland")
        });

        let mode = if c.fullscreen_mode == 2 || c.fullscreen_mode == 3 {
            "Fullscreen"
        } else if c.fullscreen_mode == 1 {
            "Maximized"
        } else if c.floating {
            "Floating"
        } else {
            "Tiled"
        };
        items.push(mode.to_owned());

        if let Some(badge) = boost_badge {
            items.push(badge.to_owned());
        }

        if c.pinned {
            items.push(String::from("Pinned"));
        }
        if c.inhibiting_idle {
            items.push(String::from("No Idle"));
        }
        if !c.grouped.is_empty() {
            items.push(format!("Grp {}", c.grouped.len()));
        }
        if let Some(tag) = c.tags.first().map(|t| t.trim()).filter(|t| !t.is_empty()) {
            items.push(format!("#{tag}"));
        }
        if c.size.0 > 0 && c.size.1 > 0 {
            items.push(format!("{}x{}", c.size.0, c.size.1));
        }
        if let Some(pid) = c.pid {
            items.push(format!("PID {pid}"));
        }
        if let Some(mon) = c.monitor {
            items.push(format!("Mon {mon}"));
        }

        items
    }

    pub fn tick_second(&mut self, boost_badge: Option<&'static str>) {
        let count = self.info_items(boost_badge).len();
        self.reel.tick_second(count);
    }

    pub fn advance_animation(&mut self, now: std::time::Instant, boost_badge: Option<&'static str>) {
        let count = self.info_items(boost_badge).len();
        self.reel.advance(now, count);
    }

    #[must_use]
    pub const fn is_animating(&self) -> bool {
        self.reel.is_animating()
    }

    #[must_use]
    pub fn tooltip(&self, boost_line: Option<String>) -> String {
        let Some(c) = &self.client else {
            return String::from("No active window");
        };

        let mut lines = Vec::new();
        if !c.class.is_empty() {
            lines.push(format!("Class: {}", c.class));
        }
        lines.push(format!("Category: {}", self.category));
        lines.push(format!(
            "Protocol: {}",
            if c.xwayland { "XWayland" } else { "Wayland" }
        ));
        let mode = if c.fullscreen_mode == 2 || c.fullscreen_mode == 3 {
            "Fullscreen"
        } else if c.fullscreen_mode == 1 {
            "Maximized"
        } else if c.floating {
            "Floating"
        } else {
            "Tiled"
        };
        lines.push(format!("Mode: {mode}"));
        if c.pinned {
            lines.push(String::from("Pinned: yes"));
        }
        if c.inhibiting_idle {
            lines.push(String::from("Idle Inhibit: yes"));
        }
        if c.size.0 > 0 && c.size.1 > 0 {
            lines.push(format!("Size: {}x{}", c.size.0, c.size.1));
        }
        if let Some(mon) = c.monitor {
            lines.push(format!("Monitor: {mon}"));
        }
        if let Some(pid) = c.pid {
            lines.push(format!("PID: {pid}"));
        }
        if !c.tags.is_empty() {
            lines.push(format!("Tags: {}", c.tags.join(", ")));
        }
        if let Some(cg) = &self.cgroup
            && let Some(unit) = &cg.systemd_unit
        {
            lines.push(format!("Unit: {unit}"));
        }
        if let Some(boost) = boost_line {
            lines.push(boost);
        }
        lines.join("\n")
    }
}

fn address_matches(full: &str, candidate: &str) -> bool {
    let a = full.trim().trim_start_matches("0x");
    let b = candidate.trim().trim_start_matches("0x");
    !a.is_empty() && a.eq_ignore_ascii_case(b)
}

#[must_use]
pub fn resolve_window_category(client: &ClientInfo, desktop_entries: &[DesktopEntry]) -> String {
    if let Some(ct) = client
        .content_type
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return match ct.to_ascii_lowercase().as_str() {
            "game" => String::from("Game"),
            "video" => String::from("Video"),
            "photo" => String::from("Photo"),
            other => capitalize_first(other),
        };
    }

    for key in [&client.class, &client.initial_class] {
        let clean = key.trim();
        if clean.is_empty() {
            continue;
        }
        if let Some(entry) = find_matching_desktop_entry(desktop_entries, clean)
            && let Some(cat) = map_desktop_categories(&entry.categories)
        {
            return cat.to_owned();
        }
    }

    String::from("Window")
}

fn find_matching_desktop_entry<'a>(
    entries: &'a [DesktopEntry],
    class: &str,
) -> Option<&'a DesktopEntry> {
    let target = class.strip_suffix(".desktop").unwrap_or(class);
    entries.iter().find(|entry| {
        let id_base = entry.id.strip_suffix(".desktop").unwrap_or(&entry.id);
        id_base.eq_ignore_ascii_case(target)
            || entry
                .startup_wm_class
                .as_deref()
                .is_some_and(|wm| wm.trim().eq_ignore_ascii_case(target))
            || entry.name.trim().eq_ignore_ascii_case(target)
    })
}

fn map_desktop_categories(categories: &[String]) -> Option<&'static str> {
    const RULES: &[(&[&str], &str)] = &[
        (&["Game"], "Game"),
        (&["WebBrowser"], "Browser"),
        (&["TerminalEmulator"], "Terminal"),
        (&["Video", "TV", "AudioVideo", "Player"], "Media"),
        (&["Audio", "Music"], "Audio"),
        (&["Development", "IDE"], "Dev"),
        (
            &["Graphics", "Photography", "2DGraphics", "3DGraphics"],
            "Graphics",
        ),
        (&["FileManager"], "Files"),
        (&["InstantMessaging", "Chat", "Email", "Network"], "Network"),
        (&["Office", "TextEditor"], "Office"),
        (&["System", "Settings", "Monitor"], "System"),
        (&["Utility"], "Utility"),
    ];

    for (needles, label) in RULES {
        if categories
            .iter()
            .any(|c| needles.iter().any(|n| c.eq_ignore_ascii_case(n)))
        {
            return Some(*label);
        }
    }
    None
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use waywi::compositor::hyprland::ClientInfo;
    use waywi::prelude::DesktopEntry;

    use super::{WindowInfoState, resolve_window_category};

    fn sample_desktop(id: &str, wm_class: &str, categories: &[&str]) -> DesktopEntry {
        DesktopEntry {
            id: id.into(),
            path: PathBuf::from(format!("/usr/share/applications/{id}")),
            name: id.into(),
            generic_name: None,
            comment: None,
            exec: None,
            try_exec: None,
            working_dir: None,
            icon: None,
            categories: categories.iter().map(|s| (*s).to_owned()).collect(),
            keywords: Vec::new(),
            terminal: false,
            no_display: false,
            hidden: false,
            only_show_in: Vec::new(),
            not_show_in: Vec::new(),
            startup_wm_class: Some(wm_class.into()),
            dbus_activatable: false,
            actions: Vec::new(),
        }
    }

    #[test]
    fn prefers_wayland_content_type_then_desktop_categories() {
        let entries = vec![
            sample_desktop("firefox.desktop", "firefox", &["Network", "WebBrowser"]),
            sample_desktop("foot.desktop", "foot", &["System", "TerminalEmulator"]),
        ];

        let mut game_client = ClientInfo {
            address: "0x10".into(),
            class: "steam_app_1091500".into(),
            content_type: Some("game".into()),
            xwayland: true,
            fullscreen: true,
            fullscreen_mode: 2,
            size: (1920, 1080),
            ..ClientInfo::default()
        };
        assert_eq!(resolve_window_category(&game_client, &entries), "Game");

        game_client.content_type = None;
        game_client.class = "firefox".into();
        assert_eq!(resolve_window_category(&game_client, &entries), "Browser");

        let mut st = WindowInfoState::default();
        st.set_client(Some(game_client), &entries);
        assert_eq!(st.category, "Browser");
        let items = st.info_items(Some("VRAM+CPU"));
        assert!(items.contains(&String::from("XWayland")));
        assert!(items.contains(&String::from("Fullscreen")));
        assert!(items.contains(&String::from("VRAM+CPU")));
        assert!(items.contains(&String::from("1920x1080")));
    }
}
