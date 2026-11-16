//! Right bar group: system tray, PipeWire audio, CPU, memory, network, drawer, and clock.

use waywi::prelude::*;

use crate::app::state::{HoverTarget, Msg, State};
use crate::modules::{clock, drawer, mpris, tray};
use crate::theme::{BAR_HEIGHT, CRUST, MANTLE, SURFACE0};
use crate::widgets::metric::{MetricModuleConfig, metric_module};
use crate::widgets::separators::{SepShape, separator};

pub fn build_right_modules(state: &State) -> Row<Msg> {
    let wireplumber = metric_module(MetricModuleConfig {
        key: "mod-wireplumber",
        metric: state.wireplumber_metric(),
        target: HoverTarget::Wireplumber,
        hovered: state.hovered == HoverTarget::Wireplumber,
        fixed_width: None,
        on_pointer_down: Some(|ev: PointerEvent| match ev.button {
            Some(MouseButton::Right) => Msg::LaunchApp {
                prog: "hyprpwcenter",
                args: &[],
            },
            _ => Msg::AudioPointerDown(ev),
        }),
        on_scroll: Some(Msg::AudioScroll),
    });

    let cpu = metric_module(MetricModuleConfig {
        key: "mod-cpu",
        metric: state.cpu_metric(),
        target: HoverTarget::Cpu,
        hovered: state.hovered == HoverTarget::Cpu,
        fixed_width: None,
        on_pointer_down: Some(|ev: PointerEvent| {
            if ev.button == Some(MouseButton::Right) {
                Msg::SpawnBtop
            } else {
                Msg::Hover(HoverTarget::Cpu)
            }
        }),
        on_scroll: None,
    });

    let memory = metric_module(MetricModuleConfig {
        key: "mod-memory",
        metric: state.memory_metric(),
        target: HoverTarget::Memory,
        hovered: state.hovered == HoverTarget::Memory,
        fixed_width: None,
        on_pointer_down: Some(|ev: PointerEvent| {
            if ev.button == Some(MouseButton::Right) {
                Msg::SpawnBtop
            } else {
                Msg::Hover(HoverTarget::Memory)
            }
        }),
        on_scroll: None,
    });

    let network = metric_module(MetricModuleConfig {
        key: "mod-network",
        metric: state.network_metric(),
        target: HoverTarget::Network,
        hovered: state.hovered == HoverTarget::Network,
        fixed_width: None,
        on_pointer_down: None,
        on_scroll: None,
    });

    Row::new()
        .key("modules-right")
        .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
        .align(Align::Center)
        .child(mpris::build_mpris(state))
        .child(separator(
            "sep-r-1",
            SepShape::SlashL,
            MANTLE,
            Color::TRANSPARENT,
        ))
        .child(tray::build_tray(state))
        .child(wireplumber)
        .child(cpu)
        .child(memory)
        .child(network)
        .child(drawer::build_expandable_mon(state))
        .child(separator(
            "sep-r-2",
            SepShape::CurveR,
            MANTLE,
            Color::TRANSPARENT,
        ))
        .child(separator(
            "sep-r-3",
            SepShape::ArrowLThin,
            SURFACE0,
            Color::TRANSPARENT,
        ))
        .child(separator(
            "sep-r-4",
            SepShape::ArrowL,
            CRUST,
            Color::TRANSPARENT,
        ))
        .child(clock::build_clock(state))
}
