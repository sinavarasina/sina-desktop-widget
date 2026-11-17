//! Non-overlapping two-column metric badge shared across bar and drawer modules.

use waywi::prelude::*;

use crate::app::format::{MetricLine, ModuleMetric};
use crate::app::state::{HoverTarget, Msg};
use crate::theme::{
    BAR_HEIGHT, MANTLE, METRIC_ICON_GAP, METRIC_ICON_WIDTH, MODULE_PAD_H, SUBTEXT0, SURFACE0, TEXT,
    font_ui_semibold,
};

pub struct MetricModuleConfig {
    pub key: &'static str,
    pub metric: ModuleMetric,
    pub target: HoverTarget,
    pub hovered: bool,
    pub fixed_width: Option<f32>,
    pub on_pointer_down: Option<fn(PointerEvent) -> Msg>,
    pub on_scroll: Option<fn(ScrollEvent) -> Msg>,
}

/// Renders one bar or drawer telemetry module with dedicated icon columns so PUA glyphs never overlap digits.
pub fn metric_module(cfg: MetricModuleConfig) -> Element<Msg> {
    let dim_inactive = cfg.metric.inactive;
    let bg = if cfg.hovered || dim_inactive {
        SURFACE0
    } else {
        MANTLE
    };
    let fg = if dim_inactive { SUBTEXT0 } else { TEXT };

    let mut lines_col = Column::new()
        .size(Size::auto())
        .gap(0.logical_px())
        .align(Align::Start);
    for line in cfg.metric.lines {
        lines_col = lines_col.child(metric_row(line, fg));
    }

    let container_size = match cfg.fixed_width {
        Some(w) => Size::px(w.logical_px(), BAR_HEIGHT.logical_px()),
        None => Size::new(Length::Auto, BAR_HEIGHT.logical_px()),
    };

    let row_size = match cfg.fixed_width {
        Some(_) => Size::fill(),
        None => Size::new(Length::Auto, BAR_HEIGHT.logical_px()),
    };

    let content_group = Group::new()
        .size(container_size)
        .child(
            Row::new()
                .size(row_size)
                .padding(Edges::symmetric(0.logical_px(), MODULE_PAD_H.logical_px()))
                .align(Align::Center)
                .child(lines_col),
        );

    let mut group = Group::new()
        .key(cfg.key)
        .size(container_size)
        .on_hover(Msg::Hover(cfg.target))
        .on_leave(Msg::Leave(cfg.target))
        .child(Rect::new().size(Size::fill()).fill(bg))
        .child(content_group);

    if let Some(cb) = cfg.on_pointer_down {
        group = group.on_pointer_down(cb);
    }
    if let Some(sc) = cfg.on_scroll {
        group = group.on_scroll(sc);
    }
    group.into()
}

/// Renders one icon + value row, drawing the strike-through across the row itself when the
/// line is muted so only that row's state is struck (output and input stay independent).
fn metric_row(line: MetricLine, fg: Color) -> Element<Msg> {
    let icon_cell = Group::new()
        .size(Size::px(METRIC_ICON_WIDTH.logical_px(), 10.logical_px()))
        .child(
            Text::new(line.icon)
                .font(font_ui_semibold())
                .font_size(8.logical_px())
                .color(fg)
                .position(Position::relative_to_parent(
                    Percent::new(50.0),
                    Percent::new(50.0),
                ))
                .anchor(ElementAnchor::Center),
        );

    let value_el = Text::new(line.text)
        .font(font_ui_semibold())
        .font_size(8.logical_px())
        .line_height(1.1)
        .color(fg);

    let row = Row::new()
        .size(Size::new(Length::Auto, 10.logical_px()))
        .gap(METRIC_ICON_GAP.logical_px())
        .align(Align::Center)
        .child(icon_cell)
        .child(value_el);

    if !line.strikethrough {
        return row.into();
    }

    Group::new()
        .size(Size::new(Length::Auto, 10.logical_px()))
        .child(row)
        .child(
            Rect::new()
                .position(Position::relative_to_parent(
                    Percent::new(50.0),
                    Percent::new(50.0),
                ))
                .anchor(ElementAnchor::Center)
                .size(Size::new(Percent::new(92.0), 1.logical_px()))
                .fill(SUBTEXT0),
        )
        .into()
}
