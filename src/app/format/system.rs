//! CPU, memory, and disk telemetry formatting.

use waywi::system::FreqUnit;

use crate::app::state::State;

use super::{MetricLine, ModuleMetric};

impl State {
    #[must_use]
    pub fn cpu_metric(&self) -> ModuleMetric {
        let usage = self.cpu.as_ref().map_or(0.0, |c| c.total_percent);
        let usage_i = usage.round() as u32;
        let avg_ghz = self
            .cpu
            .as_ref()
            .and_then(|c| c.avg_freq(FreqUnit::Ghz))
            .unwrap_or(0.0);
        let max_ghz = self
            .cpu
            .as_ref()
            .and_then(|c| c.max_freq(FreqUnit::Ghz))
            .unwrap_or(0.0);
        let min_ghz = self
            .cpu
            .as_ref()
            .and_then(|c| c.min_freq(FreqUnit::Ghz))
            .unwrap_or(0.0);
        let load_1m = self.cpu.as_ref().map_or(0.0, |c| c.load_avg.one);
        let mut tip_parts = vec![
            format!("Usage: {usage_i}%"),
            format!("Load: {load_1m:.2}"),
            format!("Avg Freq: {avg_ghz:.2}GHz"),
            format!("Max Freq: {max_ghz:.2}GHz"),
            format!("Min Freq: {min_ghz:.2}GHz"),
        ];
        if let Some(temp) = self.cpu_temp_celsius {
            tip_parts.push(format!("Temp: {temp:.0}°C"));
        }
        if let Some(watts) = self.cpu_power_watts {
            tip_parts.push(format!("Power: {watts:.1} W"));
        }
        ModuleMetric {
            lines: vec![
                MetricLine::new("\u{f2db}", format!("{usage_i}%")),
                MetricLine::new("\u{f04c5}", format!("{avg_ghz:.2}GHz")),
            ],
            tooltip: tip_parts.join("\n"),
            inactive: false,
        }
    }

    #[must_use]
    pub fn temp_metric(&self) -> ModuleMetric {
        let gpu_dev = self.gpu.as_ref().and_then(|g| g.gpus.first());
        let gpu_temp = gpu_dev.and_then(|g| g.temperature_celsius);
        let cpu_str = self
            .cpu_temp_celsius
            .map_or_else(|| String::from("--°C"), |t| format!("{t:.0}°C"));
        let gpu_str = gpu_temp.map_or_else(|| String::from("--°C"), |t| format!("{t:.0}°C"));

        let mut tip_parts = vec![
            format!("CPU Temp: {cpu_str}"),
            format!("GPU Temp: {gpu_str}"),
        ];
        if let Some(hotspot) = gpu_dev.and_then(|g| g.hotspot_celsius) {
            tip_parts.push(format!("GPU Hotspot: {hotspot:.0}°C"));
        }

        ModuleMetric {
            lines: vec![
                MetricLine::new("\u{f2db}", cpu_str),
                MetricLine::new("\u{f08ae}", gpu_str),
            ],
            tooltip: tip_parts.join("\n"),
            inactive: false,
        }
    }

    #[cfg(test)]
    #[must_use]
    pub fn cpu_display(&self) -> (String, String) {
        let m = self.cpu_metric();
        (m.joined_text(), m.tooltip)
    }

    #[must_use]
    pub fn memory_metric(&self) -> ModuleMetric {
        let Some(mem) = &self.memory else {
            return ModuleMetric {
                lines: vec![
                    MetricLine::new("\u{efc5}", "0%"),
                    MetricLine::new("\u{f0f86}", "0.0G"),
                ],
                tooltip: String::from("RAM: 0.0GB / 0.0GB (0%)\nSwap: 0.0GB / 0.0GB (0%)"),
                inactive: false,
            };
        };
        let total_b = mem.total.as_bytes() as f64;
        let used_b = mem.used.as_bytes() as f64;
        let pct = if total_b > 0.0 {
            ((used_b / total_b) * 100.0).round() as u32
        } else {
            0
        };
        let used_gib = mem.used.as_gib();
        let total_gib = mem.total.as_gib();

        let swap_total_b = mem.swap_total.as_bytes() as f64;
        let swap_used_b = mem.swap_used.as_bytes() as f64;
        let swap_pct = if swap_total_b > 0.0 {
            ((swap_used_b / swap_total_b) * 100.0).round() as u32
        } else {
            0
        };
        ModuleMetric {
            lines: vec![
                MetricLine::new("\u{efc5}", format!("{pct}%")),
                MetricLine::new("\u{f0f86}", format!("{used_gib:.1}G")),
            ],
            tooltip: format!(
                "RAM: {used_gib:.1}GB / {total_gib:.1}GB ({pct}%)\nSwap: {:.1}GB / {:.1}GB ({swap_pct}%)",
                mem.swap_used.as_gib(),
                mem.swap_total.as_gib()
            ),
            inactive: false,
        }
    }

    #[cfg(test)]
    #[must_use]
    pub fn memory_display(&self) -> (String, String) {
        let m = self.memory_metric();
        (m.joined_text(), m.tooltip)
    }

    #[must_use]
    pub fn disk_metric(&self) -> ModuleMetric {
        let selected_mount = self
            .disk_mounts
            .get(self.disk_ui.selected_mount_idx)
            .or_else(|| {
                self.disk_mounts
                    .iter()
                    .find(|m| m.mount_point.as_os_str() == "/")
            })
            .or_else(|| self.disk_mounts.first());

        let selected_path = selected_mount
            .map(|m| m.mount_point.to_string_lossy().into_owned())
            .unwrap_or_else(|| String::from("/"));

        let line1 = if selected_path == "/" || self.disk_mounts.len() <= 1 {
            format!("{:>5.1}G Used", self.disk_used_gb)
        } else {
            let short = selected_mount
                .and_then(|m| m.mount_point.file_name())
                .map(|s| s.to_string_lossy())
                .unwrap_or(std::borrow::Cow::Borrowed(&selected_path));
            let short_trunc: String = short.chars().take(5).collect();
            format!("{:>5.1}G /{short_trunc}", self.disk_used_gb)
        };
        let line2 = format!("{:>5.1}G Free", self.disk_free_gb);

        let mut tip = if let Some(m) = selected_mount {
            format!(
                "Path: {} ({} · {})\nTotal: {:.1} GB\nUsed: {:.1} GB ({:.0}%)\nFree: {:.1} GB ({:.0}%)",
                selected_path,
                m.device,
                m.fs_type,
                self.disk_total_gb,
                self.disk_used_gb,
                self.disk_pct_used,
                self.disk_free_gb,
                self.disk_pct_free
            )
        } else {
            format!(
                "Path: /\nTotal: {:.1} GB\nUsed: {:.1} GB ({:.0}%)\nFree: {:.1} GB ({:.0}%)",
                self.disk_total_gb,
                self.disk_used_gb,
                self.disk_pct_used,
                self.disk_free_gb,
                self.disk_pct_free
            )
        };

        if self.disk_mounts.len() > 1 {
            tip.push_str("\n\nMounted Filesystems:");
            for (idx, m) in self.disk_mounts.iter().enumerate() {
                let marker = if idx == self.disk_ui.selected_mount_idx {
                    "\u{25b8}"
                } else {
                    " "
                };
                const GB: f64 = 1_000_000_000.0;
                let used = m.used.as_bytes() as f64 / GB;
                let total = m.total.as_bytes() as f64 / GB;
                let pct = m.usage_percent();
                tip.push_str(&format!(
                    "\n{marker} {} ({}) {:.1}/{:.1}GB ({pct:.0}%)",
                    m.mount_point.display(),
                    m.fs_type,
                    used,
                    total
                ));
            }
        }

        let removable = self.udisks.removable_volumes();
        if !removable.is_empty() {
            tip.push_str("\n\nRemovable / UDisks2:");
            for b in removable {
                let status = if let Some(op) = self.disk_ui.pending_ops.get(&b.object_path) {
                    format!("{op}{}", self.disk_ui.pending_dots())
                } else if b.is_mounted() {
                    format!("mounted @ {}", b.mount_points.join(", "))
                } else if b.is_encrypted && !b.is_unlocked() {
                    String::from("locked (LUKS)")
                } else {
                    String::from("unmounted")
                };
                let size_gb = b.size_bytes as f64 / 1_000_000_000.0;
                tip.push_str(&format!(
                    "\n• {} ({:.1}GB {}) — {status}",
                    b.display_label(),
                    size_gb,
                    if b.id_type.is_empty() {
                        b.device.as_str()
                    } else {
                        b.id_type.as_str()
                    }
                ));
            }
        }

        ModuleMetric {
            lines: vec![
                MetricLine::new("\u{f0a0}", line1),
                MetricLine::new("\u{f0a0}", line2),
            ],
            tooltip: tip,
            inactive: false,
        }
    }

    #[cfg(test)]
    #[must_use]
    pub fn disk_display(&self) -> (String, String) {
        let m = self.disk_metric();
        (m.joined_text(), m.tooltip)
    }
}
