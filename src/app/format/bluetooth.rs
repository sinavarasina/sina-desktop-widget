//! Bluetooth adapter and connected device formatting.

use waywi::dbus::BluetoothDevice;

use crate::app::state::State;

use super::{MetricLine, ModuleMetric};

impl State {
    #[must_use]
    pub fn bluetooth_metric(&self) -> ModuleMetric {
        let adapter = self.bluetooth.adapters.first();
        let powered = adapter.is_some_and(|a| a.powered);
        let adapter_alias = adapter.map_or("Bluetooth", |a| {
            if a.alias.is_empty() {
                a.name.as_str()
            } else {
                a.alias.as_str()
            }
        });

        let connected: Vec<&BluetoothDevice> = self
            .bluetooth
            .devices
            .iter()
            .filter(|d| d.connected)
            .collect();

        if !powered || adapter.is_none() {
            return ModuleMetric {
                lines: vec![
                    MetricLine::new("\u{f00b2}", "Disabled"),
                    MetricLine::new("\u{f0949}", "N/A"),
                ],
                tooltip: format!("Controller: {adapter_alias}\nStatus: disabled"),
                inactive: false,
            };
        }

        if connected.is_empty() {
            return ModuleMetric {
                lines: vec![
                    MetricLine::new("\u{f294}", "on"),
                    MetricLine::new("\u{f0949}", "N/A"),
                ],
                tooltip: format!("Controller: {adapter_alias}\nStatus: on"),
                inactive: false,
            };
        }

        let batt = connected
            .iter()
            .find_map(|d| d.battery_percentage)
            .map_or_else(|| String::from("N/A"), |p| format!("{p}%"));

        let list = connected
            .iter()
            .map(|d| {
                let name = if d.alias.is_empty() {
                    d.name.as_str()
                } else {
                    d.alias.as_str()
                };
                let dbatt = d
                    .battery_percentage
                    .map_or_else(|| String::from("N/A"), |b| format!("{b}%"));
                format!("\u{f00b1} {name} ({dbatt} Battery)")
            })
            .collect::<Vec<_>>()
            .join("\n");

        ModuleMetric {
            lines: vec![
                MetricLine::new("\u{f00b1}", "connected"),
                MetricLine::new("\u{f0949}", batt),
            ],
            tooltip: format!(
                "Controller: {adapter_alias}\nConnections: {}\n\n{list}",
                connected.len()
            ),
            inactive: false,
        }
    }

    #[cfg(test)]
    #[must_use]
    pub fn bluetooth_display(&self) -> (String, String) {
        let m = self.bluetooth_metric();
        (m.joined_text(), m.tooltip)
    }
}
