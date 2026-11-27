//! Individual notification card inside the Notification Center with marquee text scrolling
//! and horizontally scrollable multi-action buttons when 2 or more actions are present.

use waywi::prelude::*;

use crate::app::notifications::NotificationItem;
use crate::app::state::{HoverTarget, Msg, State};
use crate::theme::{
    BLUE, MANTLE, MAUVE, NOTIF_CARD_TEXT_WIDTH, NOTIF_CENTER_ICON_SIZE, OVERLAY0, RED, SUBTEXT0,
    SUBTEXT1, SURFACE0, SURFACE1, TEXT, font_ui, font_ui_bold, font_ui_semibold,
};
use crate::widgets::notification_icon::build_notif_icon;
use crate::widgets::text::marquee_slice;

const MAX_SUMMARY_COLS: usize = 28;
const MAX_BODY_COLS: usize = 39;
pub const BASE_CARD_HEIGHT: f32 = 62.0;
pub const ACTION_CARD_HEIGHT: f32 = 86.0;
const ACTION_ROW_HEIGHT: f32 = 22.0;
const ACTION_BTN_GAP: f32 = 6.0;
const ACTION_BTN_PAD_H: f32 = 14.0;
const ACTION_CHAR_PX: f32 = 6.0;

#[must_use]
pub fn visible_actions(item: &NotificationItem) -> Vec<(&str, &str)> {
    item.stored
        .actions
        .iter()
        .filter_map(|(k, l)| {
            let kt = k.trim();
            let lt = l.trim();
            (!kt.is_empty() && !lt.is_empty()).then_some((kt, lt))
        })
        .collect()
}

#[must_use]
pub fn has_multi_actions(item: &NotificationItem) -> bool {
    visible_actions(item).len() >= 2
}

#[must_use]
pub fn card_height(item: &NotificationItem) -> f32 {
    if has_multi_actions(item) {
        ACTION_CARD_HEIGHT
    } else {
        BASE_CARD_HEIGHT
    }
}

#[must_use]
pub fn action_max_scroll_x(item: &NotificationItem) -> f32 {
    let actions = visible_actions(item);
    if actions.len() < 2 {
        return 0.0;
    }
    let content_w = action_row_content_width(&actions);
    (content_w - NOTIF_CARD_TEXT_WIDTH).max(0.0)
}

fn action_row_content_width(actions: &[(&str, &str)]) -> f32 {
    let buttons_w: f32 = actions
        .iter()
        .map(|(_, label)| (label.chars().count() as f32) * ACTION_CHAR_PX + ACTION_BTN_PAD_H)
        .sum();
    let gaps_w = (actions.len().saturating_sub(1) as f32) * ACTION_BTN_GAP;
    buttons_w + gaps_w
}

pub fn build_notification_card(state: &State, item: &NotificationItem) -> Element<Msg> {
    let id = item.uid;
    let is_hovered = state.hovered == HoverTarget::NotifCenterItem(id)
        || matches!(state.hovered, HoverTarget::NotifAction { id: aid, .. } if aid == id);
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
    let summary_text = marquee_slice(
        raw_summary,
        MAX_SUMMARY_COLS,
        state.notifications.marquee_tick,
    );
    let app_text = marquee_slice(&item.stored.app_name, 14, state.notifications.marquee_tick);

    let header_row = Clip::new(
        Row::new()
            .size(Size::new(
                NOTIF_CARD_TEXT_WIDTH.logical_px(),
                16.logical_px(),
            ))
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
    .size(Size::new(
        NOTIF_CARD_TEXT_WIDTH.logical_px(),
        16.logical_px(),
    ));

    let mut text_col = Column::new()
        .size(Size::new(NOTIF_CARD_TEXT_WIDTH.logical_px(), Length::Auto))
        .gap(3.logical_px())
        .child(header_row);

    let clean_body = item.stored.body.trim().replace('\n', " ");
    if !clean_body.is_empty() {
        let body_text = marquee_slice(&clean_body, MAX_BODY_COLS, state.notifications.marquee_tick);
        text_col = text_col.child(
            Clip::new(
                Row::new()
                    .size(Size::new(
                        NOTIF_CARD_TEXT_WIDTH.logical_px(),
                        16.logical_px(),
                    ))
                    .align(Align::Center)
                    .child(
                        Text::new(body_text)
                            .font(font_ui())
                            .font_size(11.logical_px())
                            .line_height(1.2)
                            .color(if item.stored.read { SUBTEXT0 } else { SUBTEXT1 }),
                    ),
            )
            .size(Size::new(
                NOTIF_CARD_TEXT_WIDTH.logical_px(),
                16.logical_px(),
            )),
        );
    }

    let actions = visible_actions(item);
    let height = card_height(item);
    if actions.len() >= 2 {
        text_col = text_col.child(build_actions_row(state, item, id, &actions));
    }

    Group::new()
        .key(("notif-card", id))
        .size(Size::new(Length::Fill, height.logical_px()))
        .cursor(CursorShape::Pointer)
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
                .size(Size::fill())
                .padding(Edges::symmetric(8.logical_px(), 10.logical_px()))
                .gap(8.logical_px())
                .align(Align::Center)
                .child(build_notif_icon(item.icon, false, NOTIF_CENTER_ICON_SIZE))
                .child(text_col),
        )
        .into()
}

fn build_actions_row(
    state: &State,
    item: &NotificationItem,
    id: u32,
    actions: &[(&str, &str)],
) -> Element<Msg> {
    let max_scroll = action_max_scroll_x(item);
    let scroll_x = state
        .notifications
        .action_scroll_x
        .get(&id)
        .copied()
        .unwrap_or(0.0)
        .clamp(0.0, max_scroll);

    let mut row = Row::new()
        .key(("notif-actions-row", id))
        .size(Size::new(Length::Auto, ACTION_ROW_HEIGHT.logical_px()))
        .gap(ACTION_BTN_GAP.logical_px())
        .align(Align::Center)
        .position(Position::absolute((-scroll_x).logical_px(), 0.logical_px()));

    for (action_idx, (action_key, label)) in actions.iter().enumerate() {
        let btn_hovered = state.hovered == HoverTarget::NotifAction { id, action_idx };
        let btn_bg = if btn_hovered { SURFACE1 } else { MANTLE };
        let btn_border = if btn_hovered { MAUVE } else { SURFACE1 };
        let btn_fg = if btn_hovered { TEXT } else { SUBTEXT1 };
        let key_owned = (*action_key).to_owned();

        let btn = Group::new()
            .key(("notif-action-btn", id, action_idx))
            .size(Size::auto())
            .cursor(CursorShape::Pointer)
            .on_hover(Msg::Hover(HoverTarget::NotifAction { id, action_idx }))
            .on_leave(Msg::Leave(HoverTarget::NotifAction { id, action_idx }))
            .on_pointer_down(move |ev| {
                if ev.button == Some(MouseButton::Left) {
                    Msg::NotifCenterActionClick {
                        id,
                        action_key: key_owned.clone(),
                    }
                } else {
                    Msg::Noop
                }
            })
            .on_scroll(move |sc| Msg::NotifActionScroll { id, sc })
            .child(
                Rect::new()
                    .size(Size::fill())
                    .fill(btn_bg)
                    .corner_radius(0.logical_px())
                    .border(1.logical_px(), btn_border),
            )
            .child(
                Row::new()
                    .padding(Edges::symmetric(2.logical_px(), 6.logical_px()))
                    .align(Align::Center)
                    .child(
                        Text::new((*label).to_owned())
                            .font(font_ui_semibold())
                            .font_size(10.logical_px())
                            .color(btn_fg),
                    ),
            );

        row = row.child(btn);
    }

    Clip::new(
        Group::new()
            .size(Size::new(
                NOTIF_CARD_TEXT_WIDTH.logical_px(),
                ACTION_ROW_HEIGHT.logical_px(),
            ))
            .on_scroll(move |sc| Msg::NotifActionScroll { id, sc })
            .child(row),
    )
    .size(Size::new(
        NOTIF_CARD_TEXT_WIDTH.logical_px(),
        ACTION_ROW_HEIGHT.logical_px(),
    ))
    .into()
}
