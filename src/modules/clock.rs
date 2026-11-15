//! Right-edge clock pill with alternate-date toggle and scroll/year calendar controls.

use waywi::prelude::*;

use crate::app::state::{HoverTarget, Msg, State};
use crate::theme::{BAR_HEIGHT, CLOCK_PAD_H, CRUST, TEXT, font_ui_bold};

pub fn build_clock(state: &State) -> Element<Msg> {
    let text = state.clock_display();
    Group::new()
        .key("mod-clock")
        .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
        .on_hover(Msg::Hover(HoverTarget::Clock))
        .on_leave(Msg::Leave(HoverTarget::Clock))
        .on_pointer_down(Msg::ClockPointerDown)
        .on_scroll(Msg::ClockScroll)
        .child(Rect::new().size(Size::fill()).fill(CRUST))
        .child(
            Row::new()
                .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
                .padding(Edges::symmetric(0.logical_px(), CLOCK_PAD_H.logical_px()))
                .align(Align::Center)
                .child(
                    Text::new(text)
                        .font(font_ui_bold())
                        .font_size(12.logical_px())
                        .line_height(1.15)
                        .color(TEXT),
                ),
        )
        .into()
}
