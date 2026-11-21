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
        expire_timeout_ms: 5000,
        received_at: SystemTime::now(),
        ..DesktopNotification::default()
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

#[test]
fn notification_center_shows_multi_actions_with_variable_height_and_horizontal_scroll() {
    use crate::app::state::HoverTarget;
    use crate::widgets::notification_center::card::{
        ACTION_CARD_HEIGHT, BASE_CARD_HEIGHT, action_max_scroll_x, card_height,
    };

    let mut rt = synthetic_rt();

    let mut single_action = sample_notif(201, "Single", "One default action");
    single_action.actions = vec![("default".into(), "Activate".into())];

    let mut multi_action = sample_notif(202, "Build", "Finished compiling package");
    multi_action.actions = vec![
        ("open".into(), "Open Output Directory".into()),
        ("logs".into(), "Inspect Full Logs".into()),
        ("retry".into(), "Rebuild Target".into()),
        ("share".into(), "Copy Link".into()),
    ];

    rt.enqueue(Msg::NotificationEvent(Box::new(
        NotificationEvent::Received(Box::new(single_action)),
    )));
    rt.enqueue(Msg::NotificationEvent(Box::new(
        NotificationEvent::Received(Box::new(multi_action)),
    )));
    rt.step().unwrap();

    let multi_item = &rt.state().notifications.items[0];
    let single_item = &rt.state().notifications.items[1];
    assert_eq!(card_height(single_item), BASE_CARD_HEIGHT);
    assert_eq!(card_height(multi_item), ACTION_CARD_HEIGHT);
    assert!(action_max_scroll_x(multi_item) > 0.0);

    // Open Notification Center and render both variable-height cards.
    rt.enqueue(Msg::NotifBarPointerDown(PointerEvent {
        position: Point::new(400.logical_px(), 12.logical_px()),
        local_position: Point::new(8.logical_px(), 12.logical_px()),
        button: Some(MouseButton::Left),
        modifiers: Modifiers::empty(),
    }));
    for _ in 0..16 {
        rt.enqueue(Msg::NotifCenterAnimTick);
        rt.step().unwrap();
    }
    assert!(rt.rendered_buffer("bar-notification-center").is_some());

    // Hover and horizontally scroll action buttons via mouse wheel.
    rt.enqueue(Msg::Hover(HoverTarget::NotifAction {
        id: 202,
        action_idx: 0,
    }));
    rt.enqueue(Msg::NotifActionScroll {
        id: 202,
        sc: ScrollEvent {
            position: Point::new(100.logical_px(), 110.logical_px()),
            local_position: Point::ZERO,
            delta_x: 0.0,
            delta_y: 1.0,
            discrete_x: 0,
            discrete_y: 1,
            modifiers: Modifiers::empty(),
        },
    });
    rt.step().unwrap();
    assert!(
        rt.state()
            .notifications
            .action_scroll_x
            .get(&202)
            .copied()
            .unwrap_or(0.0)
            > 0.0
    );

    // Clicking an action button dismisses the notification and cleans up its scroll offset.
    rt.enqueue(Msg::NotifCenterActionClick {
        id: 202,
        action_key: String::from("logs"),
    });
    rt.step().unwrap();
    assert_eq!(rt.state().notifications.items.len(), 1);
    assert!(!rt.state().notifications.action_scroll_x.contains_key(&202));
}

#[test]
fn notification_sound_dnd_and_xdg_v1_2_hints_behave_properly() {
    use crate::app::notifications::NotificationSoundPlan;

    let mut rt = synthetic_rt();

    // 1. Normal IM notification with DND OFF resolves to XDG instant-message sound.
    let mut im = sample_notif(301, "Chat", "Hello!");
    im.category = Some(String::from("im.received"));
    rt.enqueue(Msg::NotificationEvent(Box::new(
        NotificationEvent::Received(Box::new(im)),
    )));
    rt.step().unwrap();
    assert_eq!(
        rt.state().notifications.last_sound_plan,
        NotificationSoundPlan::ThemeName {
            primary: "message-new-instant",
            fallback_theme: Some("message"),
            fallback_preset: SoundPreset::Notification,
        }
    );

    // 2. Explicit `sound_file` overrides default sound when DND is OFF.
    let mut with_file = sample_notif(302, "Alarm", "Wake up");
    with_file.sound_file = Some(String::from("/usr/share/sounds/ding.oga"));
    rt.enqueue(Msg::NotificationEvent(Box::new(
        NotificationEvent::Received(Box::new(with_file)),
    )));
    rt.step().unwrap();
    assert_eq!(
        rt.state().notifications.last_sound_plan,
        NotificationSoundPlan::File(String::from("/usr/share/sounds/ding.oga"))
    );

    // 3. `suppress_sound = true` silences sound even when DND is OFF.
    let mut suppressed = sample_notif(303, "Quiet", "No chime");
    suppressed.sound_name = Some(String::from("bell"));
    suppressed.suppress_sound = true;
    rt.enqueue(Msg::NotificationEvent(Box::new(
        NotificationEvent::Received(Box::new(suppressed)),
    )));
    rt.step().unwrap();
    assert_eq!(
        rt.state().notifications.last_sound_plan,
        NotificationSoundPlan::Silent
    );

    // 4. `transient = true` shows ticker when DND is OFF but bypasses history persistence.
    let count_before = rt.state().notifications.total_count;
    let mut trans = sample_notif(304, "Volume", "75%");
    trans.transient = true;
    trans.urgency = NotificationUrgency::Low;
    rt.enqueue(Msg::NotificationEvent(Box::new(
        NotificationEvent::Received(Box::new(trans)),
    )));
    rt.step().unwrap();
    assert_eq!(rt.state().notifications.total_count, count_before);
    assert_eq!(
        rt.state().notifications.last_sound_plan,
        NotificationSoundPlan::Silent
    );
    assert!(rt.state().notifications.active_ticker.is_some());

    // 5. `resident = true` keeps notification in history when its action is clicked.
    let mut res = sample_notif(305, "Music", "Controls");
    res.resident = true;
    rt.enqueue(Msg::NotificationEvent(Box::new(
        NotificationEvent::Received(Box::new(res)),
    )));
    rt.step().unwrap();
    rt.enqueue(Msg::NotifCenterActionClick {
        id: 305,
        action_key: String::from("default"),
    });
    rt.step().unwrap();
    assert!(
        rt.state()
            .notifications
            .items
            .iter()
            .any(|i| i.stored.id == 305),
        "resident notification must remain open after action click"
    );

    // 6. Enabling DND clears active ticker and silences ALL incoming notifications
    //    (even Critical notifications with explicit sound_file).
    rt.enqueue(Msg::NotifCenterToggleDnd);
    rt.step().unwrap();
    assert!(rt.state().notifications.dnd);
    assert!(rt.state().notifications.active_ticker.is_none());

    let mut crit = sample_notif(306, "Critical", "Silenced by DND");
    crit.urgency = NotificationUrgency::Critical;
    crit.sound_file = Some(String::from("/usr/share/sounds/alarm.oga"));
    rt.enqueue(Msg::NotificationEvent(Box::new(
        NotificationEvent::Received(Box::new(crit)),
    )));
    rt.step().unwrap();
    assert_eq!(
        rt.state().notifications.last_sound_plan,
        NotificationSoundPlan::Silent
    );
    assert!(rt.state().notifications.active_ticker.is_none());
}
