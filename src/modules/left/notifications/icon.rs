//! Notification icon widget (image or fallback glyph, no grey circle background).

use waywi::prelude::*;

use crate::app::state::Msg;
use crate::theme::{NOTIF_ICON_SIZE, OVERLAY0, RED, font_ui};

pub fn build_circle_icon(asset: Option<ImageAsset>, dnd: bool) -> Element<Msg> {
    if let Some(img) = asset {
        let radius = (NOTIF_ICON_SIZE * 0.5).logical_px();
        Clip::new(
            Image::new(img)
                .size(Size::px(
                    NOTIF_ICON_SIZE.logical_px(),
                    NOTIF_ICON_SIZE.logical_px(),
                ))
                .fit(ImageFit::Cover),
        )
        .corner_radius(radius)
        .size(Size::px(
            NOTIF_ICON_SIZE.logical_px(),
            NOTIF_ICON_SIZE.logical_px(),
        ))
        .into()
    } else {
        Group::new()
            .size(Size::px(
                NOTIF_ICON_SIZE.logical_px(),
                NOTIF_ICON_SIZE.logical_px(),
            ))
            .child(
                Text::new(if dnd { "\u{f009b}" } else { "\u{f009a}" })
                    .font(font_ui())
                    .font_size(12.logical_px())
                    .color(if dnd { RED } else { OVERLAY0 })
                    .position(Position::relative_to_parent(
                        Percent::new(50.0),
                        Percent::new(50.0),
                    ))
                    .anchor(ElementAnchor::Center),
            )
            .into()
    }
}
