//! Notification icon element (image or fallback glyph), shared by the bar pill,
//! the incoming ticker, and the notification center cards.

use waywi::prelude::*;

use crate::app::state::Msg;
use crate::theme::{OVERLAY0, RED, font_ui};

pub fn build_notif_icon(asset: Option<ImageAsset>, dnd: bool, size: f32) -> Element<Msg> {
    if let Some(img) = asset {
        // `ImageFit::Cover` scales non-square icons beyond `size` on the longer axis; clip to
        // the exact square slot so wide/tall notification thumbnails never overflow siblings.
        Clip::new(
            Image::new(img)
                .size(Size::fill())
                .fit(ImageFit::Cover),
        )
        .size(Size::new(size.logical_px(), size.logical_px()))
        .into()
    } else {
        let glyph_px = (size * 0.65).max(12.0);
        Group::new()
            .size(Size::new(size.logical_px(), size.logical_px()))
            .child(
                Text::new(if dnd { "\u{f009b}" } else { "\u{f009a}" })
                    .font(font_ui())
                    .font_size(glyph_px.logical_px())
                    .bounds_mode(TextBoundsMode::Optical)
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
