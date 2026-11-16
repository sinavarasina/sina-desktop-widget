//! Notification daemon, bar pill, and notification center event handlers.

use waywi::prelude::*;

use crate::app::state::State;

use super::bluetooth::handle_bt_agent_decision;
use super::events::scroll_direction_y;

pub fn handle_notification_event(
    state: &mut State,
    ev: NotificationEvent,
    ctx: &mut EffectCtx<'_>,
) {
    match ev {
        NotificationEvent::Received(notif) => {
            state.notifications.push_incoming(
                *notif,
                &mut state.icon_lookup,
                &mut state.path_asset_cache,
                ctx,
            );
        }
        NotificationEvent::ClosedByCaller { id } => {
            state.notifications.dismiss(id);
        }
        _ => {}
    }
}

pub fn handle_notif_bar_pointer(state: &mut State, ev: PointerEvent, ctx: &mut EffectCtx<'_>) {
    match ev.button {
        Some(MouseButton::Left) => {
            if state.notifications.toggle_center() {
                state.notifications.ensure_visible_icons(
                    state.notifications.scroll_index,
                    12,
                    &mut state.icon_lookup,
                    &mut state.path_asset_cache,
                    ctx,
                );
            }
        }
        Some(MouseButton::Right) => {
            state.notifications.toggle_dnd();
        }
        _ => {}
    }
}

pub fn handle_notif_center_scroll(state: &mut State, sc: ScrollEvent, ctx: &mut EffectCtx<'_>) {
    let step = scroll_direction_y(sc);
    if step == 0 {
        return;
    }
    let next = if step > 0 {
        state
            .notifications
            .scroll_index
            .saturating_add(step as usize)
    } else {
        state
            .notifications
            .scroll_index
            .saturating_sub(step.unsigned_abs() as usize)
    };
    state.notifications.maybe_load_more_from_disk(next + 10);
    let max_idx = state.notifications.items.len().saturating_sub(1);
    state.notifications.scroll_index = next.min(max_idx);
    state.notifications.ensure_visible_icons(
        state.notifications.scroll_index,
        12,
        &mut state.icon_lookup,
        &mut state.path_asset_cache,
        ctx,
    );
}

pub fn handle_notif_card_pointer(state: &mut State, id: u32, ev: PointerEvent, ctx: &EffectCtx<'_>) {
    match ev.button {
        Some(MouseButton::Right) => {
            if state.notifications.dismiss(id)
                && let Ok(dbus) = ctx.dbus()
            {
                let _ = dbus.close_notification(id, NotificationCloseReason::Dismissed);
            }
        }
        Some(MouseButton::Left) => {
            if let Some((notif_id, action_key)) = state.notifications.take_action_and_dismiss(id) {
                if let Some(key) = action_key {
                    invoke_or_route_action(state, notif_id, &key, ctx);
                } else if let Ok(dbus) = ctx.dbus() {
                    let _ = dbus.close_notification(notif_id, NotificationCloseReason::Dismissed);
                }
            }
        }
        _ => {}
    }
}

fn invoke_or_route_action(state: &mut State, id: u32, action_key: &str, ctx: &EffectCtx<'_>) {
    if let Some(req_str) = action_key.strip_prefix("bt-accept:")
        && let Ok(req_id) = req_str.parse::<u64>()
    {
        handle_bt_agent_decision(state, req_id, true, ctx);
        return;
    }
    if let Some(req_str) = action_key.strip_prefix("bt-reject:")
        && let Ok(req_id) = req_str.parse::<u64>()
    {
        handle_bt_agent_decision(state, req_id, false, ctx);
        return;
    }
    if let Ok(dbus) = ctx.dbus() {
        let _ = dbus.invoke_notification_action(id, action_key);
        let _ = dbus.close_notification(id, NotificationCloseReason::Dismissed);
    }
}
