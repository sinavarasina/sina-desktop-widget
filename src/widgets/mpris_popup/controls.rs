//! Minimalist MPRIS control box (album art, track text, and transport icon buttons).

use waywi::prelude::*;

use crate::app::mpris::{MPRIS_MARQUEE_GAP_PX, MPRIS_POPUP_TEXT_INNER_W, MprisControlAction};
use crate::app::state::{HoverTarget, Msg, State};
use crate::theme::{
    CRUST, MANTLE, MAUVE, MPRIS_POPUP_WIDTH, OVERLAY0, SUBTEXT1, SURFACE0, SURFACE1, TEXT,
    font_ui, font_ui_bold,
};

const ART_POPUP_SIZE: f32 = 52.0;
const META_TEXT_WIDTH: f32 = MPRIS_POPUP_TEXT_INNER_W;

pub fn build_control_box(state: &State) -> Element<Msg> {
    let player = state.mpris.active_player();
    let title_str = if state.mpris.bar_bot_text.is_empty() {
        "No active track"
    } else {
        state.mpris.bar_bot_text.as_str()
    };
    let sub_str = if state.mpris.bar_top_text.is_empty() {
        "Idle"
    } else {
        state.mpris.bar_top_text.as_str()
    };

    let art_elem: Element<Msg> = if let Some(img) = state.mpris.art_asset {
        // ImageFit::Cover resolves to a rect larger than the box for non-1:1 art (centered
        // overflow, unclipped); Clip keeps the spill inside the square frame.
        Clip::new(
            Image::new(img)
                .size(Size::new(
                    ART_POPUP_SIZE.logical_px(),
                    ART_POPUP_SIZE.logical_px(),
                ))
                .fit(ImageFit::Cover),
        )
        .size(Size::new(
            ART_POPUP_SIZE.logical_px(),
            ART_POPUP_SIZE.logical_px(),
        ))
        .into()
    } else {
        Group::new()
            .size(Size::new(
                ART_POPUP_SIZE.logical_px(),
                ART_POPUP_SIZE.logical_px(),
            ))
            .child(
                Rect::new()
                    .size(Size::fill())
                    .fill(MANTLE)
                    .border(1.logical_px(), SURFACE0),
            )
            .child(
                Text::new("\u{f001}")
                    .font(font_ui())
                    .font_size(20.logical_px())
                    .bounds_mode(TextBoundsMode::Optical)
                    .color(MAUVE)
                    .position(Position::relative_to_parent(
                        Percent::new(50.0),
                        Percent::new(50.0),
                    ))
                    .anchor(ElementAnchor::Center),
            )
            .into()
    };

    let is_playing = player.is_some_and(|p| p.status == MprisPlaybackStatus::Playing);
    let shuffle_on = player.is_some_and(|p| p.shuffle);
    let loop_status = player.map_or(MprisLoopStatus::None, |p| p.loop_status);
    let loop_icon = match loop_status {
        MprisLoopStatus::Track => "\u{f0458}",
        _ => "\u{f0456}",
    };

    let controls_row = Row::new()
        .size(Size::new(Length::Auto, 22.logical_px()))
        .gap(12.logical_px())
        .align(Align::Center)
        .child(icon_btn(
            "mpris-btn-shuffle",
            "\u{f049d}",
            HoverTarget::MprisShuffle,
            state.hovered == HoverTarget::MprisShuffle,
            shuffle_on,
            MprisControlAction::ToggleShuffle,
        ))
        .child(icon_btn(
            "mpris-btn-prev",
            "\u{f04ae}",
            HoverTarget::MprisPrev,
            state.hovered == HoverTarget::MprisPrev,
            false,
            MprisControlAction::Previous,
        ))
        .child(icon_btn(
            "mpris-btn-play",
            if is_playing { "\u{f03e4}" } else { "\u{f040a}" },
            HoverTarget::MprisPlayPause,
            state.hovered == HoverTarget::MprisPlayPause,
            is_playing,
            MprisControlAction::PlayPause,
        ))
        .child(icon_btn(
            "mpris-btn-stop",
            "\u{f04db}",
            HoverTarget::MprisStop,
            state.hovered == HoverTarget::MprisStop,
            false,
            MprisControlAction::Stop,
        ))
        .child(icon_btn(
            "mpris-btn-next",
            "\u{f04ad}",
            HoverTarget::MprisNext,
            state.hovered == HoverTarget::MprisNext,
            false,
            MprisControlAction::Next,
        ))
        .child(icon_btn(
            "mpris-btn-loop",
            loop_icon,
            HoverTarget::MprisLoop,
            state.hovered == HoverTarget::MprisLoop,
            loop_status != MprisLoopStatus::None,
            MprisControlAction::CycleLoop,
        ));

    let title_line = build_popup_marquee_line(
        "mpris-popup-title",
        title_str,
        state.mpris.popup_title_width_px,
        state.mpris.popup_title_scroll_px,
        state.mpris.popup_title_overflows(),
        font_ui_bold(),
        11.5,
        16.0,
        TEXT,
    );
    let sub_line = build_popup_marquee_line(
        "mpris-popup-sub",
        sub_str,
        state.mpris.popup_sub_width_px,
        state.mpris.popup_sub_scroll_px,
        state.mpris.popup_sub_overflows(),
        font_ui(),
        10.5,
        14.0,
        SUBTEXT1,
    );

    let right_col = Column::new()
        .size(Size::new(META_TEXT_WIDTH.logical_px(), Length::Auto))
        .gap(4.logical_px())
        .child(title_line)
        .child(sub_line)
        .child(controls_row);

    Group::new()
        .key("mpris-control-box")
        .size(Size::new(MPRIS_POPUP_WIDTH.logical_px(), Length::Auto))
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(CRUST)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), SURFACE1),
        )
        .child(
            Row::new()
                .size(Size::new(Length::Fill, Length::Auto))
                .padding(Edges::symmetric(10.logical_px(), 12.logical_px()))
                .gap(12.logical_px())
                .align(Align::Center)
                .child(art_elem)
                .child(right_col),
        )
        .into()
}

#[allow(clippy::too_many_arguments)]
fn build_popup_marquee_line(
    key: &'static str,
    text: &str,
    measured_w: f32,
    scroll_px: f32,
    overflows: bool,
    font: Font,
    font_size: f32,
    line_h: f32,
    color: Color,
) -> Element<Msg> {
    // Keep `Text` content constant across frames so `TextEngine::buffer_cache` and GPU
    // `resolve_text_quad` achieve 100% cache hits (eliminating CJK fallback reshapes & stutter).
    let make_text = |s: &str, x: f32| {
        Text::new(s.to_owned())
            .font(font.clone())
            .font_size(font_size.logical_px())
            .color(color)
            .position(Position::absolute(x.logical_px(), 0.logical_px()))
    };

    let inner: Element<Msg> = if !overflows {
        Group::new()
            .size(Size::new(META_TEXT_WIDTH.logical_px(), line_h.logical_px()))
            .child(make_text(text, 0.0))
            .into()
    } else {
        let cycle = measured_w + MPRIS_MARQUEE_GAP_PX;
        let x1 = -scroll_px;
        let dot_x = measured_w + MPRIS_MARQUEE_GAP_PX * 0.5 - scroll_px;
        let x2 = cycle - scroll_px;
        Group::new()
            .size(Size::new(META_TEXT_WIDTH.logical_px(), line_h.logical_px()))
            .child(make_text(text, x1))
            .child(
                Text::new("\u{2022}")
                    .font(font_ui())
                    .font_size((font_size - 1.5).max(8.0).logical_px())
                    .color(OVERLAY0)
                    .position(Position::absolute(
                        dot_x.logical_px(),
                        (line_h * 0.5).logical_px(),
                    ))
                    .anchor(ElementAnchor::Center),
            )
            .child(make_text(text, x2))
            .into()
    };

    Clip::new(inner)
        .key(key)
        .size(Size::new(META_TEXT_WIDTH.logical_px(), line_h.logical_px()))
        .into()
}

fn icon_btn(
    key: &'static str,
    glyph: &'static str,
    target: HoverTarget,
    hovered: bool,
    active: bool,
    action: MprisControlAction,
) -> Element<Msg> {
    let fg = if hovered {
        TEXT
    } else if active {
        MAUVE
    } else {
        SUBTEXT1
    };
    Group::new()
        .key(key)
        .size(Size::new(18.logical_px(), 18.logical_px()))
        .cursor(CursorShape::Pointer)
        .on_hover(Msg::Hover(target))
        .on_leave(Msg::Leave(target))
        .on_click(Msg::MprisAction(action))
        .child(
            Text::new(glyph)
                .font(font_ui())
                .font_size(13.logical_px())
                .bounds_mode(TextBoundsMode::Optical)
                .color(fg)
                .position(Position::relative_to_parent(
                    Percent::new(50.0),
                    Percent::new(50.0),
                ))
                .anchor(ElementAnchor::Center),
        )
        .into()
}
