//! Left-bar notification pill (circular icons + X+ count + DND) and incoming ticker.

mod ticker;

use waywi::prelude::*;

use crate::app::state::{HoverTarget, Msg, State};
use crate::theme::{
    BAR_HEIGHT, MANTLE, MAUVE, NOTIF_ICON_SIZE, NOTIF_MAX_BAR_ICONS, RED, SUBTEXT1, SURFACE0,
    font_ui, font_ui_semibold,
};
use crate::widgets::notification_icon::build_notif_icon;
use crate::widgets::separators::{SepShape, separator};

use ticker::build_incoming_ticker;

pub fn build_notification_segment(state: &State) -> Element<Msg> {
    let pill_bg = if state.hovered == HoverTarget::Notifications {
        SURFACE0
    } else {
        MANTLE
    };

    let mut content = Row::new()
        .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
        .padding(Edges::symmetric(0.logical_px(), 6.logical_px()))
        .gap(4.logical_px())
        .align(Align::Center);

    if state.notifications.items.is_empty() {
        content = content.child(build_notif_icon(
            None,
            state.notifications.dnd,
            NOTIF_ICON_SIZE,
        ));
    } else {
        for (idx, item) in state
            .notifications
            .items
            .iter()
            .take(NOTIF_MAX_BAR_ICONS)
            .enumerate()
        {
            content = content.child(
                Group::new()
                    .key(("notif-bar-icon", idx))
                    .size(Size::new(
                        NOTIF_ICON_SIZE.logical_px(),
                        NOTIF_ICON_SIZE.logical_px(),
                    ))
                    .child(build_notif_icon(item.icon, false, NOTIF_ICON_SIZE)),
            );
        }
    }

    let count_label = format!("{}+", state.notifications.total_count);
    content = content.child(
        Text::new(count_label)
            .font(font_ui_semibold())
            .font_size(11.logical_px())
            .line_height(1.1)
            .color(if state.notifications.total_count > 0 {
                MAUVE
            } else {
                SUBTEXT1
            }),
    );

    if state.notifications.dnd && !state.notifications.items.is_empty() {
        content = content.child(
            Text::new("\u{f009b}")
                .font(font_ui())
                .font_size(12.logical_px())
                .bounds_mode(TextBoundsMode::Optical)
                .color(RED),
        );
    }

    let pill = Group::new()
        .key("notifications-pill")
        .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
        .on_hover(Msg::Hover(HoverTarget::Notifications))
        .on_leave(Msg::Leave(HoverTarget::Notifications))
        .on_pointer_down(Msg::NotifBarPointerDown)
        .child(Rect::new().size(Size::fill()).fill(pill_bg))
        .child(content);

    let mut row = Row::new()
        .key("notifications-segment")
        .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
        .align(Align::Center)
        .child(separator(
            "sep-notif-l",
            SepShape::SlashLInv,
            pill_bg,
            Color::TRANSPARENT,
        ))
        .child(pill)
        .child(separator(
            "sep-notif-r1",
            SepShape::SlashR,
            pill_bg,
            Color::TRANSPARENT,
        ))
        .child(separator(
            "sep-notif-r2",
            SepShape::SlashRThin,
            SURFACE0,
            Color::TRANSPARENT,
        ));

    if let Some(ticker) = &state.notifications.active_ticker {
        row = row.child(build_incoming_ticker(ticker));
    }

    row.into()
}
