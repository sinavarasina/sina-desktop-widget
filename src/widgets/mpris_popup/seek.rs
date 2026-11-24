//! Interactive MPRIS seek bar box (`mm:ss` + progress track + `mm:ss`).

use waywi::prelude::*;

use crate::app::state::{HoverTarget, Msg, State};
use crate::theme::{
    CRUST, MAUVE, MPRIS_POPUP_WIDTH, MPRIS_SEEK_TRACK_WIDTH, OVERLAY0, SUBTEXT1, SURFACE0,
    SURFACE1, font_ui_semibold,
};

pub fn build_seek_box(state: &State) -> Element<Msg> {
    let player = state.mpris.active_player();
    let pos_us = player.and_then(|p| p.position_us).unwrap_or(0).max(0);
    let len_us = player.and_then(|p| p.length_us).unwrap_or(0).max(0);
    let ratio = if len_us > 0 {
        ((pos_us as f64) / (len_us as f64)).clamp(0.0, 1.0) as f32
    } else {
        0.0
    };
    let fill_w = (MPRIS_SEEK_TRACK_WIDTH * ratio).max(0.0);

    let track = Group::new()
        .key("mpris-seek-track")
        .size(Size::new(
            MPRIS_SEEK_TRACK_WIDTH.logical_px(),
            6.logical_px(),
        ))
        .on_hover(Msg::Hover(HoverTarget::MprisSeek))
        .on_leave(Msg::Leave(HoverTarget::MprisSeek))
        .on_pointer_down(Msg::MprisSeekClick)
        .on_scroll(Msg::MprisBarScroll)
        .child(Rect::new().size(Size::fill()).fill(SURFACE0))
        .child(
            Rect::new()
                .size(Size::new(fill_w.logical_px(), 6.logical_px()))
                .fill(MAUVE),
        );

    Group::new()
        .key("mpris-seek-box")
        .size(Size::new(MPRIS_POPUP_WIDTH.logical_px(), 26.logical_px()))
        .on_scroll(Msg::MprisBarScroll)
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(CRUST)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), SURFACE1),
        )
        .child(
            Row::new()
                .size(Size::fill())
                .padding(Edges::symmetric(0.logical_px(), 10.logical_px()))
                .gap(8.logical_px())
                .align(Align::Center)
                .child(
                    Text::new(format_mm_ss(pos_us))
                        .font(font_ui_semibold())
                        .font_size(10.logical_px())
                        .color(SUBTEXT1),
                )
                .child(track)
                .child(
                    Text::new(format_mm_ss(len_us))
                        .font(font_ui_semibold())
                        .font_size(10.logical_px())
                        .color(OVERLAY0),
                ),
        )
        .into()
}

fn format_mm_ss(us: i64) -> String {
    let total_secs = (us.max(0) / 1_000_000) as u64;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{mins:02}:{secs:02}")
}
