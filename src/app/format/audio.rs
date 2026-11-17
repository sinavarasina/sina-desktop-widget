//! PipeWire audio sink/source formatting.

use crate::app::state::{AudioControlTarget, State};

use super::{MetricLine, ModuleMetric};

impl State {
    #[must_use]
    pub fn wireplumber_metric(&self) -> ModuleMetric {
        let Some(audio) = &self.audio else {
            return ModuleMetric {
                lines: vec![
                    MetricLine::new("\u{f028}", "0%"),
                    MetricLine::new("\u{f036c}", "0%"),
                ],
                tooltip: String::from("Device: PipeWire"),
                inactive: false,
            };
        };
        let sink_pct = audio.percent().round() as u16;
        let src_pct = audio.source_percent().round() as u16;
        let sink_icon = if sink_pct <= 33 {
            "\u{f026}"
        } else if sink_pct <= 66 {
            "\u{f027}"
        } else {
            "\u{f028}"
        };
        let dev = audio
            .default_sink
            .as_ref()
            .map(|d| {
                if d.description.is_empty() {
                    d.name.as_str()
                } else {
                    d.description.as_str()
                }
            })
            .unwrap_or("Default Sink");

        let src_dev = audio
            .default_source
            .as_ref()
            .map(|d| {
                if d.description.is_empty() {
                    d.name.as_str()
                } else {
                    d.description.as_str()
                }
            })
            .unwrap_or("Default Source");

        let sink_active = self.audio_target == AudioControlTarget::Sink;
        let sink_line = if audio.muted {
            MetricLine::new("\u{f075f}", "0%").strikethrough()
        } else {
            MetricLine::new(sink_icon, format!("{sink_pct}%"))
        };
        let src_line = if audio.source_muted {
            MetricLine::new("\u{f036d}", if sink_active { "0%" } else { "0%*" }).strikethrough()
        } else {
            MetricLine::new(
                "\u{f036c}",
                if sink_active {
                    format!("{src_pct}%")
                } else {
                    format!("{src_pct}%*")
                },
            )
        };
        let control_hint = if sink_active {
            "Control: Speaker (Middle-click: Mic)"
        } else {
            "Control: Mic (Middle-click: Speaker)"
        };

        ModuleMetric {
            lines: vec![sink_line, src_line],
            tooltip: format!("Device: {dev}\nMic: {src_dev}\n{control_hint}"),
            inactive: if sink_active {
                audio.muted
            } else {
                audio.source_muted
            },
        }
    }

    #[cfg(test)]
    #[must_use]
    pub fn wireplumber_display(&self) -> (String, String, bool) {
        let m = self.wireplumber_metric();
        (m.joined_text(), m.tooltip, m.inactive)
    }
}
