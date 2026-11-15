//! Structured telemetry line and tooltip formatting for bar modules.

mod audio;
mod bluetooth;
mod gpu;
mod network;
mod system;

/// One icon + text metric row rendered in separate layout columns so PUA icons never collide with digits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricLine {
    pub icon: &'static str,
    pub text: String,
}

impl MetricLine {
    #[must_use]
    pub fn new(icon: &'static str, text: impl Into<String>) -> Self {
        Self {
            icon,
            text: text.into(),
        }
    }
}

/// Complete bar display data and hover tooltip for one telemetry module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleMetric {
    pub lines: Vec<MetricLine>,
    pub tooltip: String,
    pub inactive: bool,
}

impl ModuleMetric {
    #[cfg(test)]
    #[must_use]
    pub fn joined_text(&self) -> String {
        self.lines
            .iter()
            .map(|l| format!("{} {}", l.icon, l.text))
            .collect::<Vec<_>>()
            .join("\n")
    }
}
