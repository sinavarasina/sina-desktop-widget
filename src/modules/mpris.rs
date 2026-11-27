//! Bar MPRIS module: spinning circular vinyl album art badge and pixel-exact scrolling track pill.

use waywi::prelude::*;

use crate::app::mpris::MPRIS_MARQUEE_GAP_PX;
use crate::app::state::{HoverTarget, Msg, State};
use crate::theme::{
    BAR_HEIGHT, CRUST, MAUVE, MPRIS_ART_SIZE, OVERLAY0, SUBTEXT1, SURFACE0, SURFACE1, TEXT,
    font_ui, font_ui_semibold,
};
use crate::widgets::separators::{SepShape, separator};

const BAR_LINE_H: f32 = 11.0;

pub fn build_mpris(state: &State) -> Element<Msg> {
    let hovered = state.hovered == HoverTarget::MprisBar;
    let badge_bg = if hovered { SURFACE0 } else { CRUST };
    let playing = state.mpris.is_playing();

    let album_inner: Element<Msg> = if let Some(img) = state.mpris.art_asset {
        let radius = MPRIS_ART_SIZE * 0.5;
        let rim_col = if playing {
            MAUVE.with_opacity(0.65)
        } else {
            SURFACE1
        };

        let spinning_img = Image::new(img)
            .size(Size::new(
                MPRIS_ART_SIZE.logical_px(),
                MPRIS_ART_SIZE.logical_px(),
            ))
            .fit(ImageFit::Cover)
            .transform(
                Transform::identity()
                    .pivot(0.5, 0.5)
                    .rotate_rad(state.mpris.disc_angle_rad),
            );

        let circular_art = Clip::new(spinning_img)
            .size(Size::new(
                MPRIS_ART_SIZE.logical_px(),
                MPRIS_ART_SIZE.logical_px(),
            ))
            .corner_radius(CornerRadius::all(radius.logical_px()));

        // Subtle circular vinyl rim + center spindle dot.
        let rim = Rect::new()
            .size(Size::fill())
            .fill(Color::TRANSPARENT)
            .corner_radius(CornerRadius::all(radius.logical_px()))
            .border(1.logical_px(), rim_col);

        let spindle = Rect::new()
            .size(Size::new(4.logical_px(), 4.logical_px()))
            .fill(CRUST)
            .corner_radius(CornerRadius::all(2.logical_px()))
            .border(0.8.logical_px(), SURFACE1)
            .position(Position::relative_to_parent(
                Percent::new(50.0),
                Percent::new(50.0),
            ))
            .anchor(ElementAnchor::Center);

        Group::new()
            .key("mpris-vinyl-disc")
            .position(Position::relative_to_parent(
                Percent::new(50.0),
                Percent::new(50.0),
            ))
            .size(Size::new(
                MPRIS_ART_SIZE.logical_px(),
                MPRIS_ART_SIZE.logical_px(),
            ))
            .anchor(ElementAnchor::Center)
            .child(circular_art)
            .child(rim)
            .child(spindle)
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

    let layout = state.layout();
    if state.mpris.has_active_session() && layout.show_mpris_text {
        let pill_bg = if hovered { SURFACE0 } else { CRUST };
        let text_w = layout.mpris_bar_text_width;
        let inner_w = layout.mpris_bar_text_inner_w();

        let top_line = build_marquee_line(
            "mpris-bar-top",
            &state.mpris.bar_top_text,
            inner_w,
            state.mpris.bar_top_width_px,
            state.mpris.bar_top_scroll_px,
            state.mpris.bar_top_width_px > inner_w + 0.5,
            font_ui(),
            9.5,
            MAUVE,
        );
        let bot_line = build_marquee_line(
            "mpris-bar-bot",
            &state.mpris.bar_bot_text,
            inner_w,
            state.mpris.bar_bot_width_px,
            state.mpris.bar_bot_scroll_px,
            state.mpris.bar_bot_width_px > inner_w + 0.5,
            font_ui_semibold(),
            10.0,
            if playing { TEXT } else { SUBTEXT1 },
        );

        let text_col = Column::new()
            .size(Size::new(text_w.logical_px(), BAR_HEIGHT.logical_px()))
            .padding(Edges::symmetric(1.logical_px(), 4.logical_px()))
            .child(top_line)
            .child(bot_line);

        let meta_pill = Group::new()
            .key("mpris-meta-pill")
            .size(Size::new(text_w.logical_px(), BAR_HEIGHT.logical_px()))
            .child(Rect::new().size(Size::fill()).fill(pill_bg))
            .child(text_col);

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
                SepShape::ArrowR,
                pill_bg,
                Color::TRANSPARENT,
            ));
    }

    row.into()
}

#[allow(clippy::too_many_arguments)]
fn build_marquee_line(
    key: &'static str,
    text: &str,
    inner_w: f32,
    measured_w: f32,
    scroll_px: f32,
    overflows: bool,
    font: Font,
    font_size: f32,
    color: Color,
) -> Element<Msg> {
    let make_text = |s: &str, x: f32| {
        Text::new(s.to_owned())
            .font(font.clone())
            .font_size(font_size.logical_px())
            .line_height(1.05)
            .color(color)
            .position(Position::absolute(x.logical_px(), 0.logical_px()))
    };

    let inner: Element<Msg> = if !overflows {
        Group::new()
            .size(Size::new(inner_w.logical_px(), BAR_LINE_H.logical_px()))
            .child(make_text(text, 0.0))
            .into()
    } else {
        let cycle = measured_w + MPRIS_MARQUEE_GAP_PX;
        let x1 = -scroll_px;
        let dot_x = measured_w + MPRIS_MARQUEE_GAP_PX * 0.5 - scroll_px;
        let x2 = cycle - scroll_px;
        Group::new()
            .size(Size::new(inner_w.logical_px(), BAR_LINE_H.logical_px()))
            .child(make_text(text, x1))
            .child(
                Text::new("\u{2022}")
                    .font(font_ui())
                    .font_size((font_size - 1.0).max(7.5).logical_px())
                    .color(OVERLAY0)
                    .position(Position::absolute(
                        dot_x.logical_px(),
                        (BAR_LINE_H * 0.5).logical_px(),
                    ))
                    .anchor(ElementAnchor::Center),
            )
            .child(make_text(text, x2))
            .into()
    };

    Clip::new(inner)
        .key(key)
        .size(Size::new(inner_w.logical_px(), BAR_LINE_H.logical_px()))
        .into()
}
