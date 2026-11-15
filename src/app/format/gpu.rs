//! GPU telemetry formatting.

use crate::app::state::State;

use super::{MetricLine, ModuleMetric};

impl State {
    #[must_use]
    pub fn gpu_metric(&self) -> ModuleMetric {
        let Some(gpu) = self.gpu.as_ref().and_then(|g| g.gpus.first()) else {
            return ModuleMetric {
                lines: vec![
                    MetricLine::new("\u{f08ae}", "0MHz"),
                    MetricLine::new("\u{f06a5}", "0W"),
                ],
                tooltip: String::from("GPU: Unavailable"),
                inactive: false,
            };
        };

        let sclk = gpu.core_clock_mhz.unwrap_or(0);
        let line1 = MetricLine::new("\u{f08ae}", format!("{sclk}MHz"));
        let line2 = if let Some(watts) = gpu.power_watts {
            MetricLine::new("\u{f06a5}", format!("{watts:.0}W"))
        } else {
            let mclk = gpu.memory_clock_mhz.unwrap_or(0);
            MetricLine::new("\u{f061a}", format!("{mclk}MHz"))
        };

        let gpu_label = gpu.model_name.as_deref().unwrap_or(&gpu.card_name);
        let mut tip_parts = vec![format!("GPU: {gpu_label}")];
        tip_parts.push(format!("Core Clock: {sclk} MHz"));
        if let Some(mclk) = gpu.memory_clock_mhz {
            tip_parts.push(format!("Mem Clock: {mclk} MHz"));
        }
        if let Some(watts) = gpu.power_watts {
            tip_parts.push(format!("Power: {watts:.1} W"));
        }
        if let Some(util) = gpu.utilization_percent {
            tip_parts.push(format!("Usage: {util:.0}%"));
        }
        if let Some(temp) = gpu.temperature_celsius {
            tip_parts.push(format!("Temp: {temp:.0}°C"));
        }

        ModuleMetric {
            lines: vec![line1, line2],
            tooltip: tip_parts.join("\n"),
            inactive: false,
        }
    }
}
