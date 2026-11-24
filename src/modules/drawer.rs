//! Dynamic expandable monitoring drawer (`exp` + `\u{e114}`) rendering whichever right-side
//! metrics are assigned to `MetricPlacement::Drawer` in the user's configured order.

use waywi::prelude::*;

use crate::app::config::RightMetricId;
use crate::app::state::{HoverTarget, Msg, State};
use crate::modules::right::build_right_metric;
use crate::theme::{BAR_HEIGHT, EXPAND_SIGN_WIDTH, MANTLE, MODULE_PAD_H, SUBTEXT0, font_ui};

pub fn build_expandable_mon(state: &State, metrics: &[RightMetricId]) -> Element<Msg> {
    let p = state.drawer_progress.clamp(0.0, 1.0);
    let mut drawer_row = Row::new()
        .key("expandable-mon")
        .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
        .align(Align::Center);

    let sign_w = ((1.0 - p) * EXPAND_SIGN_WIDTH).round();
    if sign_w >= 1.0 {
        let sign_inner = Group::new()
            .size(Size::new(
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
                .size(Size::new(sign_w.logical_px(), BAR_HEIGHT.logical_px())),
        );
    }

    let total_revealed_w: f32 = metrics.iter().map(|id| id.drawer_width()).sum();
    let revealed_w = (p * total_revealed_w).round();
    if revealed_w >= 1.0 && total_revealed_w >= 1.0 {
        let mut inner_revealed = Row::new()
            .size(Size::new(
                total_revealed_w.logical_px(),
                BAR_HEIGHT.logical_px(),
            ))
            .align(Align::Center);

        for &id in metrics {
            inner_revealed =
                inner_revealed.child(build_right_metric(state, id, Some(id.drawer_width())));
        }

        drawer_row = drawer_row.child(
            Clip::new(inner_revealed)
                .key("drawer-reveal-clip")
                .size(Size::new(revealed_w.logical_px(), BAR_HEIGHT.logical_px())),
        );
    }

    drawer_row.into()
}
