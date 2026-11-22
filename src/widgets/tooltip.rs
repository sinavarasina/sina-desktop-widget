//! Intrinsically sized floating hover tooltip surface (`Size::auto()`, zero truncation).

use waywi::prelude::*;

use crate::app::state::{HoverTarget, Msg, State};
use crate::modules::left::workspaces::hovered_workspace_tooltip;
use crate::theme::{CRUST, POPUP_TOP_MARGIN, SURFACE1, TEXT, font_ui};

/// Builds the floating hover tooltip surface for the currently hovered target.
pub fn build_tooltip_surface(state: &State) -> Option<Surface<Msg>> {
    let (body, left_anchor_margin) = match state.hovered {
        HoverTarget::SysInfo => (
            state
                .sysinfo
                .tooltip(state.cpu.as_ref(), state.memory.as_ref()),
            Some(
                state
                    .hover_anchor_x
                    .unwrap_or(38.0)
                    .clamp(8.0, (state.bar_width - 120.0).max(8.0)),
            ),
        ),
        HoverTarget::ActiveWs
        | HoverTarget::InactiveWs(_)
        | HoverTarget::NamedWs(_)
        | HoverTarget::SpecialWs(_) => (
            hovered_workspace_tooltip(state)?,
            Some(
                state
                    .hover_anchor_x
                    .unwrap_or(140.0)
                    .clamp(8.0, (state.bar_width - 140.0).max(8.0)),
            ),
        ),
        HoverTarget::WindowInfo => (
            state.window_info.tooltip(
                state
                    .booster
                    .tooltip_line(state.window_info.cgroup.as_ref()),
            ),
            Some(
                state
                    .hover_anchor_x
                    .unwrap_or(340.0)
                    .clamp(8.0, (state.bar_width - 160.0).max(8.0)),
            ),
        ),
        HoverTarget::Wireplumber => (state.wireplumber_metric().tooltip, None),
        HoverTarget::Cpu => (state.cpu_metric().tooltip, None),
        HoverTarget::Memory => (state.memory_metric().tooltip, None),
        HoverTarget::Network => (state.network_metric().tooltip, None),
        HoverTarget::Gpu => (state.gpu_metric().tooltip, None),
        HoverTarget::Temp => (state.temp_metric().tooltip, None),
        HoverTarget::Disk => (state.disk_metric().tooltip, None),
        HoverTarget::Bluetooth => (state.bluetooth_metric().tooltip, None),
        HoverTarget::TrayItem(idx) => {
            let entry = state.tray_items.get(idx)?;
            let text = if let Some(tip) = &entry.snapshot.tooltip
                && (!tip.title.is_empty() || !tip.description.is_empty())
            {
                if tip.description.is_empty() {
                    tip.title.clone()
                } else {
                    format!("{}\n{}", tip.title, tip.description)
                }
            } else if !entry.snapshot.title.is_empty() {
                entry.snapshot.title.clone()
            } else {
                entry.snapshot.id.clone()
            };
            (text, None)
        }
        _ => return None,
    };

    let (anchor, margin) = if let Some(left_margin) = left_anchor_margin {
        (
            Anchor::Top | Anchor::Left,
            Edges::new(
                POPUP_TOP_MARGIN.logical_px(),
                0.logical_px(),
                0.logical_px(),
                left_margin.logical_px(),
            ),
        )
    } else {
        let right_margin = state
            .hovered
            .right_margin_hint(state.clock_alt, state.tray_items.len());
        (
            Anchor::Top | Anchor::Right,
            Edges::top_right(POPUP_TOP_MARGIN.logical_px(), right_margin.logical_px()),
        )
    };

    let mut lines_col = Column::new()
        .size(Size::auto())
        .padding(Edges::symmetric(8.logical_px(), 10.logical_px()))
        .gap(2.logical_px());
    for line in body.lines() {
        let display_line = if line.is_empty() { " " } else { line };
        lines_col = lines_col.child(
            Text::new(display_line.to_owned())
                .font(font_ui())
                .font_size(11.logical_px())
                .line_height(1.2)
                .color(TEXT),
        );
    }

    let card = Group::new()
        .size(Size::auto())
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(CRUST)
                .corner_radius(6.logical_px())
                .border(1.logical_px(), SURFACE1),
        )
        .child(lines_col);

    Some(
        Surface::floating("bar-tooltip", Output::Primary)
            .layer(Layer::Overlay)
            .ignore_exclusive_zone()
            .anchor(anchor)
            .margin(margin)
            .size(Size::auto())
            .input_mode(InputMode::PassThrough)
            .keyboard_interactivity(KeyboardInteractivity::None)
            .background(Color::TRANSPARENT)
            .child(card),
    )
}
