//! Fixed-capacity zero-allocation telemetry ring buffers and popup state for the reusable
//! live graph overlay (`CPU`, `Memory`, `GPU`, `Network`).

use std::time::Instant;

use waywi::system::{CpuSnapshot, GpuSnapshot, MemorySnapshot, NetworkSnapshot};

use super::anim::RevealAnim;
use super::config::RightMetricId;

/// Number of historical telemetry samples kept per series (~60s at 1Hz, ~2m at 0.5Hz).
pub const GRAPH_HISTORY_CAP: usize = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TelemetryGraphKind {
    Cpu,
    Memory,
    Gpu,
    Network,
}

impl TelemetryGraphKind {
    pub const ALL: [Self; 4] = [Self::Cpu, Self::Memory, Self::Gpu, Self::Network];

    #[must_use]
    pub const fn id_u8(self) -> u8 {
        match self {
            Self::Cpu => 0,
            Self::Memory => 1,
            Self::Gpu => 2,
            Self::Network => 3,
        }
    }

    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Cpu => "CPU Telemetry",
            Self::Memory => "Memory Telemetry",
            Self::Gpu => "GPU Telemetry",
            Self::Network => "Network Telemetry",
        }
    }

    #[must_use]
    pub const fn tab_label(self) -> &'static str {
        match self {
            Self::Cpu => "CPU",
            Self::Memory => "RAM",
            Self::Gpu => "GPU",
            Self::Network => "NET",
        }
    }

    #[must_use]
    pub const fn icon(self) -> &'static str {
        match self {
            Self::Cpu => "\u{f2db}",
            Self::Memory => "\u{efc5}",
            Self::Gpu => "\u{f08ae}",
            Self::Network => "\u{f063}",
        }
    }

    #[must_use]
    pub const fn metric_id(self) -> RightMetricId {
        match self {
            Self::Cpu => RightMetricId::Cpu,
            Self::Memory => RightMetricId::Memory,
            Self::Gpu => RightMetricId::Gpu,
            Self::Network => RightMetricId::Network,
        }
    }
}

/// Fixed-capacity stack circular buffer for real-time telemetry samples (zero heap allocations).
#[derive(Debug, Clone)]
pub struct HistorySeries {
    data: [f32; GRAPH_HISTORY_CAP],
    len: usize,
    head: usize,
}

impl Default for HistorySeries {
    fn default() -> Self {
        Self {
            data: [0.0; GRAPH_HISTORY_CAP],
            len: 0,
            head: 0,
        }
    }
}

impl HistorySeries {
    pub fn push(&mut self, value: f32) {
        let v = if value.is_finite() && value >= 0.0 {
            value
        } else {
            0.0
        };
        self.data[self.head] = v;
        self.head = (self.head + 1) % GRAPH_HISTORY_CAP;
        if self.len < GRAPH_HISTORY_CAP {
            self.len += 1;
        }
    }

    #[cfg(test)]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Copies samples in chronological order (oldest -> newest) into `out` and returns the count.
    pub fn copy_chronological(&self, out: &mut [f32; GRAPH_HISTORY_CAP]) -> usize {
        if self.len == 0 {
            return 0;
        }
        let start = if self.len == GRAPH_HISTORY_CAP {
            self.head
        } else {
            0
        };
        for (i, slot) in out.iter_mut().take(self.len).enumerate() {
            *slot = self.data[(start + i) % GRAPH_HISTORY_CAP];
        }
        self.len
    }

    #[cfg(test)]
    #[must_use]
    pub fn latest(&self) -> f32 {
        if self.len == 0 {
            return 0.0;
        }
        let idx = (self.head + GRAPH_HISTORY_CAP - 1) % GRAPH_HISTORY_CAP;
        self.data[idx]
    }

    #[must_use]
    pub fn peak(&self) -> f32 {
        self.data
            .iter()
            .take(self.len)
            .copied()
            .fold(0.0_f32, f32::max)
    }
}

pub struct TelemetryGraphState {
    pub active: TelemetryGraphKind,
    pub popup_anim: RevealAnim,
    pub popup_left_x: f32,
    pub sample_seq: u64,
    pub cpu_usage: HistorySeries,
    pub mem_usage: HistorySeries,
    pub swap_usage: HistorySeries,
    pub gpu_usage: HistorySeries,
    pub gpu_vram: HistorySeries,
    pub gpu_clock_mhz: HistorySeries,
    pub net_rx_bps: HistorySeries,
    pub net_tx_bps: HistorySeries,
}

impl Default for TelemetryGraphState {
    fn default() -> Self {
        Self {
            active: TelemetryGraphKind::Cpu,
            popup_anim: RevealAnim::popup(),
            popup_left_x: 0.0,
            sample_seq: 0,
            cpu_usage: HistorySeries::default(),
            mem_usage: HistorySeries::default(),
            swap_usage: HistorySeries::default(),
            gpu_usage: HistorySeries::default(),
            gpu_vram: HistorySeries::default(),
            gpu_clock_mhz: HistorySeries::default(),
            net_rx_bps: HistorySeries::default(),
            net_tx_bps: HistorySeries::default(),
        }
    }
}

impl TelemetryGraphState {
    #[must_use]
    pub const fn is_popup_open(&self) -> bool {
        self.popup_anim.is_open()
    }

    #[must_use]
    pub fn is_popup_visible(&self) -> bool {
        self.popup_anim.is_visible()
    }

    #[must_use]
    pub fn is_animating_popup(&self) -> bool {
        self.popup_anim.is_animating()
    }

    #[must_use]
    pub fn popup_progress(&self) -> f32 {
        self.popup_anim.progress()
    }

    pub fn open(&mut self, kind: TelemetryGraphKind, left_x: f32) {
        self.active = kind;
        self.popup_left_x = left_x;
        self.popup_anim.set_open(true);
    }

    pub fn select(&mut self, kind: TelemetryGraphKind) {
        self.active = kind;
    }

    pub fn close(&mut self) {
        self.popup_anim.set_open(false);
    }

    pub fn advance_popup_animation(&mut self, now: Instant) {
        self.popup_anim.advance(now);
    }

    pub fn record_cpu(&mut self, snap: &CpuSnapshot) {
        self.cpu_usage.push(snap.total_percent.clamp(0.0, 100.0));
        self.bump_seq();
    }

    pub fn record_memory(&mut self, snap: &MemorySnapshot) {
        let total = snap.total.as_bytes() as f64;
        let used_pct = if total > 0.0 {
            ((snap.used.as_bytes() as f64 / total) * 100.0) as f32
        } else {
            0.0
        };
        let swap_total = snap.swap_total.as_bytes() as f64;
        let swap_pct = if swap_total > 0.0 {
            ((snap.swap_used.as_bytes() as f64 / swap_total) * 100.0) as f32
        } else {
            0.0
        };
        self.mem_usage.push(used_pct.clamp(0.0, 100.0));
        self.swap_usage.push(swap_pct.clamp(0.0, 100.0));
        self.bump_seq();
    }

    pub fn record_gpu(&mut self, snap: &GpuSnapshot) {
        let Some(dev) = snap.gpus.first() else {
            return;
        };
        let util = dev.utilization_percent.unwrap_or(0.0).clamp(0.0, 100.0);
        let vram_pct = match (dev.vram_used, dev.vram_total) {
            (Some(u), Some(t)) if t.as_bytes() > 0 => {
                ((u.as_bytes() as f64 / t.as_bytes() as f64) * 100.0) as f32
            }
            _ => 0.0,
        };
        self.gpu_usage.push(util);
        self.gpu_vram.push(vram_pct.clamp(0.0, 100.0));
        self.gpu_clock_mhz
            .push(dev.core_clock_mhz.unwrap_or(0) as f32);
        self.bump_seq();
    }

    pub fn record_network(&mut self, snap: &NetworkSnapshot) {
        let (rx, tx) = snap
            .active_interface()
            .map_or((0.0, 0.0), |i| (i.rx_bytes_per_sec, i.tx_bytes_per_sec));
        self.net_rx_bps.push(rx.max(0.0) as f32);
        self.net_tx_bps.push(tx.max(0.0) as f32);
        self.bump_seq();
    }

    fn bump_seq(&mut self) {
        self.sample_seq = self.sample_seq.wrapping_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::{GRAPH_HISTORY_CAP, HistorySeries};

    #[test]
    fn history_series_wraps_in_chronological_order() {
        let mut s = HistorySeries::default();
        for i in 0..(GRAPH_HISTORY_CAP + 5) {
            s.push(i as f32);
        }
        assert_eq!(s.len(), GRAPH_HISTORY_CAP);
        let mut buf = [0.0; GRAPH_HISTORY_CAP];
        let n = s.copy_chronological(&mut buf);
        assert_eq!(n, GRAPH_HISTORY_CAP);
        assert_eq!(buf[0], 5.0);
        assert_eq!(buf[n - 1], (GRAPH_HISTORY_CAP + 4) as f32);
        assert_eq!(s.latest(), (GRAPH_HISTORY_CAP + 4) as f32);
        assert_eq!(s.peak(), (GRAPH_HISTORY_CAP + 4) as f32);
    }
}
