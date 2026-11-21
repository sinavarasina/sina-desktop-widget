//! Minimalist MPRIS control box (album art, track text, and transport icon buttons).

use waywi::prelude::*;

use crate::app::mpris::MprisControlAction;
use crate::app::state::{HoverTarget, Msg, State};
use crate::theme::{
    CRUST, MANTLE, MAUVE, MPRIS_POPUP_WIDTH, SUBTEXT1, SURFACE0, SURFACE1, TEXT, font_ui,
    font_ui_bold,
};
use crate::widgets::text::{format_artist_album, marquee_slice};

const ART_POPUP_SIZE: f32 = 52.0;
const META_TEXT_WIDTH: f32 = 232.0;

pub fn build_control_box(state: &State) -> Element<Msg> {
    let player = state.mpris.active_player();
    let title_raw = player
        .and_then(|p| p.title.as_deref())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("No active track");
    let sub_raw = player.map_or_else(|| String::from("Idle"), format_artist_album);

    let title_text = marquee_slice(title_raw, 25, state.mpris.marquee_tick);
    let sub_text = marquee_slice(&sub_raw, 26, state.mpris.marquee_tick);

    let art_elem: Element<Msg> = if let Some(img) = state.mpris.art_asset {
        // ImageFit::Cover resolves to a rect larger than the box for non-1:1 art (centered
        // overflow, unclipped); Clip keeps the spill inside the square frame.
        Clip::new(
            Image::new(img)
                .size(Size::px(
                    ART_POPUP_SIZE.logical_px(),
                    ART_POPUP_SIZE.logical_px(),
                ))
                .fit(ImageFit::Cover),
        )
        .size(Size::px(
            ART_POPUP_SIZE.logical_px(),
            ART_POPUP_SIZE.logical_px(),
        ))
        .into()
    } else {
        Group::new()
            .size(Size::px(
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
                    .line_height(1.0)
                    .color(MAUVE)
                    .position(Position::RelativeToParent {
                        x: Percent::new(50.0),
                        y: Percent::new(50.0),
                        offset_x: (-2.0).logical_px(),
                        offset_y: 0.logical_px(),
                    })
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

    let right_col = Column::new()
        .size(Size::new(META_TEXT_WIDTH.logical_px(), Length::Auto))
        .gap(4.logical_px())
        .child(
            Clip::new(
                Text::new(title_text)
                    .font(font_ui_bold())
                    .font_size(11.5.logical_px())
                    .color(TEXT),
            )
            .size(Size::new(META_TEXT_WIDTH.logical_px(), 16.logical_px())),
        )
        .child(
            Clip::new(
                Text::new(sub_text)
                    .font(font_ui())
                    .font_size(10.5.logical_px())
                    .color(SUBTEXT1),
            )
            .size(Size::new(META_TEXT_WIDTH.logical_px(), 14.logical_px())),
        )
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
        .size(Size::px(18.logical_px(), 18.logical_px()))
        .on_hover(Msg::Hover(target))
        .on_leave(Msg::Leave(target))
        .on_click(Msg::MprisAction(action))
        .child(
            Text::new(glyph)
                .font(font_ui())
                .font_size(13.logical_px())
                .color(fg)
                .position(Position::relative_to_parent(
                    Percent::new(50.0),
                    Percent::new(50.0),
                ))
                .anchor(ElementAnchor::Center),
        )
        .into()
}
