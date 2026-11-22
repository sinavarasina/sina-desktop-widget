//! Two-line system metadata pill attached to the right of the Gentoo logo (`Text A` / divider / `Text B`).

use waywi::prelude::*;

use crate::app::state::{HoverTarget, Msg, State};
use crate::theme::{
    BAR_HEIGHT, BLUE, CRUST, GREEN, MANTLE, MAUVE, RED, SUBTEXT0, SUBTEXT1, SURFACE0, SURFACE1,
    font_ui, font_ui_semibold,
};
use crate::widgets::reel::{ReelAlign, ReelDirection, build_reel_slot, build_reel_slot_accented};
use crate::widgets::separators::{SepShape, separator};

const ROW_HEIGHT: f32 = 11.0;
const CHAR_WIDTH_PX: f32 = 5.0;
const MAX_CHARS: usize = 14;
const INNER_PAD_H: f32 = 2.0;
const TEXT_A_INITIAL_ACCENTS: [Color; 4] = [MAUVE, BLUE, GREEN, RED];

pub fn build_sysinfo(state: &State) -> Element<Msg> {
    let bg = if state.hovered == HoverTarget::SysInfo {
        SURFACE0
    } else {
        MANTLE
    };

    let items_a = state.sysinfo.text_a_items();
    let items_b = state
        .sysinfo
        .text_b_items(state.cpu.as_ref(), state.memory.as_ref());

    let max_chars = items_a
        .iter()
        .chain(items_b.iter())
        .map(|s| s.chars().count().min(MAX_CHARS))
        .max()
        .unwrap_or(6);
    let text_w = ((max_chars as f32) * CHAR_WIDTH_PX + 2.0).clamp(36.0, 74.0);
    let body_w = text_w + INNER_PAD_H * 2.0;
    let divider_w = (text_w - 10.0).max(16.0);

    let line_a = build_reel_slot_accented(
        "sysinfo-text-a",
        text_w,
        ROW_HEIGHT,
        &items_a,
        &state.sysinfo.reel_a,
        font_ui_semibold(),
        8.0,
        SUBTEXT1,
        Some(&TEXT_A_INITIAL_ACCENTS),
        MAX_CHARS,
        ReelAlign::Center,
        ReelDirection::SlideUp,
    );
    let divider = Rect::new()
        .size(Size::px(divider_w.logical_px(), 1.logical_px()))
        .fill(SURFACE1);
    let line_b = build_reel_slot(
        "sysinfo-text-b",
        text_w,
        ROW_HEIGHT,
        &items_b,
        &state.sysinfo.reel_b,
        font_ui(),
        8.0,
        SUBTEXT0,
        MAX_CHARS,
        ReelAlign::Center,
        ReelDirection::SlideUp,
    );

    let body = Group::new()
        .key("mod-sysinfo-body")
        .size(Size::px(body_w.logical_px(), BAR_HEIGHT.logical_px()))
        .child(Rect::new().size(Size::fill()).fill(bg))
        .child(
            Column::new()
                .size(Size::auto())
                .align(Align::Center)
                .gap(0.logical_px())
                .position(Position::relative_to_parent(
                    Percent::new(50.0),
                    Percent::new(50.0),
                ))
                .anchor(ElementAnchor::Center)
                .child(line_a)
                .child(divider)
                .child(line_b),
        );

    Row::new()
        .key("mod-sysinfo")
        .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
        .align(Align::Center)
        .on_hover(Msg::Hover(HoverTarget::SysInfo))
        .on_leave(Msg::Leave(HoverTarget::SysInfo))
        .on_pointer_move(|ev| {
            let left = (ev.position.x.get() - ev.local_position.x.get()).max(8.0);
            Msg::HoverAnchorX(left)
        })
        .child(separator("sep-l-1", SepShape::ArrowR, CRUST, bg))
        .child(body)
        .child(separator(
            "sep-l-sys-r",
            SepShape::ArrowR,
            bg,
            Color::TRANSPARENT,
        ))
        .into()
}
