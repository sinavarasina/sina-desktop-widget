//! Intrinsically sized floating hover tooltip surface (`Size::auto()`, zero truncation).

use waywi::prelude::*;

use crate::app::state::{HoverTarget, Msg, State};
use crate::theme::{CRUST, POPUP_TOP_MARGIN, SURFACE1, TEXT, font_ui};
use crate::widgets::calendar::build_calendar_tooltip_surface;

/// Builds the floating hover tooltip or calendar surface for the currently hovered target.
pub fn build_tooltip_surface(state: &State) -> Option<Surface<Msg>> {
    if state.hovered == HoverTarget::Clock {
        return Some(build_calendar_tooltip_surface(state));
    }

    let body = match state.hovered {
        HoverTarget::Wireplumber => state.wireplumber_metric().tooltip,
        HoverTarget::Cpu => state.cpu_metric().tooltip,
        HoverTarget::Memory => state.memory_metric().tooltip,
        HoverTarget::Network => state.network_metric().tooltip,
        HoverTarget::Gpu => state.gpu_metric().tooltip,
        HoverTarget::Disk => state.disk_metric().tooltip,
        HoverTarget::Bluetooth => state.bluetooth_metric().tooltip,
        HoverTarget::TrayItem(idx) => {
            let entry = state.tray_items.get(idx)?;
            if let Some(tip) = &entry.snapshot.tooltip
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
            }
        }
        _ => return None,
    };

    let right_margin = state
        .hovered
        .right_margin_hint(state.clock_alt, state.tray_items.len());

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
            .anchor(Anchor::Top | Anchor::Right)
            .margin(Edges::top_right(
                POPUP_TOP_MARGIN.logical_px(),
                right_margin.logical_px(),
            ))
            .size(Size::auto())
            .input_mode(InputMode::PassThrough)
            .keyboard_interactivity(KeyboardInteractivity::None)
            .background(Color::TRANSPARENT)
            .child(card),
    )
}
