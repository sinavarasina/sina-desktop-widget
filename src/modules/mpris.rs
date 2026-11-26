//! Bar MPRIS module: album art chevron badge and adaptive track info pill.

use waywi::prelude::*;

use crate::app::state::{HoverTarget, Msg, State};
use crate::theme::{
    BAR_HEIGHT, CRUST, MAUVE, MPRIS_ART_SIZE, MPRIS_BAR_TEXT_WIDTH, OVERLAY0, SUBTEXT1, SURFACE0,
    TEXT, font_ui, font_ui_semibold,
};
use crate::widgets::separators::{SepShape, separator};
use crate::widgets::text::{format_artist_album, marquee_slice};

const MAX_BAR_META_COLS: usize = 22;

pub fn build_mpris(state: &State) -> Element<Msg> {
    let hovered = state.hovered == HoverTarget::MprisBar;
    let badge_bg = if hovered { SURFACE0 } else { CRUST };

    let album_inner: Element<Msg> = if let Some(img) = state.mpris.art_asset {
        // Clip for the same reason as the popup art: non-1:1 Cover art overflows its box.
        Clip::new(
            Image::new(img)
                .size(Size::new(
                    MPRIS_ART_SIZE.logical_px(),
                    MPRIS_ART_SIZE.logical_px(),
                ))
                .fit(ImageFit::Cover),
        )
        .position(Position::RelativeToParent {
            x: Percent::new(50.0),
            y: Percent::new(50.0),
            offset_x: 0.logical_px(),
            offset_y: 0.logical_px(),
        })
        .size(Size::new(
            MPRIS_ART_SIZE.logical_px(),
            MPRIS_ART_SIZE.logical_px(),
        ))
        .anchor(ElementAnchor::Center)
        .into()
    } else {
        let icon_col = if state.mpris.has_active_session() {
            MAUVE
        } else {
            OVERLAY0
        };
        Text::new("\u{f001}")
            .font(font_ui())
            .font_size(11.logical_px())
            .bounds_mode(TextBoundsMode::Optical)
            .color(icon_col)
            .position(Position::relative_to_parent(
                Percent::new(50.0),
                Percent::new(50.0),
            ))
            .anchor(ElementAnchor::Center)
            .into()
    };

    let album_box = Group::new()
        .key("mpris-art-box")
        .size(Size::new(24.logical_px(), BAR_HEIGHT.logical_px()))
        .child(Rect::new().size(Size::fill()).fill(badge_bg))
        .child(album_inner);

    let mut row = Row::new()
        .key("mod-mpris")
        .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
        .align(Align::Center)
        .cursor(CursorShape::Pointer)
        .on_hover(Msg::Hover(HoverTarget::MprisBar))
        .on_leave(Msg::Leave(HoverTarget::MprisBar))
        .on_pointer_down(Msg::MprisBarPointerDown)
        .on_scroll(Msg::MprisBarScroll)
        .child(separator(
            "sep-mpris-l",
            SepShape::ArrowL,
            badge_bg,
            Color::TRANSPARENT,
        ))
        .child(album_box)
        .child(separator(
            "sep-mpris-mid",
            SepShape::ArrowR,
            badge_bg,
            Color::TRANSPARENT,
        ));

    if state.mpris.has_active_session()
        && let Some(player) = state.mpris.active_player()
    {
        let pill_bg = if hovered { SURFACE0 } else { CRUST };
        let top_raw = format_artist_album(player);
        let bot_raw = player
            .title
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .or(player.identity.as_deref())
            .unwrap_or("Playing");

        let top_text = marquee_slice(&top_raw, MAX_BAR_META_COLS, state.mpris.marquee_tick);
        let bot_text = marquee_slice(bot_raw, MAX_BAR_META_COLS, state.mpris.marquee_tick);

        let text_col = Column::new()
            .size(Size::new(
                MPRIS_BAR_TEXT_WIDTH.logical_px(),
                BAR_HEIGHT.logical_px(),
            ))
            .padding(Edges::symmetric(1.logical_px(), 4.logical_px()))
            .child(
                Text::new(top_text)
                    .font(font_ui())
                    .font_size(9.5.logical_px())
                    .line_height(1.05)
                    .color(MAUVE),
            )
            .child(
                Text::new(bot_text)
                    .font(font_ui_semibold())
                    .font_size(10.0.logical_px())
                    .line_height(1.05)
                    .color(if player.status == MprisPlaybackStatus::Playing {
                        TEXT
                    } else {
                        SUBTEXT1
                    }),
            );

        let meta_pill = Group::new()
            .key("mpris-meta-pill")
            .size(Size::new(
                MPRIS_BAR_TEXT_WIDTH.logical_px(),
                BAR_HEIGHT.logical_px(),
            ))
            .child(Rect::new().size(Size::fill()).fill(pill_bg))
            .child(Clip::new(text_col).size(Size::fill()));

        row = row
            .child(separator(
                "sep-mpris-notch",
                SepShape::CurveL,
                pill_bg,
                Color::TRANSPARENT,
            ))
            .child(meta_pill)
            .child(separator(
                "sep-mpris-r",
                SepShape::SlashRInv,
                pill_bg,
                Color::TRANSPARENT,
            ));
    } else {
        // Minimalist transparent `>` frame when stopped or idle; `sep-r-1` provides the right `\` edge.
        row = row.child(separator(
            "sep-mpris-notch-idle",
            SepShape::ArrowRThin,
            SURFACE0,
            Color::TRANSPARENT,
        ));
    }

    row.into()
}
