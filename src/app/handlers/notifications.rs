//! Notification daemon, bar pill, and notification center event handlers.

use waywi::prelude::*;

use crate::app::state::State;
use crate::widgets::notification_center::card::action_max_scroll_x;
use crate::widgets::notification_center::max_notif_scroll_index;

use super::bluetooth::handle_bt_agent_decision;
use super::events::scroll_direction_y;

const ACTION_SCROLL_STEP_PX: f32 = 32.0;

pub fn handle_notification_event(
    state: &mut State,
    ev: NotificationEvent,
    ctx: &mut EffectCtx<'_>,
) {
    match ev {
        NotificationEvent::Received(notif) => {
            let dnd = state.notifications.dnd;
            state.notifications.push_incoming(
                *notif,
                &mut state.icon_lookup,
                &mut state.path_asset_cache,
                ctx,
            );
            if !dnd {
                state.desktop_sprite.trigger_mouth_react();
            }
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
            state.notifications.toggle_dnd(ctx);
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
    let max_idx = max_notif_scroll_index(&state.notifications.items, state.screen_height);
    state.notifications.scroll_index = next.min(max_idx);
    state.notifications.ensure_visible_icons(
        state.notifications.scroll_index,
        12,
        &mut state.icon_lookup,
        &mut state.path_asset_cache,
        ctx,
    );
}

pub fn handle_notif_card_pointer(
    state: &mut State,
    id: u32,
    ev: PointerEvent,
    ctx: &EffectCtx<'_>,
) {
    match ev.button {
        Some(MouseButton::Right) => {
            if state.notifications.dismiss(id)
                && let Ok(dbus) = ctx.dbus()
            {
                let _ = dbus.close_notification(id, NotificationCloseReason::Dismissed);
            }
        }
        Some(MouseButton::Left) => {
            if let Some((notif_id, action_key, keep_resident)) =
                state.notifications.take_action_and_maybe_dismiss(id)
            {
                if let Some(key) = action_key {
                    invoke_or_route_action(state, notif_id, &key, keep_resident, ctx);
                } else if let Ok(dbus) = ctx.dbus() {
                    let _ = dbus.close_notification(notif_id, NotificationCloseReason::Dismissed);
                }
            }
        }
        _ => {}
    }
}

pub fn handle_notif_action_click(
    state: &mut State,
    id: u32,
    action_key: &str,
    ctx: &EffectCtx<'_>,
) {
    let keep_resident = state.notifications.is_resident(id);
    if keep_resident || state.notifications.dismiss(id) {
        invoke_or_route_action(state, id, action_key, keep_resident, ctx);
    }
}

pub fn handle_notif_action_scroll(state: &mut State, id: u32, sc: ScrollEvent) {
    let delta_x = if sc.discrete_x != 0 {
        (sc.discrete_x as f32) * ACTION_SCROLL_STEP_PX
    } else if sc.delta_x.abs() > 0.01 {
        sc.delta_x * 2.5
    } else {
        let step_y = scroll_direction_y(sc);
        if step_y == 0 {
            return;
        }
        (step_y as f32) * ACTION_SCROLL_STEP_PX
    };

    let Some(item) = state.notifications.items.iter().find(|i| i.stored.id == id) else {
        return;
    };
    let max_scroll = action_max_scroll_x(item);
    if max_scroll <= 0.0 {
        state.notifications.action_scroll_x.remove(&id);
        return;
    }

    let current = state
        .notifications
        .action_scroll_x
        .get(&id)
        .copied()
        .unwrap_or(0.0);
    let next = (current + delta_x).clamp(0.0, max_scroll);
    state.notifications.action_scroll_x.insert(id, next);
}

fn invoke_or_route_action(
    state: &mut State,
    id: u32,
    action_key: &str,
    keep_resident: bool,
    ctx: &EffectCtx<'_>,
) {
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
        if !keep_resident {
            let _ = dbus.close_notification(id, NotificationCloseReason::Dismissed);
        }
    }
}
