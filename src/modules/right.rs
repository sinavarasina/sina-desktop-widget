//! Right bar group: Privacy, auto-hiding MPRIS, Clipboard, system tray, dynamically ordered/placed
//! monitoring metrics (Audio, CPU, Memory, Network, GPU, Disk, Bluetooth), expandable drawer, and clock.

use waywi::prelude::*;

use crate::app::config::RightMetricId;
use crate::app::state::{HoverTarget, Msg, State, TelemetryGraphKind};
use crate::modules::{clock, drawer, mpris, status_pill, tray};
use crate::theme::{BAR_HEIGHT, MANTLE, SURFACE0};
use crate::widgets::metric::{MetricModuleConfig, metric_module};
use crate::widgets::separators::{SepShape, separator};

/// Builds a single right-side monitoring metric widget, shared cleanly by both the main bar
/// (`fixed_width = None`) and the animated expandable drawer (`fixed_width = Some(w)`).
pub fn build_right_metric(
    state: &State,
    id: RightMetricId,
    fixed_width: Option<f32>,
) -> Element<Msg> {
    match id {
        RightMetricId::Audio => metric_module(MetricModuleConfig {
            key: "mod-wireplumber",
            metric: state.wireplumber_metric(),
            target: HoverTarget::Wireplumber,
            hovered: state.hovered == HoverTarget::Wireplumber,
            fixed_width,
            on_pointer_down: Some(|ev: PointerEvent| match ev.button {
                // Left opens the mixer popup, right toggles mute (inverted on purpose);
                // middle-click keeps switching the sink/source control target.
                Some(MouseButton::Left) | Some(MouseButton::Right) => Msg::MixerBarPointerDown(ev),
                _ => Msg::AudioPointerDown(ev),
            }),
            on_scroll: Some(Msg::AudioScroll),
        }),
        RightMetricId::Cpu => metric_module(MetricModuleConfig {
            key: "mod-cpu",
            metric: state.cpu_metric(),
            target: HoverTarget::Cpu,
            hovered: state.hovered == HoverTarget::Cpu,
            fixed_width,
            on_pointer_down: Some(|ev: PointerEvent| Msg::TelemetryBarPointerDown {
                kind: TelemetryGraphKind::Cpu,
                ev,
            }),
            on_scroll: None,
        }),
        RightMetricId::Memory => metric_module(MetricModuleConfig {
            key: "mod-memory",
            metric: state.memory_metric(),
            target: HoverTarget::Memory,
            hovered: state.hovered == HoverTarget::Memory,
            fixed_width,
            on_pointer_down: Some(|ev: PointerEvent| Msg::TelemetryBarPointerDown {
                kind: TelemetryGraphKind::Memory,
                ev,
            }),
            on_scroll: None,
        }),
        RightMetricId::Network => metric_module(MetricModuleConfig {
            key: "mod-network",
            metric: state.network_metric(),
            target: HoverTarget::Network,
            hovered: state.hovered == HoverTarget::Network,
            fixed_width,
            on_pointer_down: Some(|ev: PointerEvent| Msg::TelemetryBarPointerDown {
                kind: TelemetryGraphKind::Network,
                ev,
            }),
            on_scroll: None,
        }),
        RightMetricId::Gpu => metric_module(MetricModuleConfig {
            key: "mod-gpu",
            metric: state.gpu_metric(),
            target: HoverTarget::Gpu,
            hovered: state.hovered == HoverTarget::Gpu,
            fixed_width,
            on_pointer_down: Some(|ev: PointerEvent| Msg::TelemetryBarPointerDown {
                kind: TelemetryGraphKind::Gpu,
                ev,
            }),
            on_scroll: None,
        }),
        RightMetricId::Disk => metric_module(MetricModuleConfig {
            key: "mod-disk",
            metric: state.disk_metric(),
            target: HoverTarget::Disk,
            hovered: state.hovered == HoverTarget::Disk,
            fixed_width,
            on_pointer_down: Some(Msg::DiskBarPointerDown),
            on_scroll: None,
        }),
        RightMetricId::Bluetooth => metric_module(MetricModuleConfig {
            key: "mod-bluetooth",
            metric: state.bluetooth_metric(),
            target: HoverTarget::Bluetooth,
            hovered: state.hovered == HoverTarget::Bluetooth,
            fixed_width,
            on_pointer_down: Some(Msg::BtBarPointerDown),
            on_scroll: None,
        }),
    }
}

pub fn build_right_modules(state: &State) -> Row<Msg> {
    let w = &state.config.widgets;
    let (bar_ids, drawer_ids) = state.effective_right_metrics();
    let has_mantle_strip = w.has_any_right_mantle();

    let mut row = Row::new()
        .key("modules-right")
        .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
        .align(Align::Center)
        .child(status_pill::build_privacy_pill(state, w.show_mpris));

    if w.show_mpris {
        row = row.child(mpris::build_mpris(state));
    }

    row = row.child(status_pill::build_clipboard_pill(state));

    if has_mantle_strip {
        row = row.child(separator(
            "sep-r-1",
            SepShape::SlashL,
            MANTLE,
            Color::TRANSPARENT,
        ));
        if w.show_tray {
            row = row.child(tray::build_tray(state));
        }
        for id in bar_ids {
            row = row.child(build_right_metric(state, id, None));
        }
        if !drawer_ids.is_empty() {
            row = row.child(drawer::build_expandable_mon(state, &drawer_ids));
        }
        row = row.child(separator(
            "sep-r-2",
            SepShape::CurveR,
            MANTLE,
            Color::TRANSPARENT,
        ));
    }

    row.child(separator(
        "sep-r-3",
        SepShape::ArrowLThin,
        SURFACE0,
        Color::TRANSPARENT,
    ))
    .child(clock::build_clock_group(state))
}
