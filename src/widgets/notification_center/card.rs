//! Individual notification card inside the Notification Center with marquee text scrolling.

use waywi::prelude::*;

use crate::app::notifications::NotificationItem;
use crate::app::state::{HoverTarget, Msg, State};
use crate::theme::{
    BLUE, MANTLE, NOTIF_CARD_TEXT_WIDTH, NOTIF_CENTER_ICON_SIZE, OVERLAY0, RED, SUBTEXT0,
    SUBTEXT1, SURFACE0, SURFACE1, TEXT, font_ui, font_ui_bold,
};
use crate::widgets::notification_icon::build_notif_icon;
use crate::widgets::text::marquee_slice;

const MAX_SUMMARY_COLS: usize = 28;
const MAX_BODY_COLS: usize = 39;
pub const BASE_CARD_HEIGHT: f32 = 62.0;

pub fn build_notification_card(state: &State, item: &NotificationItem) -> Element<Msg> {
    let id = item.stored.id;
    let is_hovered = state.hovered == HoverTarget::NotifCenterItem(id);
    let bg = if is_hovered { SURFACE0 } else { MANTLE };
    let border_col = if item.stored.urgency == 2 {
        RED
    } else if !item.stored.read {
        BLUE
    } else {
        SURFACE1
    };

    let raw_summary = if item.stored.summary.is_empty() {
        item.stored.app_name.as_str()
    } else {
        item.stored.summary.as_str()
    };
    let summary_text = marquee_slice(raw_summary, MAX_SUMMARY_COLS, state.notifications.marquee_tick);
    let app_text = marquee_slice(&item.stored.app_name, 14, state.notifications.marquee_tick);

    let header_row = Clip::new(
        Row::new()
            .size(Size::new(NOTIF_CARD_TEXT_WIDTH.logical_px(), 16.logical_px()))
            .gap(6.logical_px())
            .align(Align::Center)
            .child(
                Text::new(summary_text)
                    .font(font_ui_bold())
                    .font_size(11.logical_px())
                    .color(TEXT),
            )
            .child(
                Text::new(app_text)
                    .font(font_ui())
                    .font_size(10.logical_px())
                    .color(OVERLAY0),
            ),
    )
    .size(Size::new(NOTIF_CARD_TEXT_WIDTH.logical_px(), 16.logical_px()));

    let mut text_col = Column::new()
        .size(Size::new(NOTIF_CARD_TEXT_WIDTH.logical_px(), Length::Auto))
        .gap(2.logical_px())
        .child(header_row);

    let clean_body = item.stored.body.trim().replace('\n', " ");
    if !clean_body.is_empty() {
        let body_text = marquee_slice(&clean_body, MAX_BODY_COLS, state.notifications.marquee_tick);
        text_col = text_col.child(
            Clip::new(
                Row::new()
                    .size(Size::new(NOTIF_CARD_TEXT_WIDTH.logical_px(), 16.logical_px()))
                    .align(Align::Center)
                    .child(
                        Text::new(body_text)
                            .font(font_ui())
                            .font_size(11.logical_px())
                            .line_height(1.2)
                            .color(if item.stored.read {
                                SUBTEXT0
                            } else {
                                SUBTEXT1
                            }),
                    ),
            )
            .size(Size::new(NOTIF_CARD_TEXT_WIDTH.logical_px(), 16.logical_px())),
        );
    }

    Group::new()
        .key(("notif-card", id))
        .size(Size::new(Length::Fill, Length::Auto))
        .on_hover(Msg::Hover(HoverTarget::NotifCenterItem(id)))
        .on_leave(Msg::Leave(HoverTarget::NotifCenterItem(id)))
        .on_pointer_down(move |ev| Msg::NotifCenterCardPointerDown { id, ev })
        // WayWi routes scroll to the single topmost hit-test node without bubbling; the card
        // wins over the list Clip whenever the pointer is over it, so it must carry the
        // scroll handler itself or scrolling over cards would do nothing.
        .on_scroll(Msg::NotifCenterScroll)
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(bg)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), border_col),
        )
        .child(
            Row::new()
                .size(Size::new(Length::Fill, Length::Auto))
                .padding(Edges::symmetric(8.logical_px(), 10.logical_px()))
                .gap(8.logical_px())
                .align(Align::Center)
                .child(build_notif_icon(item.icon, false, NOTIF_CENTER_ICON_SIZE))
                .child(text_col),
        )
        .into()
}
