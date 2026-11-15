//! Notification bar pill, incoming ticker, DND, RAM limit + lazy cache, and Notification Center tests.

use std::time::SystemTime;

use waywi::prelude::*;

use crate::app::Msg;
use crate::app::notifications::storage::MAX_RAM_NOTIFICATIONS;

use super::support::synthetic_rt;

fn sample_notif(id: u32, summary: &str, body: &str) -> DesktopNotification {
    DesktopNotification {
        id,
        app_name: String::from("notify-send"),
        replaces_id: 0,
        app_icon: String::new(),
        summary: summary.to_owned(),
        body: body.to_owned(),
        actions: vec![(String::from("default"), String::from("Open"))],
        urgency: NotificationUrgency::Normal,
        desktop_entry: None,
        image_path: None,
        image_data: None,
        expire_timeout_ms: 5000,
        received_at: SystemTime::now(),
    }
}

#[test]
fn incoming_notification_scrolls_right_to_left_and_respects_dnd() {
    let mut rt = synthetic_rt();
    rt.enqueue(Msg::NotificationEvent(Box::new(
        NotificationEvent::Received(Box::new(sample_notif(1, "Build Done", "All tests passed"))),
    )));
    rt.step().unwrap();

    assert_eq!(rt.state().notifications.total_count, 1);
    let initial_x = rt
        .state()
        .notifications
        .active_ticker
        .as_ref()
        .expect("ticker active")
        .offset_x;

    rt.enqueue(Msg::NotifTickerTick);
    rt.step().unwrap();
    let next_x = rt
        .state()
        .notifications
        .active_ticker
        .as_ref()
        .expect("ticker still active")
        .offset_x;
    assert!(next_x < initial_x, "ticker must scroll from right to left");

    // Advance until ticker finishes.
    for _ in 0..250 {
        if rt.state().notifications.active_ticker.is_none() {
            break;
        }
        rt.enqueue(Msg::NotifTickerTick);
        rt.step().unwrap();
    }
    assert!(rt.state().notifications.active_ticker.is_none());

    // Right-click toggles DND ON and suppresses ticker for new notifications.
    rt.enqueue(Msg::NotifBarPointerDown(PointerEvent {
        position: Point::new(350.logical_px(), 12.logical_px()),
        local_position: Point::new(10.logical_px(), 12.logical_px()),
        button: Some(MouseButton::Right),
        modifiers: Modifiers::empty(),
    }));
    rt.step().unwrap();
    assert!(rt.state().notifications.dnd);

    rt.enqueue(Msg::NotificationEvent(Box::new(
        NotificationEvent::Received(Box::new(sample_notif(2, "Silent", "DND active"))),
    )));
    rt.step().unwrap();
    assert!(rt.state().notifications.active_ticker.is_none());
    assert_eq!(rt.state().notifications.total_count, 2);
}

#[test]
fn notification_center_opens_on_left_click_closes_on_outside_or_esc_and_bounds_ram() {
    let mut rt = synthetic_rt();
    for i in 1..=(MAX_RAM_NOTIFICATIONS as u32 + 10) {
        rt.enqueue(Msg::NotificationEvent(Box::new(
            NotificationEvent::Received(Box::new(sample_notif(
                i,
                &format!("Alert {i}"),
                "Notification body text",
            ))),
        )));
    }
    rt.step().unwrap();

    assert_eq!(
        rt.state().notifications.items.len(),
        MAX_RAM_NOTIFICATIONS,
        "in-RAM notification count must stay capped"
    );
    assert_eq!(
        rt.state().notifications.total_count,
        MAX_RAM_NOTIFICATIONS + 10
    );

    // Left-click bar pill opens overlay notification center and animates it open.
    rt.enqueue(Msg::NotifBarPointerDown(PointerEvent {
        position: Point::new(350.logical_px(), 12.logical_px()),
        local_position: Point::new(10.logical_px(), 12.logical_px()),
        button: Some(MouseButton::Left),
        modifiers: Modifiers::empty(),
    }));
    rt.step().unwrap();
    assert!(rt.state().notifications.center_target);
    for _ in 0..16 {
        rt.enqueue(Msg::NotifCenterAnimTick);
        rt.step().unwrap();
    }
    assert!((rt.state().notifications.center_progress - 1.0).abs() < 1e-4);
    assert!(
        rt.rendered_buffer("bar-notification-center").is_some(),
        "notification center overlay surface must render"
    );

    // Marquee tick advances for overflowing text.
    let prev_marquee = rt.state().notifications.marquee_tick;
    rt.enqueue(Msg::NotifCenterMarqueeTick);
    rt.step().unwrap();
    assert!(rt.state().notifications.marquee_tick > prev_marquee);

    // Left-click card invokes action and removes it from the notification center.
    let newest_id = MAX_RAM_NOTIFICATIONS as u32 + 10;
    rt.enqueue(Msg::NotifCenterCardPointerDown {
        id: newest_id,
        ev: PointerEvent {
            position: Point::new(100.logical_px(), 80.logical_px()),
            local_position: Point::new(80.logical_px(), 20.logical_px()),
            button: Some(MouseButton::Left),
            modifiers: Modifiers::empty(),
        },
    });
    rt.step().unwrap();
    assert_eq!(
        rt.state().notifications.total_count,
        MAX_RAM_NOTIFICATIONS + 9
    );

    // Right-click next card also removes it.
    let next_id = MAX_RAM_NOTIFICATIONS as u32 + 9;
    rt.enqueue(Msg::NotifCenterCardPointerDown {
        id: next_id,
        ev: PointerEvent {
            position: Point::new(100.logical_px(), 80.logical_px()),
            local_position: Point::new(80.logical_px(), 20.logical_px()),
            button: Some(MouseButton::Right),
            modifiers: Modifiers::empty(),
        },
    });
    rt.step().unwrap();
    assert_eq!(
        rt.state().notifications.total_count,
        MAX_RAM_NOTIFICATIONS + 8
    );

    // Clicking outside closes the notification center and animates it closed.
    rt.enqueue(Msg::CloseNotificationCenter);
    rt.step().unwrap();
    assert!(!rt.state().notifications.center_target);
    for _ in 0..16 {
        rt.enqueue(Msg::NotifCenterAnimTick);
        rt.step().unwrap();
    }
    assert!(rt.state().notifications.center_progress <= 1e-4);
    assert_eq!(rt.surface_count(), 1);
}

#[test]
fn notification_image_data_persists_across_restart_and_clear_all_removes_icon_files() {
    let mut rt = synthetic_rt();
    let mut notif = sample_notif(99, "Avatar", "With raw pixel icon");
    notif.image_data = Some(NotificationImageData {
        width: 2,
        height: 2,
        rowstride: 8,
        has_alpha: true,
        bits_per_sample: 8,
        channels: 4,
        data: vec![
            255, 0, 0, 255, 0, 255, 0, 255, // row 0
            0, 0, 255, 255, 255, 255, 255, 255, // row 1
        ],
    });
    rt.enqueue(Msg::NotificationEvent(Box::new(
        NotificationEvent::Received(Box::new(notif)),
    )));
    rt.step().unwrap();

    let cached_path = rt.state().notifications.items[0]
        .stored
        .cached_icon_path
        .clone()
        .expect("image_data must be cached to disk");
    assert!(std::path::Path::new(&cached_path).exists());

    // Clear all must remove the cached icon file / directory so zero junk remains.
    rt.enqueue(Msg::NotifCenterClearAll);
    rt.step().unwrap();
    assert_eq!(rt.state().notifications.total_count, 0);
    assert!(!std::path::Path::new(&cached_path).exists());
}
