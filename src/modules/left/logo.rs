//! Gentoo launcher logo button on the far left of the bar.

use waywi::prelude::*;

use crate::app::state::{HoverTarget, Msg, State};
use crate::theme::{
    BAR_HEIGHT, CRUST, LOGO_MIN_WIDTH, LOGO_PAD_LEFT, LOGO_PAD_RIGHT, MAUVE, font_ui,
};

pub fn build_logo(state: &State) -> Element<Msg> {
    let img: Element<Msg> = if let Some(asset) = state.logo_asset {
        Image::new(asset)
            .size(Size::new(LOGO_MIN_WIDTH.logical_px(), 18.logical_px()))
            .fit(ImageFit::Contain)
            .into()
    } else {
        Text::new("\u{f30d}")
            .font(font_ui())
            .font_size(14.logical_px())
            .bounds_mode(TextBoundsMode::Optical)
            .color(MAUVE)
            .into()
    };

    Group::new()
        .key("custom-logo")
        .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
        .on_hover(Msg::Hover(HoverTarget::Logo))
        .on_leave(Msg::Leave(HoverTarget::Logo))
        .on_pointer_down(|ev| {
            if ev.button == Some(MouseButton::Left) {
                Msg::ToggleDrun
            } else {
                Msg::Hover(HoverTarget::Logo)
            }
        })
        .child(Rect::new().size(Size::fill()).fill(CRUST))
        .child(
            Row::new()
                .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
                .padding(Edges::new(
                    0.logical_px(),
                    LOGO_PAD_RIGHT.logical_px(),
                    0.logical_px(),
                    LOGO_PAD_LEFT.logical_px(),
                ))
                .align(Align::Center)
                .child(img),
        )
        .into()
}
