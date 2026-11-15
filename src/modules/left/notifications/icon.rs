//! Notification icon widget (image or fallback glyph).

use waywi::prelude::*;

use crate::app::state::Msg;
use crate::theme::{OVERLAY0, RED, font_ui};

pub fn build_notif_icon(asset: Option<ImageAsset>, dnd: bool, size: f32) -> Element<Msg> {
    if let Some(img) = asset {
        Image::new(img)
            .size(Size::px(size.logical_px(), size.logical_px()))
            .fit(ImageFit::Cover)
            .into()
    } else {
        let glyph_px = (size * 0.65).max(12.0);
        Group::new()
            .size(Size::px(size.logical_px(), size.logical_px()))
            .child(
                Text::new(if dnd { "\u{f009b}" } else { "\u{f009a}" })
                    .font(font_ui())
                    .font_size(glyph_px.logical_px())
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
