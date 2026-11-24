//! Right-edge clock pill with hover-revealed Powerline menu button (quick status popup)
//! and middle-click calendar toggle.

use waywi::prelude::*;

use crate::app::state::{HoverTarget, Msg, State};
use crate::theme::{
    BAR_HEIGHT, CLOCK_PAD_H, CRUST, MANTLE, MAUVE, SEP_WIDTH, SURFACE0, TEXT, font_ui,
    font_ui_bold,
};
use crate::widgets::separators::{SepShape, separator};

const MENU_BODY_W: f32 = 26.0;
const MENU_FULL_W: f32 = SEP_WIDTH + MENU_BODY_W;

/// Builds the Powerline separator + hover-revealed Clock Menu button + Clock pill.
pub fn build_clock_group(state: &State) -> Element<Msg> {
    let p = state.clock_menu_progress.clamp(0.0, 1.0);
    let ease = 1.0 - (1.0 - p).powi(3);
    let reveal_w = MENU_FULL_W * ease;

    let menu_hovered = state.hovered == HoverTarget::ClockMenu || state.quick_menu.popup_target;
    let menu_bg = if menu_hovered { SURFACE0 } else { MANTLE };
    let menu_icon_col = if state.quick_menu.popup_target {
        MAUVE
    } else {
        TEXT
    };

    let mut row = Row::new()
        .key("mod-clock-group")
        .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
        .align(Align::Center);

    if reveal_w > 0.5 {
        let menu_body = Group::new()
            .key("mod-clock-menu-body")
            .size(Size::new(MENU_BODY_W.logical_px(), BAR_HEIGHT.logical_px()))
            .child(Rect::new().size(Size::fill()).fill(menu_bg))
            .child(
                Text::new("\u{f0c9}")
                    .font(font_ui())
                    .font_size(11.logical_px())
                    .bounds_mode(TextBoundsMode::Optical)
                    .color(menu_icon_col)
                    .position(Position::relative_to_parent(
                        Percent::new(50.0),
                        Percent::new(50.0),
                    ))
                    .anchor(ElementAnchor::Center),
            );

        let menu_pill = Row::new()
            .key("mod-clock-menu")
            .size(Size::new(MENU_FULL_W.logical_px(), BAR_HEIGHT.logical_px()))
            .align(Align::Center)
            .on_hover(Msg::Hover(HoverTarget::ClockMenu))
            .on_leave(Msg::Leave(HoverTarget::ClockMenu))
            .on_click(Msg::ClockMenuClick)
            .child(separator(
                "sep-clock-menu-l",
                SepShape::ArrowL,
                menu_bg,
                Color::TRANSPARENT,
            ))
            .child(menu_body);

        let clipped_menu = Clip::new(menu_pill)
            .key("mod-clock-menu-clip")
            .size(Size::new(reveal_w.logical_px(), BAR_HEIGHT.logical_px()));

        row = row.child(clipped_menu).child(separator(
            "sep-r-4",
            SepShape::ArrowL,
            CRUST,
            menu_bg,
        ));
    } else {
        row = row.child(separator(
            "sep-r-4",
            SepShape::ArrowL,
            CRUST,
            Color::TRANSPARENT,
        ));
    }

    row.child(build_clock(state)).into()
}

pub fn build_clock(state: &State) -> Element<Msg> {
    let text = state.clock_display();
    Group::new()
        .key("mod-clock")
        .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
        .on_hover(Msg::Hover(HoverTarget::Clock))
        .on_leave(Msg::Leave(HoverTarget::Clock))
        .on_pointer_down(Msg::ClockPointerDown)
        .on_scroll(Msg::ClockScroll)
        .child(Rect::new().size(Size::fill()).fill(CRUST))
        .child(
            Row::new()
                .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
                .padding(Edges::symmetric(0.logical_px(), CLOCK_PAD_H.logical_px()))
                .align(Align::Center)
                .child(
                    Text::new(text)
                        .font(font_ui_bold())
                        .font_size(12.logical_px())
                        .line_height(1.15)
                        .color(TEXT),
                ),
        )
        .into()
}
