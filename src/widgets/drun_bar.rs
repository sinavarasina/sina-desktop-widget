//! Top-bar `drun` overlay launcher matching the main bar's `24px` Powerline aesthetic.

mod slots;

use waywi::prelude::*;

use crate::app::state::{Msg, State};
use crate::theme::{
    BAR_HEIGHT, CRUST, MANTLE, MAUVE, OVERLAY0, POPUP_TOP_MARGIN, SEP_WIDTH, TEXT, font_ui,
    font_ui_semibold,
};
use crate::widgets::separators::{SepShape, separator};

use slots::{COUNT_PILL_W, build_hidden_count_pill, build_results_section};

const RUNNER_PILL_W: f32 = 28.0;
const INPUT_PAD_X: f32 = 6.0;

pub fn build_drun_surface(state: &State) -> Surface<Msg> {
    let ease = state.drun.progress();
    let animated_h = (BAR_HEIGHT * ease).max(1.0);

    let input_w = state.drun.input_box_width(state.bar_width);
    let left_total_w = RUNNER_PILL_W + COUNT_PILL_W + 4.0 * SEP_WIDTH + input_w;
    let right_total_w = COUNT_PILL_W + SEP_WIDTH;
    let results_w = (state.bar_width - left_total_w - right_total_w).max(80.0);

    let total_matches = state.drun.match_indices.len();
    let visible_len = state.drun.visible_match_indices().len();
    let left_hidden = state.drun.scroll_start.min(total_matches);
    let right_hidden = total_matches.saturating_sub(left_hidden + visible_len);

    let runner_pill = build_runner_pill();
    let input_box = build_input_field(state, input_w);
    let left_count_pill = build_hidden_count_pill(
        "drun-left-cnt",
        "sep-drun-lc-l",
        left_hidden,
        SepShape::CurveL,
        Some(("sep-drun-lc-r", SepShape::SlashR)),
    );
    let results_section = build_results_section(state, results_w);
    let right_count_pill = build_hidden_count_pill(
        "drun-right-cnt",
        "sep-drun-rc-l",
        right_hidden,
        SepShape::SlashLInv,
        None,
    );

    let strip = Row::new()
        .key("drun-strip-row")
        .size(Size::new(
            state.bar_width.logical_px(),
            BAR_HEIGHT.logical_px(),
        ))
        .align(Align::Center)
        .on_scroll(Msg::DrunScroll)
        .child(runner_pill)
        .child(separator(
            "sep-drun-runner-r",
            SepShape::ArrowR,
            CRUST,
            MANTLE,
        ))
        .child(input_box)
        .child(separator(
            "sep-drun-in-r",
            SepShape::ArrowR,
            MANTLE,
            Color::TRANSPARENT,
        ))
        .child(left_count_pill)
        .child(results_section)
        .child(right_count_pill);

    let animated_strip = Clip::new(strip)
        .key("drun-strip-clip")
        .position(Position::absolute(
            0.logical_px(),
            POPUP_TOP_MARGIN.logical_px(),
        ))
        .size(Size::new(
            state.bar_width.logical_px(),
            animated_h.logical_px(),
        ))
        .on_pointer_down(|_| Msg::Noop);

    let backdrop = Group::new()
        .key("drun-backdrop")
        .size(Size::fill())
        .on_pointer_down(|_| Msg::CloseDrun)
        .child(animated_strip);

    let ime_cfg = ImeConfig::enabled()
        .from_text_input(&state.drun.input)
        .cursor_rect(state.drun.ime_cursor_rect(state.bar_width));

    Surface::floating("bar-drun", Output::Primary)
        .layer(Layer::Overlay)
        .ignore_exclusive_zone()
        .anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT)
        .size(state.viewport_size())
        .input_mode(InputMode::Full)
        .keyboard_interactivity(KeyboardInteractivity::Exclusive)
        .ime(ime_cfg)
        .background(Color::TRANSPARENT)
        .on_close(Msg::CloseDrun)
        .on_key(|ev| Some(Msg::DrunKey(ev)))
        .on_ime(|ev| Some(Msg::DrunIme(ev)))
        .child(backdrop)
}

fn build_runner_pill() -> Element<Msg> {
    Group::new()
        .key("drun-runner-pill")
        .size(Size::new(
            RUNNER_PILL_W.logical_px(),
            BAR_HEIGHT.logical_px(),
        ))
        .child(Rect::new().size(Size::fill()).fill(CRUST))
        .child(
            Text::new(">_")
                .font(font_ui_semibold())
                .font_size(11.logical_px())
                .bounds_mode(TextBoundsMode::Optical)
                .color(MAUVE)
                .position(Position::relative_to_parent(
                    Percent::new(50.0),
                    Percent::new(50.0),
                ))
                .anchor(ElementAnchor::Center),
        )
        .into()
}

fn build_input_field(state: &State, input_w: f32) -> Element<Msg> {
    let inner_w = (input_w - INPUT_PAD_X * 2.0).max(24.0);
    let display = state.drun.input.display_text();
    let has_text = !display.is_empty();
    let pan_x = (state.drun.cursor_x - (inner_w - 4.0)).max(0.0);
    let cursor_draw_x = (state.drun.cursor_x - pan_x).clamp(0.0, inner_w - 2.0);

    let text_el: Element<Msg> = if has_text {
        Text::new(display)
            .font(font_ui())
            .font_size(11.5.logical_px())
            .line_height(1.2)
            .color(TEXT)
            .position(Position::RelativeToParent {
                x: Percent::new(0.0),
                y: Percent::new(50.0),
                offset_x: (-pan_x).logical_px(),
                offset_y: 0.logical_px(),
            })
            .anchor(ElementAnchor::CenterLeft)
            .into()
    } else {
        Text::new("Search apps...")
            .font(font_ui())
            .font_size(11.logical_px())
            .line_height(1.2)
            .color(OVERLAY0)
            .position(Position::relative_to_parent(
                Percent::new(0.0),
                Percent::new(50.0),
            ))
            .anchor(ElementAnchor::CenterLeft)
            .into()
    };

    let cursor_el = Rect::new()
        .size(Size::new(1.5.logical_px(), 13.logical_px()))
        .fill(MAUVE)
        .position(Position::RelativeToParent {
            x: Percent::new(0.0),
            y: Percent::new(50.0),
            offset_x: cursor_draw_x.logical_px(),
            offset_y: 0.logical_px(),
        })
        .anchor(ElementAnchor::CenterLeft);

    Group::new()
        .key("drun-input-field")
        .size(Size::new(input_w.logical_px(), BAR_HEIGHT.logical_px()))
        .cursor(CursorShape::Text)
        .child(Rect::new().size(Size::fill()).fill(MANTLE))
        .child(
            Clip::new(
                Group::new()
                    .size(Size::new(inner_w.logical_px(), BAR_HEIGHT.logical_px()))
                    .child(text_el)
                    .child(cursor_el),
            )
            .position(Position::relative_to_parent(
                Percent::new(50.0),
                Percent::new(50.0),
            ))
            .anchor(ElementAnchor::Center)
            .size(Size::new(inner_w.logical_px(), BAR_HEIGHT.logical_px())),
        )
        .into()
}
