//! Attached slanted window category and rotating metadata module beside the window title scroller.

use waywi::prelude::*;

use crate::app::state::{HoverTarget, Msg, State};
use crate::theme::{
    BAR_HEIGHT, CRUST, MANTLE, MAUVE, OVERLAY0, SUBTEXT1, SURFACE0, font_ui, font_ui_semibold,
};
use crate::widgets::reel::{ReelAlign, ReelDirection, build_reel_slot};
use crate::widgets::separators::{SepShape, separator};

const ROW_HEIGHT: f32 = 11.0;
const CHAR_WIDTH_PX: f32 = 5.1;
const MAX_CHARS: usize = 12;
/// Top row indent matching the `/` left slant (`11x24` diagonal shifts ~5.5px between row centers).
const TOP_SLANT_INDENT: f32 = 5.0;
const RIGHT_PAD: f32 = 2.0;

pub fn build_window_info(state: &State) -> Element<Msg> {
    let bg = if state.hovered == HoverTarget::WindowInfo {
        SURFACE0
    } else {
        MANTLE
    };

    let info_items = state.window_info.info_items(state.booster.short_badge());
    let cat_chars = state.window_info.category.chars().count().min(MAX_CHARS);

    let cat_w = ((cat_chars as f32) * CHAR_WIDTH_PX + 2.0).max(20.0);
    let info_w = animated_info_width(&info_items, &state.window_info.reel);
    let body_w = (TOP_SLANT_INDENT + cat_w).max(info_w).max(32.0) + RIGHT_PAD;

    let cat_color = if state.window_info.client.is_some() {
        MAUVE
    } else {
        OVERLAY0
    };
    let cat_line = Clip::new(
        Group::new()
            .size(Size::new(cat_w.logical_px(), ROW_HEIGHT.logical_px()))
            .child(
                Text::new(state.window_info.category.clone())
                    .font(font_ui_semibold())
                    .font_size(8.0.logical_px())
                    .line_height(1.0)
                    .color(cat_color)
                    .position(Position::relative_to_parent(
                        Percent::new(0.0),
                        Percent::new(50.0),
                    ))
                    .anchor(ElementAnchor::CenterLeft),
            ),
    )
    .size(Size::new(cat_w.logical_px(), ROW_HEIGHT.logical_px()))
    .position(Position::absolute(
        TOP_SLANT_INDENT.logical_px(),
        1.logical_px(),
    ));

    let info_line = Group::new()
        .size(Size::new(info_w.logical_px(), ROW_HEIGHT.logical_px()))
        .position(Position::absolute(0.logical_px(), 12.logical_px()))
        .child(build_reel_slot(
            "win-info-reel",
            info_w,
            ROW_HEIGHT,
            &info_items,
            &state.window_info.reel,
            font_ui(),
            8.0,
            SUBTEXT1,
            MAX_CHARS,
            ReelAlign::Left,
            ReelDirection::SlideLeft,
        ));

    let body = Group::new()
        .key("mod-window-info-body")
        .size(Size::new(body_w.logical_px(), BAR_HEIGHT.logical_px()))
        .child(Rect::new().size(Size::fill()).fill(bg))
        .child(cat_line)
        .child(info_line);

    Row::new()
        .key("mod-window-info")
        .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
        .align(Align::Center)
        .on_hover(Msg::Hover(HoverTarget::WindowInfo))
        .on_leave(Msg::Leave(HoverTarget::WindowInfo))
        .on_pointer_move(|ev| {
            let left = (ev.position.x.get() - ev.local_position.x.get()).max(8.0);
            Msg::HoverAnchorX(left)
        })
        .child(separator("sep-l-win-meta", SepShape::SlashR, CRUST, bg))
        .child(body)
        .child(separator(
            "sep-l-7",
            SepShape::SlashR,
            bg,
            Color::TRANSPARENT,
        ))
        .into()
}

fn animated_info_width(items: &[String], reel: &crate::app::reel::ReelState) -> f32 {
    if items.is_empty() {
        return 20.0;
    }
    let idx = reel.index % items.len();
    let cur_w =
        ((items[idx].chars().count().min(MAX_CHARS) as f32) * CHAR_WIDTH_PX + 2.0).max(20.0);
    if reel.animating && items.len() > 1 {
        let next_idx = (idx + 1) % items.len();
        let next_w = ((items[next_idx].chars().count().min(MAX_CHARS) as f32) * CHAR_WIDTH_PX
            + 2.0)
            .max(20.0);
        let p = reel.progress.clamp(0.0, 1.0);
        let ease = 1.0 - (1.0 - p).powi(3);
        cur_w + (next_w - cur_w) * ease
    } else {
        cur_w
    }
}
