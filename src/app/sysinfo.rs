//! Static OS metadata (Text A) and runtime system telemetry (Text B) for the left bar pill.

use std::time::Duration;

use waywi::compositor::hyprland::HyprlandVersion;
use waywi::system::{CpuSnapshot, MemorySnapshot, OsInfo, UptimeSnapshot};

use super::reel::ReelState;

/// Text A rotates less frequently (static OS/kernel/host metadata).
const TEXT_A_HOLD_SECS: u16 = 8;
/// Text B rotates more frequently (live uptime/load/memory runtime telemetry).
const TEXT_B_HOLD_SECS: u16 = 3;

#[derive(Debug, Clone)]
pub struct SysInfoState {
    pub os_info: Option<OsInfo>,
    pub hostname: Option<String>,
    pub hypr_version: Option<HyprlandVersion>,
    pub uptime: Option<UptimeSnapshot>,
    pub reel_a: ReelState,
    pub reel_b: ReelState,
}

impl Default for SysInfoState {
    fn default() -> Self {
        Self {
            os_info: None,
            hostname: None,
            hypr_version: None,
            uptime: None,
            reel_a: ReelState::new(TEXT_A_HOLD_SECS),
            reel_b: ReelState::new(TEXT_B_HOLD_SECS),
        }
    }
}

impl SysInfoState {
    #[must_use]
    pub fn from_waywi(
        os_info: Option<OsInfo>,
        hostname: Option<String>,
        hypr_version: Option<HyprlandVersion>,
        uptime: Option<UptimeSnapshot>,
    ) -> Self {
        Self {
            os_info,
            hostname,
            hypr_version,
            uptime,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn text_a_items(&self) -> Vec<String> {
        let mut items = Vec::with_capacity(4);
        if let Some(os) = &self.os_info {
            if let Some(distro) = short_distro_name(os) {
                items.push(distro);
            }
            if let Some(kernel) = os
                .kernel_release
                .as_deref()
                .map(compact_kernel_release)
                .filter(|s| !s.is_empty())
            {
                items.push(kernel);
            }
        }
        if let Some(host) = self
            .hostname
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            items.push(host.to_owned());
        }
        if let Some(hv) = &self.hypr_version {
            let ver = if !hv.tag.trim().is_empty() {
                hv.tag.trim()
            } else {
                hv.version.trim()
            };
            if !ver.is_empty() {
                items.push(format!("Hypr {ver}"));
            }
        } else if let Some(os) = &self.os_info {
            items.push(os.arch.to_owned());
        }
        if items.is_empty() {
            items.push(String::from("Linux"));
        }
        items
    }

    #[must_use]
    pub fn text_b_items(
        &self,
        cpu: Option<&CpuSnapshot>,
        memory: Option<&MemorySnapshot>,
    ) -> Vec<String> {
        let mut items = Vec::with_capacity(3);
        if let Some(up) = &self.uptime {
            items.push(format!("up {}", format_uptime_compact(up.uptime)));
        }
        let load = self
            .uptime
            .as_ref()
            .and_then(|u| u.load)
            .or_else(|| cpu.map(|c| c.load_avg));
        if let Some(l) = load {
            items.push(format!("ld {:.2} {:.2}", l.one, l.five));
            if l.total_tasks > 0 {
                items.push(format!("tsk {}/{}", l.running_tasks, l.total_tasks));
            }
        }
        if let Some(mem) = memory {
            items.push(format!(
                "mem {:.1}/{:.0}G",
                mem.used.as_gib(),
                mem.total.as_gib()
            ));
        }
        if items.is_empty() {
            items.push(String::from("up 0m"));
        }
        items
    }

    pub fn tick_second(&mut self, cpu: Option<&CpuSnapshot>, memory: Option<&MemorySnapshot>) {
        let count_a = self.text_a_items().len();
        let count_b = self.text_b_items(cpu, memory).len();
        self.reel_a.tick_second(count_a);
        self.reel_b.tick_second(count_b);
    }

    pub fn advance_animation(
        &mut self,
        now: std::time::Instant,
        cpu: Option<&CpuSnapshot>,
        memory: Option<&MemorySnapshot>,
    ) {
        let count_a = self.text_a_items().len();
        let count_b = self.text_b_items(cpu, memory).len();
        self.reel_a.advance(now, count_a);
        self.reel_b.advance(now, count_b);
    }

    #[must_use]
    pub const fn is_animating(&self) -> bool {
        self.reel_a.is_animating() || self.reel_b.is_animating()
    }

    #[must_use]
    pub fn tooltip(&self, cpu: Option<&CpuSnapshot>, memory: Option<&MemorySnapshot>) -> String {
        let mut lines = Vec::new();
        if let Some(os) = &self.os_info {
            let distro = os
                .pretty_name
                .as_deref()
                .or(os.name.as_deref())
                .unwrap_or("Linux");
            lines.push(format!("OS: {distro} ({})", os.arch));
            if let Some(k) = &os.kernel_release {
                lines.push(format!("Kernel: {k}"));
            }
        }
        if let Some(host) = &self.hostname
            && !host.trim().is_empty()
        {
            lines.push(format!("Host: {}", host.trim()));
        }
        if let Some(hv) = &self.hypr_version {
            let ver = if !hv.tag.is_empty() {
                hv.tag.as_str()
            } else {
                hv.version.as_str()
            };
            if !ver.is_empty() {
                lines.push(format!("Hyprland: {ver}"));
            }
        }
        if let Some(up) = &self.uptime {
            lines.push(format!("Uptime: {}", format_uptime_compact(up.uptime)));
        }
        let load = self
            .uptime
            .as_ref()
            .and_then(|u| u.load)
            .or_else(|| cpu.map(|c| c.load_avg));
        if let Some(l) = load {
            lines.push(format!("Load: {:.2} {:.2} {:.2}", l.one, l.five, l.fifteen));
            if l.total_tasks > 0 {
                lines.push(format!("Tasks: {} / {}", l.running_tasks, l.total_tasks));
            }
        }
        if let Some(mem) = memory {
            lines.push(format!(
                "Memory: {:.1} / {:.1} GiB",
                mem.used.as_gib(),
                mem.total.as_gib()
            ));
        }
        if lines.is_empty() {
            String::from("System Info")
        } else {
            lines.join("\n")
        }
    }
}

fn short_distro_name(os: &OsInfo) -> Option<String> {
    let raw = os
        .name
        .as_deref()
        .or(os.pretty_name.as_deref())
        .or(os.id.as_deref())?
        .trim()
        .trim_matches(|c| c == '"' || c == '\'');
    if raw.is_empty() {
        return None;
    }
    let trimmed = raw
        .strip_suffix(" Linux")
        .or_else(|| raw.strip_suffix(" GNU/Linux"))
        .unwrap_or(raw);
    Some(trimmed.to_owned())
}

fn compact_kernel_release(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.len() <= 13 {
        return trimmed.to_owned();
    }
    trimmed
        .split_once('-')
        .map_or_else(|| trimmed.to_owned(), |(base, _)| format!("Linux {base}"))
}

#[must_use]
pub fn format_uptime_compact(dur: Duration) -> String {
    let secs = dur.as_secs();
    let days = secs / 86_400;
    let hours = (secs % 86_400) / 3_600;
    let mins = (secs % 3_600) / 60;
    if days > 0 {
        format!("{days}d {hours}h")
    } else if hours > 0 {
        format!("{hours}h {mins}m")
    } else {
        format!("{mins}m")
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use waywi::system::{LoadAverage, OsInfo, UptimeSnapshot};

    use super::{SysInfoState, format_uptime_compact};

    #[test]
    fn builds_distinct_text_a_and_text_b_items_with_different_timings() {
        let info = SysInfoState::from_waywi(
            Some(OsInfo {
                name: Some("Gentoo Linux".into()),
                pretty_name: Some("Gentoo Linux".into()),
                id: Some("gentoo".into()),
                version_id: None,
                kernel_release: Some("6.19.8-gentoo-dist".into()),
                arch: "x86_64",
            }),
            Some("varasina".into()),
            None,
            Some(UptimeSnapshot {
                uptime: Duration::from_secs(7_500),
                idle_total: Duration::from_secs(0),
                load: Some(LoadAverage {
                    one: 0.42,
                    five: 0.35,
                    fifteen: 0.21,
                    running_tasks: 2,
                    total_tasks: 512,
                }),
            }),
        );

        let a = info.text_a_items();
        let b = info.text_b_items(None, None);
        assert!(a.iter().any(|s| s == "Gentoo"));
        assert!(a.iter().any(|s| s == "Linux 6.19.8"));
        assert!(a.iter().any(|s| s == "varasina"));
        assert!(b.iter().any(|s| s == "up 2h 5m"));
        assert!(b.iter().any(|s| s == "ld 0.42 0.35"));
        assert!(info.reel_a.hold_secs > info.reel_b.hold_secs);
        assert_eq!(format_uptime_compact(Duration::from_secs(90_000)), "1d 1h");
    }
}
