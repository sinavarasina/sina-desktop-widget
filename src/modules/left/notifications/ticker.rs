//! Incoming notification right-to-left scrolling bar ticker (transparent background).

use waywi::prelude::*;

use crate::app::notifications::IncomingTicker;
use crate::app::state::Msg;
use crate::theme::{BAR_HEIGHT, NOTIF_TICKER_WIDTH, SURFACE0, TEXT, font_ui};
use crate::widgets::separators::{SepShape, separator};

use super::icon::build_circle_icon;

pub fn build_incoming_ticker(ticker: &IncomingTicker) -> Element<Msg> {
    let text_pos = Position::RelativeToParent {
        x: Percent::new(0.0),
        y: Percent::new(50.0),
        offset_x: ticker.offset_x.logical_px(),
        offset_y: 0.logical_px(),
    };

    let scrolling_clip = Clip::new(
        Group::new()
            .size(Size::px(
                NOTIF_TICKER_WIDTH.logical_px(),
                BAR_HEIGHT.logical_px(),
            ))
            .child(
                Text::new(ticker.text.clone())
                    .font(font_ui())
                    .font_size(11.logical_px())
                    .line_height(1.15)
                    .color(TEXT)
                    .position(text_pos)
                    .anchor(ElementAnchor::CenterLeft),
            ),
    )
    .size(Size::px(
        NOTIF_TICKER_WIDTH.logical_px(),
        BAR_HEIGHT.logical_px(),
    ));

    let body = Row::new()
        .key(("notif-ticker", ticker.id))
        .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
        .padding(Edges::symmetric(0.logical_px(), 6.logical_px()))
        .gap(6.logical_px())
        .align(Align::Center)
        .child(build_circle_icon(ticker.icon, false))
        .child(scrolling_clip);

    Row::new()
        .key("notif-ticker-wrap")
        .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
        .align(Align::Center)
        .child(body)
        .child(separator(
            "sep-ticker-r",
            SepShape::SlashRThin,
            SURFACE0,
            Color::TRANSPARENT,
        ))
        .into()
}
