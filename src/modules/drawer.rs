//! Expandable monitoring drawer (`exp` + `\u{e114}`, Disk, and Bluetooth).

use waywi::prelude::*;

use crate::app::state::{HoverTarget, Msg, State};
use crate::theme::{
    BAR_HEIGHT, BT_EXPANDED_WIDTH, DISK_EXPANDED_WIDTH, DRAWER_REVEALED_WIDTH, EXPAND_SIGN_WIDTH,
    GPU_EXPANDED_WIDTH, MANTLE, MODULE_PAD_H, SUBTEXT0, font_ui,
};
use crate::widgets::metric::{MetricModuleConfig, metric_module};

pub fn build_expandable_mon(state: &State) -> Element<Msg> {
    let p = state.drawer_progress.clamp(0.0, 1.0);
    let mut drawer_row = Row::new()
        .key("expandable-mon")
        .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
        .align(Align::Center);

    let sign_w = ((1.0 - p) * EXPAND_SIGN_WIDTH).round();
    if sign_w >= 1.0 {
        let sign_inner = Group::new()
            .size(Size::px(
                EXPAND_SIGN_WIDTH.logical_px(),
                BAR_HEIGHT.logical_px(),
            ))
            .on_hover(Msg::Hover(HoverTarget::ExpandSign))
            .on_leave(Msg::Leave(HoverTarget::ExpandSign))
            .child(Rect::new().size(Size::fill()).fill(MANTLE))
            .child(
                Row::new()
                    .size(Size::fill())
                    .padding(Edges::symmetric(0.logical_px(), MODULE_PAD_H.logical_px()))
                    .align(Align::Center)
                    .child(
                        Text::new("exp\n  \u{e114}")
                            .font(font_ui())
                            .font_size(8.logical_px())
                            .line_height(1.2)
                            .color(SUBTEXT0.with_opacity(1.0 - p)),
                    ),
            );
        drawer_row = drawer_row.child(
            Clip::new(sign_inner)
                .key("expand-sign-clip")
                .size(Size::px(sign_w.logical_px(), BAR_HEIGHT.logical_px())),
        );
    }

    let revealed_w = (p * DRAWER_REVEALED_WIDTH).round();
    if revealed_w >= 1.0 {
        let gpu_el = metric_module(MetricModuleConfig {
            key: "mod-gpu",
            metric: state.gpu_metric(),
            target: HoverTarget::Gpu,
            hovered: state.hovered == HoverTarget::Gpu,
            fixed_width: Some(GPU_EXPANDED_WIDTH),
            on_pointer_down: Some(|ev| {
                if ev.button == Some(MouseButton::Right) {
                    Msg::SpawnBtop
                } else {
                    Msg::Hover(HoverTarget::Gpu)
                }
            }),
            on_scroll: None,
        });

        let disk_el = metric_module(MetricModuleConfig {
            key: "mod-disk",
            metric: state.disk_metric(),
            target: HoverTarget::Disk,
            hovered: state.hovered == HoverTarget::Disk,
            fixed_width: Some(DISK_EXPANDED_WIDTH),
            on_pointer_down: Some(|ev| {
                if ev.button == Some(MouseButton::Right) {
                    Msg::SpawnBtop
                } else {
                    Msg::Hover(HoverTarget::Disk)
                }
            }),
            on_scroll: None,
        });

        let bt_el = metric_module(MetricModuleConfig {
            key: "mod-bluetooth",
            metric: state.bluetooth_metric(),
            target: HoverTarget::Bluetooth,
            hovered: state.hovered == HoverTarget::Bluetooth,
            fixed_width: Some(BT_EXPANDED_WIDTH),
            on_pointer_down: Some(|ev| {
                if ev.button == Some(MouseButton::Right) {
                    Msg::SpawnBluetui
                } else {
                    Msg::Hover(HoverTarget::Bluetooth)
                }
            }),
            on_scroll: None,
        });

        let inner_revealed = Row::new()
            .size(Size::px(
                DRAWER_REVEALED_WIDTH.logical_px(),
                BAR_HEIGHT.logical_px(),
            ))
            .align(Align::Center)
            .child(gpu_el)
            .child(disk_el)
            .child(bt_el);

        drawer_row = drawer_row.child(
            Clip::new(inner_revealed)
                .key("drawer-reveal-clip")
                .size(Size::px(revealed_w.logical_px(), BAR_HEIGHT.logical_px())),
        );
    }

    drawer_row.into()
}
