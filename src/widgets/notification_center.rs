//! Overlay Notification Center on the left side of the screen.

pub mod card;
mod header;

use waywi::prelude::*;

use crate::app::notifications::NotificationItem;
use crate::app::state::{Msg, State};
use crate::theme::{
    BAR_HEIGHT, CRUST, NOTIF_CENTER_TOP_MARGIN, NOTIF_CENTER_WIDTH, OVERLAY0, SURFACE1, font_ui,
};

use card::{build_notification_card, card_height};
use header::{HEADER_HEIGHT, build_header};

const CARD_GAP: f32 = 4.0;
const LIST_PAD_Y: f32 = 12.0;

#[must_use]
fn max_list_body_height(screen_height: f32) -> f32 {
    let max_total_height = (screen_height - NOTIF_CENTER_TOP_MARGIN - 16.0).max(200.0);
    (max_total_height - HEADER_HEIGHT - 8.0).max(120.0)
}

/// Highest valid `scroll_index`: the start of the last window of variable-height cards that
/// fits in `screen_height` without leaving empty space at the bottom.
#[must_use]
pub fn max_notif_scroll_index(items: &[NotificationItem], screen_height: f32) -> usize {
    if items.is_empty() {
        return 0;
    }
    let avail = (max_list_body_height(screen_height) - LIST_PAD_Y - 2.0).max(1.0);
    let mut used = 0.0_f32;
    let mut start = items.len();
    for (idx, item) in items.iter().enumerate().rev() {
        let h = card_height(item);
        let extra = if idx + 1 < items.len() { CARD_GAP } else { 0.0 };
        if idx < items.len() - 1 && used + h + extra > avail {
            break;
        }
        used += h + extra;
        start = idx;
    }
    start
}

fn collect_visible_cards(items: &[NotificationItem], start: usize, screen_height: f32) -> (&[NotificationItem], f32) {
    if items.is_empty() || start >= items.len() {
        return (&[], 60.0);
    }
    let max_body = max_list_body_height(screen_height);
    let avail = (max_body - LIST_PAD_Y - 2.0).max(1.0);
    let mut used = 0.0_f32;
    let mut end = start;
    for (offset, item) in items[start..].iter().enumerate() {
        let h = card_height(item);
        let gap = if offset > 0 { CARD_GAP } else { 0.0 };
        if offset > 0 && used + gap + h > avail {
            break;
        }
        used += gap + h;
        end = start + offset + 1;
    }
    let body_height = (used + LIST_PAD_Y + 2.0).min(max_body);
    (&items[start..end], body_height)
}

pub fn build_notification_center_surface(state: &State) -> Surface<Msg> {
    let p = state.notifications.center_progress.clamp(0.0, 1.0);
    let ease = 1.0 - (1.0 - p).powi(3);

    let start = state
        .notifications
        .scroll_index
        .min(max_notif_scroll_index(&state.notifications.items, state.screen_height));
    let (visible_slice, body_height) =
        collect_visible_cards(&state.notifications.items, start, state.screen_height);

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
        for item in visible_slice {
            list_col = list_col.child(build_notification_card(state, item));
        }
    }

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
