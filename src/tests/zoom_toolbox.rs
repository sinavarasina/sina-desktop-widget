//! Headless tests for:
//! - Quick Menu scrollable 2-slot action rail (`^v──` indicator) & expandable Toolbox strip
//! - Animated Screen Magnifier (Snapshot mode + Hyprland Live mode toggle)
//! - Native WayWi Color Picker (loupe sampling, HEX/RGB toggle, clipboard copy & cleanup)
//! - Native WayWi Screenshot (Area / Window / Full, PNG crop & encode, cleanup)
//! - Non-square notification icon clipping (zero overflow)

use std::sync::Arc;
use std::time::{Duration, Instant};

use waywi::prelude::*;

use crate::SinaDesktopWidget;
use crate::app::Msg;
use crate::app::color_picker::{ColorFormat, ColorPickerPhase};
use crate::app::ipc::{IpcAction, IpcTarget, parse_ipc_command};
use crate::app::quick_menu::QuickActionSlot;
use crate::app::screenshot::{ScreenshotMode, ScreenshotPhase, crop_and_encode_png};
use crate::app::toolbox::ToolboxTool;
use crate::app::zoom::{DEFAULT_ZOOM, MIN_ZOOM, ZoomMode, ZoomPhase};

use super::support::{settle_ui_anims, synthetic_rt};

fn synthetic_frame(width: u32, height: u32) -> ScreencopyFrame {
    let mut rgba = vec![0u8; (width as usize) * (height as usize) * 4];
    for y in 0..height {
        for x in 0..width {
            let off = ((y * width + x) as usize) * 4;
            rgba[off] = ((x * 255) / width.max(1)) as u8;
            rgba[off + 1] = ((y * 255) / height.max(1)) as u8;
            rgba[off + 2] = 0x99;
            rgba[off + 3] = 0xff;
        }
    }
    ScreencopyFrame {
        width,
        height,
        stride: width * 4,
        rgba,
    }
}

fn ptr_ev(x: f32, y: f32, button: Option<MouseButton>) -> PointerEvent {
    PointerEvent {
        position: Point::new(x.logical_px(), y.logical_px()),
        local_position: Point::new(x.logical_px(), y.logical_px()),
        button,
        modifiers: Modifiers::empty(),
    }
}

fn key_ev(key: Key, utf8: Option<&str>) -> KeyEvent {
    KeyEvent {
        key,
        modifiers: Modifiers::empty(),
        utf8: utf8.map(|s| SharedStr::from(s.to_owned())),
        repeat: false,
    }
}

fn advance_zoom_frames(
    rt: &mut HeadlessRuntime<SinaDesktopWidget>,
    frames: usize,
    dt_ms: u64,
) {
    let mut now = Instant::now();
    for _ in 0..frames {
        now += Duration::from_millis(dt_ms);
        rt.enqueue(Msg::AnimationFrame(now));
    }
    rt.step().unwrap();
}

#[test]
fn toolbox_zoom_colorpicker_and_screenshot_ipc_parse_correctly() {
    assert!(matches!(
        parse_ipc_command("toolbox"),
        Some(Msg::IpcControl {
            action: IpcAction::Toggle,
            target: IpcTarget::Toolbox,
        })
    ));
    assert!(matches!(
        parse_ipc_command("zoom"),
        Some(Msg::IpcControl {
            action: IpcAction::Toggle,
            target: IpcTarget::Zoom,
        })
    ));
    assert!(matches!(parse_ipc_command("zoom in"), Some(Msg::ZoomStep(1))));
    assert!(matches!(
        parse_ipc_command("zoom out"),
        Some(Msg::ZoomStep(-1))
    ));
    assert!(matches!(
        parse_ipc_command("zoom reset"),
        Some(Msg::ZoomReset)
    ));
    assert!(matches!(
        parse_ipc_command("zoom live"),
        Some(Msg::ZoomToggleLive)
    ));
    assert!(matches!(
        parse_ipc_command("colorpicker"),
        Some(Msg::IpcControl {
            action: IpcAction::Toggle,
            target: IpcTarget::ColorPicker,
        })
    ));
    assert!(matches!(
        parse_ipc_command("open picker"),
        Some(Msg::IpcControl {
            action: IpcAction::Open,
            target: IpcTarget::ColorPicker,
        })
    ));
    assert!(matches!(
        parse_ipc_command("screenshot"),
        Some(Msg::IpcControl {
            action: IpcAction::Toggle,
            target: IpcTarget::Screenshot(None),
        })
    ));
    assert!(matches!(
        parse_ipc_command("screenshot area"),
        Some(Msg::IpcControl {
            action: IpcAction::Open,
            target: IpcTarget::Screenshot(Some(ScreenshotMode::Area)),
        })
    ));
    assert!(matches!(
        parse_ipc_command("ss window"),
        Some(Msg::IpcControl {
            action: IpcAction::Open,
            target: IpcTarget::Screenshot(Some(ScreenshotMode::Window)),
        })
    ));
    assert!(matches!(
        parse_ipc_command("shot full"),
        Some(Msg::IpcControl {
            action: IpcAction::Open,
            target: IpcTarget::Screenshot(Some(ScreenshotMode::Full)),
        })
    ));
}

#[test]
fn quick_menu_scrollable_action_rail_reveals_toolbox_and_launches_tools() {
    let mut rt = synthetic_rt();

    // Open Quick Menu: default scroll=0 shows [Settings, Power], with Toolbox hidden below.
    rt.enqueue(Msg::ClockMenuClick);
    settle_ui_anims(&mut rt);
    assert!(rt.state().quick_menu.is_popup_open());
    assert_eq!(
        rt.state().quick_menu.visible_action_slots(),
        [QuickActionSlot::Settings, QuickActionSlot::Power]
    );
    assert!(!rt.state().quick_menu.has_actions_above());
    assert!(rt.state().quick_menu.has_actions_below());

    // Scroll down on action rail: reveals [Power, Toolbox], with Settings now hidden above.
    rt.enqueue(Msg::QuickMenuActionScroll(ScrollEvent {
        position: Point::new(20.logical_px(), 120.logical_px()),
        local_position: Point::new(20.logical_px(), 120.logical_px()),
        delta_x: 0.0,
        delta_y: 15.0,
        discrete_x: 0,
        discrete_y: 1,
        source: ScrollSource::Wheel,
        stop_x: false,
        stop_y: false,
        modifiers: Modifiers::empty(),
    }));
    rt.step().unwrap();
    assert_eq!(
        rt.state().quick_menu.visible_action_slots(),
        [QuickActionSlot::Power, QuickActionSlot::Toolbox]
    );
    assert!(rt.state().quick_menu.has_actions_above());
    assert!(!rt.state().quick_menu.has_actions_below());

    // Toggle Toolbox lower strip inside Quick Menu and render cleanly.
    rt.enqueue(Msg::QuickMenuToggleToolbox);
    rt.step().unwrap();
    assert!(rt.state().quick_menu.toolbox_open);
    let buf = rt
        .rendered_buffer("bar-quick-menu")
        .expect("bar-quick-menu renders with Toolbox strip");
    assert!(buf.as_chunks::<4>().0.iter().any(|px| px[3] > 0));

    // Launching Annotate opens Annotate cleanly.
    rt.enqueue(Msg::ToolboxLaunch(ToolboxTool::Annotate));
    settle_ui_anims(&mut rt);
    assert!(rt.state().annotate.is_active());
    assert!(!rt.state().quick_menu.is_popup_open());

    // IPC `toolbox` opens Quick Menu straight to Toolbox strip & closes Annotate.
    rt.enqueue(Msg::IpcControl {
        action: IpcAction::Open,
        target: IpcTarget::Toolbox,
    });
    settle_ui_anims(&mut rt);
    assert!(rt.state().quick_menu.is_popup_open());
    assert!(rt.state().quick_menu.toolbox_open);
    assert!(!rt.state().annotate.is_active());

    // Launching Zoom hides Quick Menu immediately (zero fade artifact in capture).
    rt.enqueue(Msg::ToolboxLaunch(ToolboxTool::Zoom));
    rt.step().unwrap();
    assert!(!rt.state().quick_menu.is_popup_visible());
    assert_eq!(rt.state().zoom.phase, ZoomPhase::Capturing);
}

#[test]
fn zoom_snapshot_deadzone_live_toggle_and_animated_exit_reclaim_memory() {
    let mut rt = synthetic_rt();

    // Opening directly on a headless runtime without Screencopy capability fails gracefully.
    rt.enqueue(Msg::IpcControl {
        action: IpcAction::Open,
        target: IpcTarget::Zoom,
    });
    rt.step().unwrap();
    assert_eq!(rt.state().zoom.phase, ZoomPhase::Closed);
    assert!(rt.state().zoom.last_error.is_some());

    // Launch via Quick Menu Toolbox to get settle delay -> inject synthetic screencopy frame.
    rt.enqueue(Msg::IpcControl {
        action: IpcAction::Open,
        target: IpcTarget::Toolbox,
    });
    settle_ui_anims(&mut rt);
    rt.enqueue(Msg::ToolboxLaunch(ToolboxTool::Zoom));
    rt.step().unwrap();
    assert_eq!(rt.state().zoom.phase, ZoomPhase::Capturing);
    let seq = rt.state().zoom.capture_seq;

    let sc = synthetic_frame(192, 108);
    rt.enqueue(Msg::ZoomCaptured {
        seq,
        frame: Arc::new(Ok(sc)),
    });
    rt.step().unwrap();

    assert_eq!(rt.state().zoom.mode, ZoomMode::Snapshot);
    assert_eq!(rt.state().zoom.phase, ZoomPhase::Active);
    assert!(rt.state().zoom.snapshot.is_some());
    assert!((rt.state().zoom.zoom - MIN_ZOOM).abs() < 1e-3);
    assert!((rt.state().zoom.target_zoom - DEFAULT_ZOOM).abs() < 1e-3);

    advance_zoom_frames(&mut rt, 60, 16);
    assert!((rt.state().zoom.zoom - DEFAULT_ZOOM).abs() < 0.05);
    let settled_cam_x = rt.state().zoom.cam_x;

    // Central deadzone stays still; right outer rim glides camera smoothly right.
    rt.enqueue(Msg::ZoomPointerMove(ptr_ev(960.0, 540.0, None)));
    rt.step().unwrap();
    assert!(rt.state().zoom.edge_pan_velocity().is_none());

    rt.enqueue(Msg::ZoomPointerMove(ptr_ev(1880.0, 540.0, None)));
    rt.step().unwrap();
    assert!(rt.state().zoom.edge_pan_velocity().is_some_and(|(vx, _)| vx > 0.0));
    advance_zoom_frames(&mut rt, 25, 16);
    assert!(rt.state().zoom.cam_x > settled_cam_x + 20.0);

    // Toggle into Live Hyprland mode (`L`): unloads snapshot immediately & keeps HUD surface mounted.
    rt.enqueue(Msg::ZoomKey(key_ev(Key::from_keysym(b'l' as u32), Some("l"))));
    rt.step().unwrap();
    assert_eq!(rt.state().zoom.mode, ZoomMode::Live);
    assert_eq!(rt.state().zoom.phase, ZoomPhase::Active);
    assert!(
        rt.state().zoom.snapshot.is_none(),
        "Live mode must immediately unload snapshot texture"
    );
    advance_zoom_frames(&mut rt, 40, 16);
    let live_buf = rt
        .rendered_buffer("screen-zoom")
        .expect("Live HUD pill renders");
    assert!(live_buf.as_chunks::<4>().0.iter().any(|px| px[3] > 0));

    // Close Live zoom -> smoothly animates back to 1.0x and unmounts surface cleanly.
    rt.enqueue(Msg::CloseZoom);
    rt.step().unwrap();
    assert_eq!(rt.state().zoom.phase, ZoomPhase::Closing);
    advance_zoom_frames(&mut rt, 60, 16);
    assert_eq!(rt.state().zoom.phase, ZoomPhase::Closed);
    assert!(rt.rendered_buffer("screen-zoom").is_none());
}

#[test]
fn color_picker_samples_toggles_format_copies_and_frees_memory() {
    let mut rt = synthetic_rt();

    // Launch via Toolbox so settle delay keeps state in Capturing for synthetic frame delivery.
    rt.enqueue(Msg::IpcControl {
        action: IpcAction::Open,
        target: IpcTarget::Toolbox,
    });
    settle_ui_anims(&mut rt);
    rt.enqueue(Msg::ToolboxLaunch(ToolboxTool::ColorPicker));
    rt.step().unwrap();
    assert_eq!(rt.state().color_picker.phase, ColorPickerPhase::Capturing);
    let seq = rt.state().color_picker.capture_seq;

    let mut sc = synthetic_frame(192, 108);
    // Paint pixel (96, 54) exact Catppuccin Mauve (#cba6f7).
    let off = ((54 * 192 + 96) as usize) * 4;
    sc.rgba[off..off + 4].copy_from_slice(&[0xcb, 0xa6, 0xf7, 0xff]);

    rt.enqueue(Msg::ColorPickerCaptured {
        seq,
        frame: Arc::new(Ok(sc)),
    });
    rt.step().unwrap();
    assert_eq!(rt.state().color_picker.phase, ColorPickerPhase::Active);

    // Move cursor to (965, 545) -> maps to pixel (96, 54).
    rt.enqueue(Msg::ColorPickerPointerMove(ptr_ev(965.0, 545.0, None)));
    rt.step().unwrap();
    assert_eq!(rt.state().color_picker.sampled_rgba(), [0xcb, 0xa6, 0xf7, 0xff]);
    assert_eq!(rt.state().color_picker.formatted_current(), "#cba6f7");

    let rendered = rt
        .rendered_buffer("screen-colorpicker")
        .expect("screen-colorpicker renders");
    assert!(rendered.as_chunks::<4>().0.iter().any(|px| px[3] > 0));

    // Right-click toggles HEX -> RGB.
    rt.enqueue(Msg::ColorPickerPointerDown(ptr_ev(
        965.0,
        545.0,
        Some(MouseButton::Right),
    )));
    rt.step().unwrap();
    assert_eq!(rt.state().color_picker.format, ColorFormat::Rgb);
    assert_eq!(
        rt.state().color_picker.formatted_current(),
        "rgb(203, 166, 247)"
    );

    // Left-click commits copy, unmounts overlay, and releases snapshot + frame buffer.
    rt.enqueue(Msg::ColorPickerPointerDown(ptr_ev(
        965.0,
        545.0,
        Some(MouseButton::Left),
    )));
    rt.step().unwrap();
    assert_eq!(rt.state().color_picker.phase, ColorPickerPhase::Closed);
    assert_eq!(
        rt.state().color_picker.last_copied.as_deref(),
        Some("rgb(203, 166, 247)")
    );
    assert!(rt.state().color_picker.snapshot.is_none());
    assert!(rt.state().color_picker.frame.is_none());
    assert!(rt.rendered_buffer("screen-colorpicker").is_none());
}

#[test]
fn screenshot_area_selection_encodes_valid_png_and_frees_memory() {
    let sc = synthetic_frame(320, 180);
    let (png_bytes, pw, ph) =
        crop_and_encode_png(&sc, (192.0, 108.0, 960.0, 540.0), 1920.0, 1080.0)
            .expect("PNG crop & encode succeeds");
    assert_eq!((pw, ph), (160, 90));
    assert_eq!(&png_bytes[..8], b"\x89PNG\r\n\x1a\n");

    let mut rt = synthetic_rt();
    rt.enqueue(Msg::IpcControl {
        action: IpcAction::Open,
        target: IpcTarget::Toolbox,
    });
    settle_ui_anims(&mut rt);
    rt.enqueue(Msg::ToolboxLaunch(ToolboxTool::Screenshot));
    rt.step().unwrap();
    assert_eq!(rt.state().screenshot.phase, ScreenshotPhase::Capturing);
    let seq = rt.state().screenshot.capture_seq;

    rt.enqueue(Msg::ScreenshotCaptured {
        seq,
        frame: Arc::new(Ok(sc)),
    });
    rt.step().unwrap();
    assert_eq!(rt.state().screenshot.phase, ScreenshotPhase::Active);

    // Disable disk save during test so we don't pollute ~/Pictures/Screenshots.
    rt.enqueue(Msg::ScreenshotToggleSave);
    rt.step().unwrap();
    assert!(!rt.state().screenshot.save_to_file);
    assert!(rt.state().screenshot.copy_to_clipboard);

    // Drag an Area selection & render scrim + badge.
    rt.enqueue(Msg::ScreenshotPointerDown(ptr_ev(
        200.0,
        150.0,
        Some(MouseButton::Left),
    )));
    rt.enqueue(Msg::ScreenshotPointerMove(ptr_ev(700.0, 450.0, None)));
    rt.step().unwrap();
    assert_eq!(
        rt.state().screenshot.active_selection_rect(),
        Some((200.0, 150.0, 500.0, 300.0))
    );
    let rendered = rt
        .rendered_buffer("screen-screenshot")
        .expect("screen-screenshot renders");
    assert!(rendered.as_chunks::<4>().0.iter().any(|px| px[3] > 0));

    // Releasing mouse commits screenshot, closes overlay, and frees memory.
    rt.enqueue(Msg::ScreenshotPointerUp(ptr_ev(
        700.0,
        450.0,
        Some(MouseButton::Left),
    )));
    rt.step().unwrap();
    assert_eq!(rt.state().screenshot.phase, ScreenshotPhase::Closed);
    assert!(rt.state().screenshot.snapshot.is_none());
    assert!(rt.state().screenshot.frame.is_none());
    assert!(rt.rendered_buffer("screen-screenshot").is_none());
}

#[test]
fn screen_tools_triggered_via_ipc_keep_other_widget_popups_open() {
    let mut rt = synthetic_rt();

    // Open Quick Menu popup first.
    rt.enqueue(Msg::ClockMenuClick);
    settle_ui_anims(&mut rt);
    assert!(rt.state().quick_menu.is_popup_open());

    // Triggering Annotate via IPC keeps Quick Menu open underneath.
    rt.enqueue(Msg::IpcControl {
        action: IpcAction::Open,
        target: IpcTarget::Annotate,
    });
    settle_ui_anims(&mut rt);
    assert!(rt.state().annotate.is_active());
    assert!(
        rt.state().quick_menu.is_popup_open(),
        "IPC annotate must not close open Quick Menu"
    );

    // Triggering Screenshot via IPC closes Annotate (competing screen tool) but keeps Quick Menu open!
    rt.enqueue(Msg::IpcControl {
        action: IpcAction::Open,
        target: IpcTarget::Screenshot(Some(ScreenshotMode::Area)),
    });
    rt.step().unwrap();
    assert!(!rt.state().annotate.is_active());
    assert_eq!(rt.state().screenshot.phase, ScreenshotPhase::Capturing);
    assert!(
        rt.state().quick_menu.is_popup_open(),
        "IPC screenshot must keep Quick Menu open so widgets can be screenshotted"
    );

    // Triggering Color Picker via IPC also keeps Quick Menu open.
    rt.enqueue(Msg::IpcControl {
        action: IpcAction::Open,
        target: IpcTarget::ColorPicker,
    });
    rt.step().unwrap();
    assert!(
        rt.state().quick_menu.is_popup_open(),
        "IPC colorpicker must keep Quick Menu open"
    );

    // Triggering Live Zoom via IPC also keeps Quick Menu open.
    rt.enqueue(Msg::ZoomToggleLive);
    rt.step().unwrap();
    assert!(rt.state().zoom.is_live());
    assert!(
        rt.state().quick_menu.is_popup_open(),
        "IPC zoom live must keep Quick Menu open"
    );
}

#[test]
fn notifications_with_duplicate_dbus_ids_get_distinct_ui_uids_and_single_dismiss() {
    let mut rt = synthetic_rt();

    // Simulate 3 notifications arriving with the exact same D-Bus ID `1` (e.g. across restarts).
    for title in ["First", "Second", "Third"] {
        rt.enqueue(Msg::NotificationEvent(Box::new(
            NotificationEvent::Received(Box::new(DesktopNotification {
                id: 1,
                app_name: String::from("Firefox"),
                summary: String::from(title),
                body: String::from("hello"),
                ..DesktopNotification::default()
            })),
        )));
    }
    rt.step().unwrap();

    assert_eq!(rt.state().notifications.items.len(), 3);
    let uids: Vec<u32> = rt.state().notifications.items.iter().map(|i| i.uid).collect();
    assert_ne!(uids[0], uids[1]);
    assert_ne!(uids[1], uids[2]);
    assert_ne!(uids[0], uids[2]);

    // Hovering one card matches only that single card's uid, and dismissing removes only 1 card.
    rt.enqueue(Msg::NotifCenterCardPointerDown {
        id: uids[1],
        ev: ptr_ev(50.0, 80.0, Some(MouseButton::Right)),
    });
    rt.step().unwrap();
    assert_eq!(rt.state().notifications.items.len(), 2);
}

#[test]
fn mpris_bar_measures_exact_overflow_and_spins_vinyl_only_while_playing() {
    let mut rt = synthetic_rt();
    rt.enqueue(Msg::MprisEvent(Box::new(MprisEvent::PlayersUpdated(vec![
        MprisSnapshot {
            service_name: String::from("org.mpris.MediaPlayer2.mpd"),
            identity: Some(String::from("MPD")),
            status: MprisPlaybackStatus::Playing,
            artist: Some(String::from("Walkure")),
            album: None,
            title: Some(String::from(
                "いけないボーダーライン ~ Album Edition Walkure Attack!",
            )),
            ..MprisSnapshot::default()
        },
    ]))));
    rt.step().unwrap();

    // Short artist fits cleanly inside 120px inner box -> must NOT scroll.
    assert!(
        !rt.state().mpris.top_overflows(),
        "short artist 'Walkure' ({:.1}px) must not scroll",
        rt.state().mpris.bar_top_width_px
    );
    // Long mixed CJK + ASCII title exceeds 120px -> MUST scroll.
    assert!(
        rt.state().mpris.bot_overflows(),
        "long mixed CJK/ASCII title ({:.1}px) must overflow and scroll",
        rt.state().mpris.bar_bot_width_px
    );

    // Advance past the initial 1.6s hold -> bottom line scrolls smoothly while top stays at 0.
    advance_zoom_frames(&mut rt, 150, 20);
    assert_eq!(rt.state().mpris.bar_top_scroll_px, 0.0);
    assert!(rt.state().mpris.bar_bot_scroll_px > 10.0);

    // Render bar cleanly with the new MPRIS marquee.
    let bar = rt.rendered_buffer("bar").expect("bar renders");
    assert!(bar.as_chunks::<4>().0.iter().any(|px| px[3] > 0));

    // Open MPRIS popup: long Japanese title overflows 222px and scrolls smoothly with static string.
    assert!(rt.state().mpris.popup_title_overflows());
    assert!(!rt.state().mpris.popup_sub_overflows());
    rt.enqueue(Msg::IpcControl {
        action: IpcAction::Open,
        target: IpcTarget::Mpris,
    });
    advance_zoom_frames(&mut rt, 150, 20);
    assert!(rt.state().mpris.popup_title_scroll_px > 10.0);
    assert_eq!(rt.state().mpris.popup_sub_scroll_px, 0.0);
    let popup = rt
        .rendered_buffer("bar-mpris-popup")
        .expect("bar-mpris-popup renders");
    assert!(popup.as_chunks::<4>().0.iter().any(|px| px[3] > 0));
}

#[test]
fn status_pill_privacy_clipboard_caffeine_and_active_tools_work_end_to_end() {
    assert!(matches!(
        parse_ipc_command("clipboard"),
        Some(Msg::IpcControl {
            action: IpcAction::Toggle,
            target: IpcTarget::Clipboard,
        })
    ));
    assert!(matches!(
        parse_ipc_command("open awake"),
        Some(Msg::IpcControl {
            action: IpcAction::Open,
            target: IpcTarget::Caffeine,
        })
    ));

    let mut rt = synthetic_rt();

    // Push text + hex color clipboard snapshots (dedup & swatch detection).
    rt.enqueue(Msg::ClipboardEvent(Box::new(ClipboardEvent::Changed(
        ClipboardSnapshot {
            selection: ClipboardSelection::Clipboard,
            mime_types: vec![String::from("text/plain;charset=utf-8")],
            text: Some(String::from("hello waywi")),
            bytes: b"hello waywi".to_vec(),
            primary_mime: Some(String::from("text/plain;charset=utf-8")),
            cleared: false,
        },
    ))));
    rt.enqueue(Msg::ClipboardEvent(Box::new(ClipboardEvent::Changed(
        ClipboardSnapshot {
            selection: ClipboardSelection::Clipboard,
            mime_types: vec![String::from("text/plain;charset=utf-8")],
            text: Some(String::from("#cba6f7")),
            bytes: b"#cba6f7".to_vec(),
            primary_mime: Some(String::from("text/plain;charset=utf-8")),
            cleared: false,
        },
    ))));
    rt.step().unwrap();

    assert_eq!(rt.state().clipboard.entries.len(), 2);
    assert_eq!(
        rt.state().clipboard.latest_hex_color(),
        Some(Color::rgb(0xcb, 0xa6, 0xf7))
    );

    // Pin top entry, then clear unpinned -> keeps only pinned entry.
    let hex_id = rt.state().clipboard.entries[0].id;
    rt.enqueue(Msg::ClipboardPinEntry(hex_id));
    rt.enqueue(Msg::ClipboardClearUnpinned);
    rt.step().unwrap();
    assert_eq!(rt.state().clipboard.entries.len(), 1);
    assert!(rt.state().clipboard.entries[0].pinned);

    // Activate privacy sensors + Caffeine + Live Zoom -> bar & clipboard popup render cleanly.
    rt.enqueue(Msg::PrivacySampled(PrivacySnapshot {
        active: vec![
            PrivacyAccessEntry {
                kind: PrivacyAccessKind::Camera,
                source: PrivacyAccessSource::PipeWire,
                app_name: String::from("Firefox"),
                node_name: Some(String::from("C270")),
                pid: Some(1234),
                device_path: Some(String::from("/dev/video0")),
            },
            PrivacyAccessEntry {
                kind: PrivacyAccessKind::Microphone,
                source: PrivacyAccessSource::PipeWire,
                app_name: String::from("Discord"),
                node_name: None,
                pid: Some(2345),
                device_path: None,
            },
        ],
        camera_in_use: true,
        microphone_in_use: true,
        audio_capture_in_use: false,
        screen_share_in_use: false,
        location_in_use: false,
    }));
    rt.enqueue(Msg::ToggleCaffeine);
    rt.enqueue(Msg::ZoomToggleLive);
    rt.enqueue(Msg::IpcControl {
        action: IpcAction::Open,
        target: IpcTarget::Clipboard,
    });
    settle_ui_anims(&mut rt);

    assert!(rt.state().caffeine_active);
    assert!(rt.state().zoom.is_live());
    assert!(rt.state().clipboard.is_popup_open());
    let tip = crate::modules::status_pill::privacy_tooltip(rt.state());
    assert!(tip.contains("Cam: Firefox"));
    assert!(tip.contains("Mic: Discord"));

    let bar_buf = rt.rendered_buffer("bar").expect("bar renders with status pill");
    assert!(bar_buf.as_chunks::<4>().0.iter().any(|px| px[3] > 0));
    let clip_buf = rt
        .rendered_buffer("bar-clipboard-popup")
        .expect("clipboard popup renders");
    assert!(clip_buf.as_chunks::<4>().0.iter().any(|px| px[3] > 0));
}

#[test]
fn screenshot_preview_lifecycle_delete_and_clipboard_open_detection() {
    let sc = synthetic_frame(320, 180);
    let mut rt = synthetic_rt();

    // Launch screenshot -> capture -> disable disk save -> commit area selection.
    rt.enqueue(Msg::IpcControl {
        action: IpcAction::Open,
        target: IpcTarget::Toolbox,
    });
    settle_ui_anims(&mut rt);
    rt.enqueue(Msg::ToolboxLaunch(ToolboxTool::Screenshot));
    rt.step().unwrap();
    let seq = rt.state().screenshot.capture_seq;
    rt.enqueue(Msg::ScreenshotCaptured {
        seq,
        frame: Arc::new(Ok(sc.clone())),
    });
    rt.step().unwrap();
    rt.enqueue(Msg::ScreenshotToggleSave);
    rt.enqueue(Msg::ScreenshotPointerDown(ptr_ev(
        200.0,
        150.0,
        Some(MouseButton::Left),
    )));
    rt.enqueue(Msg::ScreenshotPointerMove(ptr_ev(700.0, 450.0, None)));
    rt.enqueue(Msg::ScreenshotPointerUp(ptr_ev(
        700.0,
        450.0,
        Some(MouseButton::Left),
    )));
    settle_ui_anims(&mut rt);

    // Preview card should be mounted and rendered non-empty.
    assert!(rt.state().screenshot.has_visible_preview());
    let preview_seq = rt
        .state()
        .screenshot
        .active_preview_timeout_seq()
        .expect("preview auto-dismiss timer active");
    let preview_buf = rt
        .rendered_buffer("screen-screenshot-preview")
        .expect("screen-screenshot-preview renders");
    assert!(preview_buf.as_chunks::<4>().0.iter().any(|px| px[3] > 0));

    // Hovering pauses the auto-dismiss timer; unhovering resumes it.
    rt.enqueue(Msg::ScreenshotPreviewHover(true));
    rt.step().unwrap();
    assert!(rt.state().screenshot.active_preview_timeout_seq().is_none());
    rt.enqueue(Msg::ScreenshotPreviewHover(false));
    rt.step().unwrap();
    assert_eq!(
        rt.state().screenshot.active_preview_timeout_seq(),
        Some(preview_seq)
    );

    // Timeout dismisses and frees the preview after exit animation settles.
    rt.enqueue(Msg::ScreenshotPreviewTimeout(preview_seq));
    settle_ui_anims(&mut rt);
    assert!(!rt.state().screenshot.has_visible_preview());
    assert!(rt.state().screenshot.preview.is_none());
    assert!(rt.rendered_buffer("screen-screenshot-preview").is_none());

    // Delete action removes a saved file on disk and clears last_saved_path.
    let tmp_file = std::env::temp_dir().join(format!(
        "sina-shot-delete-test-{}.png",
        std::process::id()
    ));
    let (png_bytes, _pw, _ph) =
        crop_and_encode_png(&sc, (0.0, 0.0, 960.0, 540.0), 1920.0, 1080.0).unwrap();
    std::fs::write(&tmp_file, &png_bytes).expect("write temp screenshot");
    assert!(tmp_file.exists());
    let mut shot_state = crate::app::screenshot::ScreenshotState::default();
    shot_state.last_saved_path = Some(tmp_file.clone());
    assert!(shot_state.delete_saved_file(Some(tmp_file.clone())));
    assert!(!tmp_file.exists(), "Delete must remove saved file");
    assert!(shot_state.last_saved_path.is_none());

    // End-to-end ScreenshotPreviewDelete message dismisses an active preview cleanly.
    rt.enqueue(Msg::IpcControl {
        action: IpcAction::Open,
        target: IpcTarget::Toolbox,
    });
    settle_ui_anims(&mut rt);
    rt.enqueue(Msg::ToolboxLaunch(ToolboxTool::Screenshot));
    rt.step().unwrap();
    let seq2 = rt.state().screenshot.capture_seq;
    rt.enqueue(Msg::ScreenshotCaptured {
        seq: seq2,
        frame: Arc::new(Ok(sc)),
    });
    rt.enqueue(Msg::ScreenshotSetMode(ScreenshotMode::Full));
    settle_ui_anims(&mut rt);
    assert!(rt.state().screenshot.has_visible_preview());
    rt.enqueue(Msg::ScreenshotPreviewDelete);
    settle_ui_anims(&mut rt);
    assert!(!rt.state().screenshot.has_visible_preview());
    assert!(rt.state().screenshot.preview.is_none());

    // Clipboard `can_open()` detection for plain text, URL, and image entries.
    rt.enqueue(Msg::ClipboardEvent(Box::new(ClipboardEvent::Changed(
        ClipboardSnapshot {
            selection: ClipboardSelection::Clipboard,
            mime_types: vec![String::from("text/plain;charset=utf-8")],
            text: Some(String::from("plain text only")),
            bytes: b"plain text only".to_vec(),
            primary_mime: Some(String::from("text/plain;charset=utf-8")),
            cleared: false,
        },
    ))));
    rt.enqueue(Msg::ClipboardEvent(Box::new(ClipboardEvent::Changed(
        ClipboardSnapshot {
            selection: ClipboardSelection::Clipboard,
            mime_types: vec![String::from("text/plain;charset=utf-8")],
            text: Some(String::from("https://wayland.freedesktop.org")),
            bytes: b"https://wayland.freedesktop.org".to_vec(),
            primary_mime: Some(String::from("text/plain;charset=utf-8")),
            cleared: false,
        },
    ))));
    rt.enqueue(Msg::ClipboardEvent(Box::new(ClipboardEvent::Changed(
        ClipboardSnapshot {
            selection: ClipboardSelection::Clipboard,
            mime_types: vec![String::from("image/png")],
            text: None,
            bytes: png_bytes,
            primary_mime: Some(String::from("image/png")),
            cleared: false,
        },
    ))));
    rt.enqueue(Msg::IpcControl {
        action: IpcAction::Open,
        target: IpcTarget::Clipboard,
    });
    settle_ui_anims(&mut rt);

    let entries = &rt.state().clipboard.entries;
    assert_eq!(entries.len(), 3);
    assert!(entries[0].can_open(), "image entry must be openable");
    assert!(
        matches!(
            entries[0].kind,
            crate::app::clipboard::ClipboardEntryKind::Image { thumb: Some(_), .. }
        ),
        "opening clipboard popup must hydrate image thumbnails"
    );
    assert!(entries[1].can_open(), "https URL entry must be openable");
    assert!(!entries[2].can_open(), "plain text must not show open");
    let clip_buf = rt
        .rendered_buffer("bar-clipboard-popup")
        .expect("clipboard popup renders with open buttons");
    assert!(clip_buf.as_chunks::<4>().0.iter().any(|px| px[3] > 0));

    // Closing clipboard popup evicts decoded thumbnail assets; re-opening re-hydrates them cleanly
    // even after a screenshot preview sharing the exact same PNG bytes has been dismissed.
    rt.enqueue(Msg::CloseClipboardPopup);
    settle_ui_anims(&mut rt);
    assert!(
        matches!(
            rt.state().clipboard.entries[0].kind,
            crate::app::clipboard::ClipboardEntryKind::Image { thumb: None, .. }
        ),
        "closing clipboard popup must evict idle image thumbnails"
    );
    rt.enqueue(Msg::IpcControl {
        action: IpcAction::Open,
        target: IpcTarget::Clipboard,
    });
    settle_ui_anims(&mut rt);
    assert!(
        matches!(
            rt.state().clipboard.entries[0].kind,
            crate::app::clipboard::ClipboardEntryKind::Image { thumb: Some(_), .. }
        ),
        "re-opening clipboard popup must re-hydrate thumbnails from retained bytes"
    );
}

#[test]
fn telemetry_graph_popup_opens_switches_tabs_and_renders_all_four_metrics() {
    use crate::app::telemetry_graph::TelemetryGraphKind;
    use waywi::system::{
        ByteSize, CpuSnapshot, GpuDeviceSnapshot, GpuSnapshot, GpuVendor, LoadAverage,
        MemorySnapshot, NetInterfaceKind, NetInterfaceSnapshot, NetLinkState, NetworkSnapshot,
    };

    let mut rt = synthetic_rt();

    // Feed multiple samples for CPU, Memory, GPU, and Network history buffers.
    for i in 1..=4 {
        rt.enqueue(Msg::CpuSampled(CpuSnapshot {
            total_percent: (i * 15) as f32,
            per_core_percent: vec![(i * 10) as f32, (i * 20) as f32, 35.0, 50.0],
            per_core_freq_khz: vec![2_400_000, 3_100_000, 2_800_000, 3_600_000],
            load_avg: LoadAverage {
                one: 0.65,
                five: 0.48,
                fifteen: 0.32,
                running_tasks: 2,
                total_tasks: 420,
            },
            logical_cores: 4,
            timestamp: std::time::Instant::now(),
        }));
        rt.enqueue(Msg::MemorySampled(MemorySnapshot {
            total: ByteSize::from_bytes(16 * 1024 * 1024 * 1024),
            used: ByteSize::from_bytes((4 + i) * 1024 * 1024 * 1024),
            available: ByteSize::from_bytes((12 - i) * 1024 * 1024 * 1024),
            swap_total: ByteSize::from_bytes(8 * 1024 * 1024 * 1024),
            swap_used: ByteSize::from_bytes(512 * 1024 * 1024),
            swap_available: ByteSize::from_bytes(7_680 * 1024 * 1024),
            psi: None,
        }));
        rt.enqueue(Msg::GpuSampled(GpuSnapshot {
            gpus: vec![GpuDeviceSnapshot {
                card_name: String::from("card0"),
                vendor: GpuVendor::Amd,
                pci_vendor_id: None,
                pci_device_id: None,
                driver: Some(String::from("amdgpu")),
                model_name: Some(String::from("Radeon RX")),
                utilization_percent: Some((i * 18) as f32),
                vram_used: Some(ByteSize::from_bytes(2 * 1024 * 1024 * 1024)),
                vram_total: Some(ByteSize::from_bytes(8 * 1024 * 1024 * 1024)),
                gtt_used: None,
                gtt_total: None,
                core_clock_mhz: Some(1200 + i as u32 * 100),
                memory_clock_mhz: Some(1750),
                temperature_celsius: Some(52.0),
                hotspot_celsius: Some(61.0),
                power_watts: Some(42.5),
                fan_rpm: None,
            }],
            timestamp: std::time::Instant::now(),
        }));
        rt.enqueue(Msg::NetworkSampled(NetworkSnapshot {
            interfaces: vec![NetInterfaceSnapshot {
                name: String::from("wlan0"),
                kind: NetInterfaceKind::Wifi,
                link_state: NetLinkState::Up,
                carrier: Some(true),
                link_speed_mbps: Some(866),
                ipv4: Some(String::from("192.168.1.42")),
                ipv6: None,
                gateway: Some(String::from("192.168.1.1")),
                wireless_signal_dbm: Some(-56),
                wireless_freq_mhz: Some(5200),
                rx_total: ByteSize::from_bytes(1024 * 1024 * i),
                tx_total: ByteSize::from_bytes(256 * 1024 * i),
                rx_bytes_per_sec: (i as f64) * 512.0 * 1024.0,
                tx_bytes_per_sec: (i as f64) * 96.0 * 1024.0,
                rx_packets_per_sec: 400.0,
                tx_packets_per_sec: 120.0,
            }],
            total_rx_bytes_per_sec: (i as f64) * 512.0 * 1024.0,
            total_tx_bytes_per_sec: (i as f64) * 96.0 * 1024.0,
            timestamp: std::time::Instant::now(),
        }));
    }
    rt.step().unwrap();

    assert!(rt.state().telemetry_graph.cpu_usage.len() >= 4);
    assert!(rt.state().telemetry_graph.mem_usage.len() >= 4);
    assert!(rt.state().telemetry_graph.gpu_usage.len() >= 4);
    assert!(rt.state().telemetry_graph.net_rx_bps.len() >= 4);

    // Left-click CPU opens the popup on CPU and renders cleanly.
    rt.enqueue(Msg::TelemetryBarPointerDown {
        kind: TelemetryGraphKind::Cpu,
        ev: ptr_ev(1500.0, 12.0, Some(MouseButton::Left)),
    });
    settle_ui_anims(&mut rt);
    assert!(rt.state().telemetry_graph.is_popup_open());
    assert_eq!(rt.state().telemetry_graph.active, TelemetryGraphKind::Cpu);
    let buf = rt
        .rendered_buffer("bar-telemetry-popup")
        .expect("CPU telemetry graph popup renders");
    assert!(buf.as_chunks::<4>().0.iter().any(|px| px[3] > 0));

    // Switch through Memory, GPU, and Network tabs cleanly.
    for kind in [
        TelemetryGraphKind::Memory,
        TelemetryGraphKind::Gpu,
        TelemetryGraphKind::Network,
    ] {
        rt.enqueue(Msg::TelemetrySelectKind(kind));
        rt.step().unwrap();
        assert_eq!(rt.state().telemetry_graph.active, kind);
        let tab_buf = rt
            .rendered_buffer("bar-telemetry-popup")
            .expect("telemetry graph tab renders");
        assert!(tab_buf.as_chunks::<4>().0.iter().any(|px| px[3] > 0));
    }

    // Left-clicking the active metric toggles the popup closed cleanly.
    rt.enqueue(Msg::TelemetryBarPointerDown {
        kind: TelemetryGraphKind::Network,
        ev: ptr_ev(1600.0, 12.0, Some(MouseButton::Left)),
    });
    settle_ui_anims(&mut rt);
    assert!(!rt.state().telemetry_graph.is_popup_visible());
    assert!(rt.rendered_buffer("bar-telemetry-popup").is_none());
}

#[test]
fn responsive_viewport_resizes_bar_overflows_metrics_and_renders_at_2x_and_narrow() {
    use crate::app::layout::LayoutTier;
    use waywi::app::HeadlessRuntime;
    use waywi::capabilities::{Capabilities, Capability};
    use waywi::geometry::{LogicalPx, LogicalSize, Point, ScaleFactor};

    let make_rt = |w: f32, h: f32, scale_int: i32| {
        let mut caps = Capabilities::new();
        caps.set(Capability::LayerShell, true);
        let outputs = vec![OutputInfo {
            id: OutputId(1),
            name: Some(String::from("DP-1")),
            description: None,
            make: String::new(),
            model: String::new(),
            logical_position: Point::ZERO,
            logical_size: LogicalSize::new(LogicalPx(w), LogicalPx(h)),
            physical_size_mm: (530, 300),
            scale_factor: ScaleFactor::from_integer(scale_int),
            transform: OutputTransform::Normal,
            refresh_mhz: 180_000,
        }];
        HeadlessRuntime::<crate::SinaDesktopWidget>::new(caps, outputs).expect("headless init")
    };

    // 1920x1080 @ scale=2 -> logical 960x540 (Tight tier).
    let mut rt_2x = make_rt(960.0, 540.0, 2);
    rt_2x.enqueue(Msg::ClockMenuClick);
    rt_2x.enqueue(Msg::QuickMenuToggleSettings);
    settle_ui_anims(&mut rt_2x);

    assert_eq!(rt_2x.state().layout().tier, LayoutTier::Tight);
    assert_eq!(
        rt_2x.surface_logical_size("bar"),
        Some(LogicalSize::new(LogicalPx(960.0), LogicalPx(24.0)))
    );
    assert_eq!(
        rt_2x.surface_logical_size("bar-quick-menu"),
        Some(LogicalSize::new(LogicalPx(960.0), LogicalPx(540.0)))
    );
    let bar_2x = rt_2x.rendered_buffer("bar").expect("bar renders at 960x24");
    assert!(bar_2x.as_chunks::<4>().0.iter().any(|px| px[3] > 0));
    let qm_2x = rt_2x
        .rendered_buffer("bar-quick-menu")
        .expect("quick menu renders at 960x540");
    assert!(qm_2x.as_chunks::<4>().0.iter().any(|px| px[3] > 0));

    // Narrow portrait/high-scale display -> logical 720x480 (Minimal tier + drawer overflow).
    let mut rt_narrow = make_rt(720.0, 480.0, 2);
    rt_narrow.step().unwrap();

    assert_eq!(rt_narrow.state().layout().tier, LayoutTier::Minimal);
    assert_eq!(
        rt_narrow.surface_logical_size("bar"),
        Some(LogicalSize::new(LogicalPx(720.0), LogicalPx(24.0)))
    );
    let (bar_ids, drawer_ids) = rt_narrow.state().effective_right_metrics();
    assert_eq!(bar_ids.len(), 1);
    assert!(drawer_ids.len() >= 5);
    let bar_narrow = rt_narrow
        .rendered_buffer("bar")
        .expect("bar renders at 720x24");
    assert!(bar_narrow.as_chunks::<4>().0.iter().any(|px| px[3] > 0));
}
