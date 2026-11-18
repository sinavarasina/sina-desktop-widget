//! CPU temperature sensor selection and Linux RAPL package power sampling.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use waywi::system::hardware::ThermalSensor;

/// Picks the best CPU die/package temperature reading from hwmon sensors.
#[must_use]
pub fn pick_cpu_temperature(sensors: &[ThermalSensor]) -> Option<f32> {
    let mut best_rank: u8 = u8::MAX;
    let mut best_temp: Option<f32> = None;

    for s in sensors {
        if !s.celsius.is_finite() {
            continue;
        }
        let hw = s.hwmon_name.to_ascii_lowercase();
        let lbl = s.label.to_ascii_lowercase();

        let rank = if matches!(hw.as_str(), "k10temp" | "zenpower" | "coretemp") {
            if lbl.contains("tdie") || lbl.contains("package") || lbl.contains("tctl") {
                0
            } else {
                1
            }
        } else if lbl.contains("cpu") {
            2
        } else if matches!(hw.as_str(), "cpu_thermal" | "acpitz") {
            3
        } else {
            continue;
        };

        if rank < best_rank {
            best_rank = rank;
            best_temp = Some(s.celsius);
            if rank == 0 {
                break;
            }
        }
    }

    best_temp
}

#[derive(Debug, Clone)]
struct RaplDomain {
    key: String,
    energy_path: PathBuf,
    max_range_uj: Option<u64>,
}

/// Computes aggregate CPU package power (watts) from `/sys/class/powercap/intel-rapl:*` counters.
#[derive(Debug, Default)]
pub struct CpuPowerTracker {
    domains: Option<Vec<RaplDomain>>,
    prev_sample: Option<(Instant, HashMap<String, u64>)>,
}

impl CpuPowerTracker {
    /// Samples package energy counters and returns instantaneous average watts since the prior read.
    pub fn sample_now(&mut self, now: Instant) -> Option<f32> {
        self.sample_from_dir(Path::new("/sys/class/powercap"), now)
    }

    pub fn sample_from_dir(&mut self, powercap_dir: &Path, now: Instant) -> Option<f32> {
        let domains = self
            .domains
            .get_or_insert_with(|| discover_rapl_packages(powercap_dir));
        if domains.is_empty() {
            return None;
        }

        let mut current = HashMap::with_capacity(domains.len());
        for d in domains.iter() {
            if let Some(uj) = read_u64(&d.energy_path) {
                current.insert(d.key.clone(), uj);
            }
        }
        if current.is_empty() {
            return None;
        }

        let watts = self.prev_sample.as_ref().and_then(|(prev_at, prev_map)| {
            let dt = now.saturating_duration_since(*prev_at).as_secs_f64();
            if dt < 0.05 {
                return None;
            }
            let mut total_uj: u64 = 0;
            let mut matched = false;
            for d in domains.iter() {
                let (Some(&cur_uj), Some(&prev_uj)) = (current.get(&d.key), prev_map.get(&d.key))
                else {
                    continue;
                };
                if let Some(delta) = energy_delta_uj(prev_uj, cur_uj, d.max_range_uj) {
                    total_uj = total_uj.saturating_add(delta);
                    matched = true;
                }
            }
            if !matched {
                return None;
            }
            let w = (total_uj as f64) / (dt * 1_000_000.0);
            (w.is_finite() && w >= 0.0).then_some(w as f32)
        });

        self.prev_sample = Some((now, current));
        watts
    }
}

fn discover_rapl_packages(powercap_dir: &Path) -> Vec<RaplDomain> {
    let Ok(rd) = fs::read_dir(powercap_dir) else {
        return Vec::new();
    };
    let mut domains = Vec::new();
    for entry in rd.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        // Only top-level package domains (`intel-rapl:0`), never subzones (`intel-rapl:0:0`).
        let Some(suffix) = name.strip_prefix("intel-rapl:") else {
            continue;
        };
        if suffix.is_empty() || !suffix.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        let dir = entry.path();
        let energy_path = dir.join("energy_uj");
        if read_u64(&energy_path).is_none() {
            continue;
        }
        let max_range_uj = read_u64(&dir.join("max_energy_range_uj"));
        domains.push(RaplDomain {
            key: name,
            energy_path,
            max_range_uj,
        });
    }
    domains.sort_by(|a, b| a.key.cmp(&b.key));
    domains
}

fn read_u64(path: &Path) -> Option<u64> {
    fs::read_to_string(path).ok()?.trim().parse::<u64>().ok()
}

fn energy_delta_uj(prev: u64, cur: u64, max_range: Option<u64>) -> Option<u64> {
    if cur >= prev {
        Some(cur - prev)
    } else {
        max_range
            .filter(|&m| m > prev)
            .map(|m| (m - prev).saturating_add(cur))
    }
}

#[cfg(test)]
mod tests {
    use waywi::system::hardware::ThermalSensor;

    use super::{energy_delta_uj, pick_cpu_temperature};

    #[test]
    fn picks_cpu_package_sensor_over_board_sensors() {
        let sensors = vec![
            ThermalSensor {
                hwmon_name: "asusec".into(),
                label: "CPU".into(),
                celsius: 41.0,
                critical_celsius: None,
            },
            ThermalSensor {
                hwmon_name: "k10temp".into(),
                label: "Tctl".into(),
                celsius: 52.5,
                critical_celsius: Some(95.0),
            },
            ThermalSensor {
                hwmon_name: "amdgpu".into(),
                label: "edge".into(),
                celsius: 44.0,
                critical_celsius: None,
            },
        ];
        assert_eq!(pick_cpu_temperature(&sensors), Some(52.5));
    }

    #[test]
    fn computes_rapl_delta_across_wraparound() {
        assert_eq!(energy_delta_uj(1000, 3500, Some(10_000)), Some(2500));
        assert_eq!(energy_delta_uj(9000, 1500, Some(10_000)), Some(2500));
        assert_eq!(energy_delta_uj(9000, 1500, None), None);
    }
}
