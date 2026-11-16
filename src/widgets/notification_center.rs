//! Overlay Notification Center on the left side of the screen.

mod card;
mod header;

use waywi::prelude::*;

use crate::app::state::{Msg, State};
use crate::theme::{
    BAR_HEIGHT, CRUST, NOTIF_CENTER_TOP_MARGIN, NOTIF_CENTER_WIDTH, OVERLAY0, SURFACE1, font_ui,
};

use card::{BASE_CARD_HEIGHT, build_notification_card};
use header::{HEADER_HEIGHT, build_header};

const CARD_GAP: f32 = 4.0;
const LIST_PAD_Y: f32 = 12.0;

pub fn build_notification_center_surface(state: &State) -> Surface<Msg> {
    let p = state.notifications.center_progress.clamp(0.0, 1.0);
    let ease = 1.0 - (1.0 - p).powi(3);

    let max_total_height = (state.screen_height - NOTIF_CENTER_TOP_MARGIN - 16.0).max(200.0);
    let max_body_height = (max_total_height - HEADER_HEIGHT - 8.0).max(120.0);
    let visible_slots =
        (((max_body_height - LIST_PAD_Y + CARD_GAP) / (BASE_CARD_HEIGHT + CARD_GAP)).floor()
            as usize)
            .max(1);

    let start = state
        .notifications
        .scroll_index
        .min(state.notifications.items.len());
    let visible_slice: Vec<_> = state
        .notifications
        .items
        .iter()
        .skip(start)
        .take(visible_slots)
        .collect();

    let mut list_col = Column::new()
        .size(Size::new(Length::Fill, Length::Auto))
        .gap(CARD_GAP.logical_px())
        .padding(Edges::all(6.logical_px()));

    if visible_slice.is_empty() {
        list_col = list_col.child(
            Row::new()
                .size(Size::new(Length::Fill, 48.logical_px()))
                .align(Align::Center)
                .padding(Edges::symmetric(0.logical_px(), 10.logical_px()))
                .child(
                    Text::new("No notifications")
                        .font(font_ui())
                        .font_size(12.logical_px())
                        .color(OVERLAY0),
                ),
        );
    } else {
        for item in &visible_slice {
            list_col = list_col.child(build_notification_card(state, item));
        }
    }

    let body_height = if visible_slice.is_empty() {
        60.0
    } else {
        let cards_h = (visible_slice.len() as f32) * BASE_CARD_HEIGHT;
        let gaps = visible_slice.len().saturating_sub(1) as f32 * CARD_GAP;
        (cards_h + gaps + LIST_PAD_Y + 2.0).min(max_body_height)
    };

    let scrollable_body = Clip::new(list_col)
        .size(Size::new(Length::Fill, body_height.logical_px()))
        .on_scroll(Msg::NotifCenterScroll);

    let full_height = HEADER_HEIGHT + 1.0 + body_height;
    let animated_height = (full_height * ease).max(1.0);
    let animated_y = (BAR_HEIGHT - 8.0) + (NOTIF_CENTER_TOP_MARGIN - (BAR_HEIGHT - 8.0)) * ease;

    let card_inner = Group::new()
        .size(Size::new(
            NOTIF_CENTER_WIDTH.logical_px(),
            full_height.logical_px(),
        ))
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(CRUST)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), SURFACE1),
        )
        .child(
            Column::new()
                .size(Size::new(Length::Fill, Length::Auto))
                .child(build_header(state))
                .child(
                    Rect::new()
                        .size(Size::new(Length::Fill, 1.logical_px()))
                        .fill(SURFACE1),
                )
                .child(scrollable_body),
        );

    let animated_card = Clip::new(card_inner)
        .key("notification-center-box")
        .position(Position::absolute(8.logical_px(), animated_y.logical_px()))
        .size(Size::new(
            NOTIF_CENTER_WIDTH.logical_px(),
            animated_height.logical_px(),
        ))
        .on_pointer_down(|_| Msg::Noop);

    let backdrop = Group::new()
        .key("notification-center-backdrop")
        .size(Size::fill())
        .on_pointer_down(|_| Msg::CloseNotificationCenter)
        .child(animated_card);

    Surface::floating("bar-notification-center", Output::Primary)
        .layer(Layer::Overlay)
        .ignore_exclusive_zone()
        .anchor(Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right)
        .size(Size::fill())
        .input_mode(InputMode::Full)
        .keyboard_interactivity(KeyboardInteractivity::OnDemand)
        .background(Color::TRANSPARENT)
        .on_key(|ev| {
            if ev.key == Key::Escape {
                Some(Msg::CloseNotificationCenter)
            } else {
                None
            }
        })
        .child(backdrop)
}
