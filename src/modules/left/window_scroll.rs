//! Active window title scroller module.

use waywi::prelude::*;

use crate::app::state::{HoverTarget, Msg, State};
use crate::theme::{BAR_HEIGHT, CRUST, SUBTEXT1, WINDOW_PAD_H, font_window_scroll};

pub fn build_window_scroll(state: &State) -> Element<Msg> {
    Group::new()
        .key("window-scroll")
        .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
        .on_hover(Msg::Hover(HoverTarget::WindowScroll))
        .on_leave(Msg::Leave(HoverTarget::WindowScroll))
        .child(Rect::new().size(Size::fill()).fill(CRUST))
        .child(
            Row::new()
                .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
                .padding(Edges::symmetric(0.logical_px(), WINDOW_PAD_H.logical_px()))
                .align(Align::Center)
                .child(
                    Text::new(
                        state
                            .scroller
                            .visible_text_limited(state.layout().max_window_chars),
                    )
                        .font(font_window_scroll())
                        .font_size(12.logical_px())
                        .line_height(1.15)
                        .color(SUBTEXT1),
                ),
        )
        .into()
}
