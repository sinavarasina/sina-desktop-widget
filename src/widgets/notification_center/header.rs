//! Notification center header bar (count, DND toggle, Clear All).

use waywi::prelude::*;

use crate::app::state::{HoverTarget, Msg, State};
use crate::theme::{MANTLE, MAUVE, RED, SUBTEXT1, SURFACE0, font_ui_bold, font_ui_semibold};

pub const HEADER_HEIGHT: f32 = 34.0;

pub fn build_header(state: &State) -> Element<Msg> {
    let dnd_bg = if state.hovered == HoverTarget::NotifCenterDnd {
        SURFACE0
    } else {
        MANTLE
    };
    let clear_bg = if state.hovered == HoverTarget::NotifCenterClear {
        SURFACE0
    } else {
        MANTLE
    };

    let dnd_btn = Group::new()
        .key("notif-center-dnd")
        .size(Size::auto())
        .cursor(CursorShape::Pointer)
        .on_hover(Msg::Hover(HoverTarget::NotifCenterDnd))
        .on_leave(Msg::Leave(HoverTarget::NotifCenterDnd))
        .on_click(Msg::NotifCenterToggleDnd)
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(dnd_bg)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), SURFACE0),
        )
        .child(
            Row::new()
                .padding(Edges::symmetric(3.logical_px(), 8.logical_px()))
                .gap(4.logical_px())
                .align(Align::Center)
                .child(
                    Text::new(if state.notifications.dnd {
                        "\u{f009b} DND"
                    } else {
                        "\u{f009a} DND"
                    })
                    .font(font_ui_semibold())
                    .font_size(11.logical_px())
                    .color(if state.notifications.dnd {
                        RED
                    } else {
                        SUBTEXT1
                    }),
                ),
        );

    let clear_btn = Group::new()
        .key("notif-center-clear")
        .size(Size::auto())
        .cursor(CursorShape::Pointer)
        .on_hover(Msg::Hover(HoverTarget::NotifCenterClear))
        .on_leave(Msg::Leave(HoverTarget::NotifCenterClear))
        .on_click(Msg::NotifCenterClearAll)
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(clear_bg)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), SURFACE0),
        )
        .child(
            Row::new()
                .padding(Edges::symmetric(3.logical_px(), 8.logical_px()))
                .align(Align::Center)
                .child(
                    Text::new("Clear")
                        .font(font_ui_semibold())
                        .font_size(11.logical_px())
                        .color(SUBTEXT1),
                ),
        );

    Row::new()
        .size((Length::Fill, HEADER_HEIGHT.logical_px()))
        .padding(Edges::symmetric(0.logical_px(), 10.logical_px()))
        .align(Align::Center)
        .justify(Justify::SpaceBetween)
        .child(
            Text::new(format!(
                "Notifications ({})",
                state.notifications.total_count
            ))
            .font(font_ui_bold())
            .font_size(12.logical_px())
            .color(MAUVE),
        )
        .child(
            Row::new()
                .size(Size::auto())
                .gap(6.logical_px())
                .align(Align::Center)
                .child(dnd_btn)
                .child(clear_btn),
        )
        .into()
}
