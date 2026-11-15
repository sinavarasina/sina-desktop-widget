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
        ModuleMetric {
            lines: vec![
                MetricLine::new("\u{f2db}", format!("{usage_i}%")),
                MetricLine::new("\u{f04c5}", format!("{avg_ghz:.2}GHz")),
            ],
            tooltip: format!(
                "Usage: {usage_i}%\nLoad: {load_1m:.2}\nAvg Freq: {avg_ghz:.2}GHz\nMax Freq: {max_ghz:.2}GHz\nMin Freq: {min_ghz:.2}GHz"
            ),
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
        ModuleMetric {
            lines: vec![
                MetricLine::new("\u{f0a0}", format!("{:>5.1}G Used", self.disk_used_gb)),
                MetricLine::new("\u{f0a0}", format!("{:>5.1}G Free", self.disk_free_gb)),
            ],
            tooltip: format!(
                "Path: /\nTotal: {:.1} GB\nUsed: {:.1} GB ({:.0}%)\nFree: {:.1} GB ({:.0}%)",
                self.disk_total_gb,
                self.disk_used_gb,
                self.disk_pct_used,
                self.disk_free_gb,
                self.disk_pct_free
            ),
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
