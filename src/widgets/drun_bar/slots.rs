//! Slanted semi-transparent app result slots and hidden-count pills for `drun`.

use waywi::prelude::*;

use crate::app::state::{HoverTarget, Msg, State};
use crate::theme::{
    BAR_HEIGHT, CRUST, MAUVE, OVERLAY0, SEP_WIDTH, SUBTEXT1, SURFACE0, SURFACE1, TEXT, font_ui,
    font_ui_semibold,
};
use crate::widgets::separators::{SepShape, separator, slanted_join};
use crate::widgets::text::marquee_slice;

pub const COUNT_PILL_W: f32 = 28.0;
const DRUN_ICON_SIZE: f32 = 16.0;

#[must_use]
fn slot_bg(is_active: bool) -> Color {
    if is_active {
        SURFACE0.with_opacity(0.88)
    } else {
        CRUST.with_opacity(0.76)
    }
}

pub fn build_hidden_count_pill(
    key_prefix: &'static str,
    left_sep_key: &'static str,
    count: usize,
    left_shape: SepShape,
    right_sep: Option<(&'static str, SepShape)>,
) -> Row<Msg> {
    let fg = if count > 0 { TEXT } else { OVERLAY0 };
    let font = if count > 0 {
        font_ui_semibold()
    } else {
        font_ui()
    };

    let mut row = Row::new()
        .key(key_prefix)
        .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
        .align(Align::Center)
        .child(separator(
            left_sep_key,
            left_shape,
            CRUST,
            Color::TRANSPARENT,
        ))
        .child(
            Group::new()
                .key((key_prefix, 0usize))
                .size(Size::px(COUNT_PILL_W.logical_px(), BAR_HEIGHT.logical_px()))
                .child(Rect::new().size(Size::fill()).fill(CRUST))
                .child(
                    Text::new(count.to_string())
                        .font(font)
                        .font_size(11.logical_px())
                        .color(fg)
                        .position(Position::relative_to_parent(
                            Percent::new(50.0),
                            Percent::new(50.0),
                        ))
                        .anchor(ElementAnchor::Center),
                ),
        );

    if let Some((right_sep_key, right_shape)) = right_sep {
        row = row.child(separator(
            right_sep_key,
            right_shape,
            CRUST,
            Color::TRANSPARENT,
        ));
    }

    row
}

pub fn build_results_section(state: &State, total_w: f32) -> Element<Msg> {
    let visible = state.drun.visible_match_indices();
    let inner_total = (total_w - 2.0 * SEP_WIDTH).max(40.0);

    if visible.is_empty() {
        return build_empty_results(state, total_w, inner_total);
    }

    let count = visible.len();
    let joins_w = (count.saturating_sub(1) as f32) * SEP_WIDTH;
    let slots_budget = (inner_total - joins_w).max(count as f32 * 40.0);
    let slot_w = slots_budget / (count as f32);

    let first_match = state.drun.scroll_start;
    let first_bg =
        slot_bg(first_match == state.drun.selected || state.hovered == HoverTarget::DrunEntry(0));
    let last_slot_idx = count - 1;
    let last_bg = slot_bg(
        (state.drun.scroll_start + last_slot_idx) == state.drun.selected
            || state.hovered == HoverTarget::DrunEntry(last_slot_idx),
    );

    let mut row = Row::new()
        .key("drun-results-row")
        .size(Size::px(total_w.logical_px(), BAR_HEIGHT.logical_px()))
        .align(Align::Center)
        .child(separator(
            "sep-drun-res-l",
            SepShape::SlashLInv,
            first_bg,
            Color::TRANSPARENT,
        ));

    let mut prev_bg = first_bg;
    let mut prev_selected = first_match == state.drun.selected;

    for (slot_idx, &entry_idx) in visible.iter().enumerate() {
        let match_pos = state.drun.scroll_start + slot_idx;
        let is_selected = match_pos == state.drun.selected;
        let is_hovered = state.hovered == HoverTarget::DrunEntry(slot_idx);
        let bg = slot_bg(is_selected || is_hovered);

        if slot_idx > 0 {
            let stroke = if prev_selected || is_selected {
                MAUVE.with_opacity(0.85)
            } else {
                SURFACE1.with_opacity(0.75)
            };
            row = row.child(slanted_join(("drun-join", slot_idx), prev_bg, bg, stroke));
        }

        row = row.child(build_app_slot(
            state,
            slot_idx,
            match_pos,
            entry_idx,
            slot_w,
            is_selected,
            is_hovered,
            bg,
        ));

        prev_bg = bg;
        prev_selected = is_selected;
    }

    row.child(separator(
        "sep-drun-res-r",
        SepShape::SlashR,
        last_bg,
        Color::TRANSPARENT,
    ))
    .into()
}

fn build_empty_results(state: &State, total_w: f32, inner_total: f32) -> Element<Msg> {
    let bg = slot_bg(false);
    let query = state.drun.input.display_text();
    let msg = if query.trim().is_empty() {
        String::from("No applications found")
    } else {
        format!("No matches for \"{}\"", query.trim())
    };

    Row::new()
        .key("drun-empty-row")
        .size(Size::px(total_w.logical_px(), BAR_HEIGHT.logical_px()))
        .align(Align::Center)
        .child(separator(
            "sep-drun-empty-l",
            SepShape::SlashLInv,
            bg,
            Color::TRANSPARENT,
        ))
        .child(
            Group::new()
                .key("drun-empty-body")
                .size(Size::px(inner_total.logical_px(), BAR_HEIGHT.logical_px()))
                .child(Rect::new().size(Size::fill()).fill(bg))
                .child(
                    Text::new(msg)
                        .font(font_ui())
                        .font_size(11.logical_px())
                        .color(OVERLAY0)
                        .position(Position::relative_to_parent(
                            Percent::new(50.0),
                            Percent::new(50.0),
                        ))
                        .anchor(ElementAnchor::Center),
                ),
        )
        .child(separator(
            "sep-drun-empty-r",
            SepShape::SlashR,
            bg,
            Color::TRANSPARENT,
        ))
        .into()
}

#[allow(clippy::too_many_arguments)]
fn build_app_slot(
    state: &State,
    slot_idx: usize,
    match_pos: usize,
    entry_idx: usize,
    slot_w: f32,
    is_selected: bool,
    is_hovered: bool,
    bg: Color,
) -> Element<Msg> {
    let Some(entry) = state.drun.entries.get(entry_idx) else {
        return Group::new()
            .key(("drun-slot-missing", slot_idx))
            .size(Size::px(slot_w.logical_px(), BAR_HEIGHT.logical_px()))
            .child(Rect::new().size(Size::fill()).fill(bg))
            .into();
    };

    let icon_asset = entry
        .icon
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(|name| {
            state
                .path_asset_cache
                .peek(&format!("drun-icon:{name}"))
                .copied()
        });

    let icon_el: Element<Msg> = if let Some(img) = icon_asset {
        Image::new(img)
            .size(Size::px(
                DRUN_ICON_SIZE.logical_px(),
                DRUN_ICON_SIZE.logical_px(),
            ))
            .fit(ImageFit::Contain)
            .into()
    } else {
        Group::new()
            .size(Size::px(
                DRUN_ICON_SIZE.logical_px(),
                DRUN_ICON_SIZE.logical_px(),
            ))
            .child(
                Text::new("\u{f135}")
                    .font(font_ui())
                    .font_size(11.logical_px())
                    .optical_bounds(true)
                    .color(if is_selected { MAUVE } else { OVERLAY0 })
                    .position(Position::relative_to_parent(
                        Percent::new(50.0),
                        Percent::new(50.0),
                    ))
                    .anchor(ElementAnchor::Center),
            )
            .into()
    };

    let text_budget_w = (slot_w - DRUN_ICON_SIZE - 14.0).max(28.0);
    let max_cols = ((text_budget_w / 7.0).floor() as usize).max(6);
    let label = marquee_slice(&entry.name, max_cols, state.drun.marquee_tick);
    let text_color = if is_selected || is_hovered {
        TEXT
    } else {
        SUBTEXT1
    };
    let text_font = if is_selected {
        font_ui_semibold()
    } else {
        font_ui()
    };

    let mut slot = Group::new()
        .key(("drun-slot", slot_idx))
        .size(Size::px(slot_w.logical_px(), BAR_HEIGHT.logical_px()))
        .on_hover(Msg::Hover(HoverTarget::DrunEntry(slot_idx)))
        .on_leave(Msg::Leave(HoverTarget::DrunEntry(slot_idx)))
        .on_click(Msg::DrunLaunchIndex(match_pos))
        .child(Rect::new().size(Size::fill()).fill(bg))
        .child(
            Row::new()
                .size(Size::fill())
                .padding(Edges::symmetric(0.logical_px(), 4.logical_px()))
                .gap(6.logical_px())
                .align(Align::Center)
                .child(icon_el)
                .child(
                    Text::new(label)
                        .font(text_font)
                        .font_size(11.5.logical_px())
                        .color(text_color),
                ),
        );

    if is_selected {
        slot = slot.child(
            Rect::new()
                .size(Size::px(slot_w.logical_px(), 1.5.logical_px()))
                .fill(MAUVE)
                .position(Position::relative_to_parent(
                    Percent::new(0.0),
                    Percent::new(100.0),
                ))
                .anchor(ElementAnchor::BottomLeft),
        );
    }

    slot.into()
}
