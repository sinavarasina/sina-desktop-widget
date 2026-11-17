//! Network interface and NetworkManager formatting.

use waywi::system::{NetInterfaceKind, RateUnit};

use crate::app::state::State;

use super::{MetricLine, ModuleMetric};

impl State {
    #[must_use]
    pub fn network_metric(&self) -> ModuleMetric {
        let disconnected = || ModuleMetric {
            lines: vec![MetricLine::new("\u{f092d}", "0/0").strikethrough()],
            tooltip: String::from("Disconnected"),
            inactive: true,
        };
        let Some(net) = &self.network else {
            return disconnected();
        };
        let Some(iface) = net.active_interface() else {
            return disconnected();
        };

        let up = iface.tx_rate().format(RateUnit::AutoBytes);
        let down = iface.rx_rate().format(RateUnit::AutoBytes);

        let nm_dev = self
            .nm
            .as_ref()
            .and_then(|nm| nm.devices.iter().find(|d| d.interface == iface.name));
        let ip = nm_dev
            .and_then(|d| d.ip4_address.as_deref())
            .or(iface.ipv4.as_deref())
            .unwrap_or("0.0.0.0");
        let gw = iface.gateway.as_deref().unwrap_or("0.0.0.0");

        let tooltip = match iface.kind {
            NetInterfaceKind::Wifi => {
                let essid = nm_dev
                    .and_then(|d| d.active_ssid.as_deref())
                    .unwrap_or("Wi-Fi");
                let strength = nm_dev.and_then(|d| d.strength).unwrap_or(0);
                let dbm = iface.wireless_signal_dbm.unwrap_or(0);
                let freq = iface
                    .wireless_freq_ghz()
                    .map_or_else(|| String::from("0.0"), |g| format!("{g:.1}"));
                format!(
                    "{} via {essid}\nSignal: {strength}% ({dbm} dBm)\nIP: {ip}\nGateway: {gw}\nUp: {up}\nDown: {down}\nFrequency: {freq} GHz",
                    iface.name
                )
            }
            NetInterfaceKind::Modem => format!(
                "{} (Modem)\nIP: {ip}\nGateway: {gw}\nUp: {up}\nDown: {down}",
                iface.name
            ),
            _ => format!(
                "{} (Ethernet)\nIP: {ip}\nGateway: {gw}\nUp: {up}\nDown: {down}",
                iface.name
            ),
        };

        ModuleMetric {
            lines: vec![
                MetricLine::new("\u{f062}", up),
                MetricLine::new("\u{f063}", down),
            ],
            tooltip,
            inactive: false,
        }
    }

    #[cfg(test)]
    #[must_use]
    pub fn network_display(&self) -> (String, String, bool) {
        let m = self.network_metric();
        (m.joined_text(), m.tooltip, m.inactive)
    }
}
