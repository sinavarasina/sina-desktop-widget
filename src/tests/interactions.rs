//! Pointer, scroll, hover, drawer animation, and tray menu interaction tests.

use std::time::{Duration, Instant};

use waywi::prelude::*;

use crate::app::Msg;
use crate::app::state::{AudioControlTarget, HoverTarget, TrayMenuPopup};
use crate::widgets::tray_menu::tray_menu_left_and_width;

use super::support::{
    make_tray_snap, maybe_dump_argb8888_ppm, populate_sample_metrics, synthetic_rt,
};

#[test]
fn workspace_click_switches_active_workspace() {
    let mut rt = synthetic_rt();
    rt.enqueue(Msg::TestSetWorkspaces(
        vec![
            WorkspaceInfo {
                id: WorkspaceId::new(1),
                name: String::from("1"),
                monitor: String::from("eDP-1"),
                windows: 1,
                focused: true,
            },
            WorkspaceInfo {
                id: WorkspaceId::new(2),
                name: String::from("2"),
                monitor: String::from("eDP-1"),
                windows: 1,
                focused: false,
            },
            WorkspaceInfo {
                id: WorkspaceId::new(3),
                name: String::from("3"),
                monitor: String::from("eDP-1"),
                windows: 0,
                focused: false,
            },
        ],
        1,
    ));
    rt.step().unwrap();

    // Inactive workspace pill starts directly after `sep-l-3` (~x=96..135). Scan to hit `ws-inactive` 2.
    let mut switched = false;
    for x in (90..150).step_by(4) {
        if rt.pointer_move_surface_point("bar", x as f32, 12.0) {
            rt.step().unwrap();
            if rt.state().hovered == HoverTarget::InactiveWs(2) {
                assert!(rt.click_surface_point("bar", x as f32, 12.0));
                rt.step().unwrap();
                switched = true;
                break;
            }
        }
    }
    assert!(switched, "pointer hit inactive workspace 2");
    assert_eq!(rt.state().active_ws_id, 2);
}

#[test]
fn audio_scroll_up_increases_and_down_decreases() {
    let mut rt = synthetic_rt();
    rt.enqueue(Msg::AudioUpdated(AudioSnapshot {
        linear: 0.50,
        db: None,
        muted: false,
        default_sink: None,
        devices: Vec::new(),
        ..AudioSnapshot::default()
    }));
    rt.step().unwrap();

    let scroll = |delta_y: f32, discrete_y: i32| {
        Msg::AudioScroll(ScrollEvent {
            position: Point::new(1650.logical_px(), 12.logical_px()),
            local_position: Point::new(10.logical_px(), 12.logical_px()),
            delta_x: 0.0,
            delta_y,
            discrete_x: 0,
            discrete_y,
            modifiers: Modifiers::empty(),
        })
    };

    // Wayland scroll UP -> delta_y < 0 / discrete_y < 0 => volume must INCREASE.
    rt.enqueue(scroll(-1.0, -1));
    rt.step().unwrap();
    let up_vol = rt.state().audio.as_ref().unwrap().linear;
    assert!((up_vol - 0.51).abs() < 1e-4, "expected 0.51, got {up_vol}");

    // Wayland scroll DOWN -> delta_y > 0 / discrete_y > 0 => volume must DECREASE.
    rt.enqueue(scroll(1.0, 1));
    rt.enqueue(scroll(1.0, 1));
    rt.step().unwrap();
    let down_vol = rt.state().audio.as_ref().unwrap().linear;
    assert!(
        (down_vol - 0.49).abs() < 1e-4,
        "expected 0.49, got {down_vol}"
    );

    // Middle click switches active target to Source (mic), and scroll adjusts source_linear.
    rt.enqueue(Msg::AudioPointerDown(PointerEvent {
        position: Point::new(1650.logical_px(), 12.logical_px()),
        local_position: Point::new(10.logical_px(), 12.logical_px()),
        button: Some(MouseButton::Middle),
        modifiers: Modifiers::empty(),
    }));
    rt.step().unwrap();
    assert_eq!(rt.state().audio_target, AudioControlTarget::Source);

    rt.enqueue(scroll(-1.0, -1));
    rt.step().unwrap();
    let src_vol = rt.state().audio.as_ref().unwrap().source_linear;
    assert!(
        (src_vol - 0.01).abs() < 1e-4,
        "expected 0.01, got {src_vol}"
    );
}

#[test]
fn clock_interactions_toggle_alt_format_calendar_mode_and_scroll_month() {
    let mut rt = synthetic_rt();
    assert!(!rt.state().clock_alt);
    assert!(rt.pointer_move_surface_point("bar", 1895.0, 12.0));
    rt.step().unwrap();
    assert_eq!(rt.state().hovered, HoverTarget::Clock);
    assert_eq!(rt.surface_count(), 2, "floating calendar tooltip opens");

    let month_size = rt
        .surface_logical_size("bar-tooltip")
        .expect("month calendar size");
    assert!(month_size.width.get() < 320.0 && month_size.height.get() < 260.0);
    if let Some(buf) = rt.rendered_buffer("bar-tooltip") {
        maybe_dump_argb8888_ppm(
            "calendar_month",
            month_size.width.get().round() as u32,
            month_size.height.get().round() as u32,
            buf,
        );
    }

    assert!(rt.pointer_down_surface_point("bar", 1895.0, 12.0, MouseButton::Left));
    rt.step().unwrap();
    assert!(rt.state().clock_alt);

    assert!(rt.pointer_down_surface_point("bar", 1895.0, 12.0, MouseButton::Right));
    rt.step().unwrap();
    assert!(rt.state().cal_year_mode);
    let year_size = rt
        .surface_logical_size("bar-tooltip")
        .expect("year calendar size");
    assert!(year_size.width.get() > 550.0 && year_size.height.get() > 450.0);
    if let Some(buf) = rt.rendered_buffer("bar-tooltip") {
        maybe_dump_argb8888_ppm(
            "calendar_year",
            year_size.width.get().round() as u32,
            year_size.height.get().round() as u32,
            buf,
        );
    }

    // Toggling back from year -> month must dynamically shrink surface dimensions immediately.
    assert!(rt.pointer_down_surface_point("bar", 1895.0, 12.0, MouseButton::Right));
    rt.step().unwrap();
    assert!(!rt.state().cal_year_mode);
    let shrunk_size = rt
        .surface_logical_size("bar-tooltip")
        .expect("shrunk month size");
    assert!((shrunk_size.width.get() - month_size.width.get()).abs() < 2.0);
    assert!((shrunk_size.height.get() - month_size.height.get()).abs() < 2.0);

    rt.enqueue(Msg::ClockScroll(ScrollEvent {
        position: Point::new(1895.logical_px(), 12.logical_px()),
        local_position: Point::new(10.logical_px(), 12.logical_px()),
        delta_x: 0.0,
        delta_y: -1.0,
        discrete_x: 0,
        discrete_y: -1,
        modifiers: Modifiers::empty(),
    }));
    rt.step().unwrap();
    assert_eq!(rt.state().cal_month_offset, 1);

    assert!(rt.pointer_leave_surface("bar"));
    rt.step().unwrap();
    assert_eq!(rt.surface_count(), 1, "tooltip closes on leave");
}

#[test]
fn expandable_mon_drawer_animates_open_and_closed_on_hover() {
    let mut rt = synthetic_rt();
    populate_sample_metrics(&mut rt);
    rt.enqueue(Msg::Hover(HoverTarget::ExpandSign));
    rt.step().unwrap();
    assert!(rt.state().drawer_target);
    for _ in 0..22 {
        rt.enqueue(Msg::DrawerTick);
        rt.step().unwrap();
    }
    assert!((rt.state().drawer_progress - 1.0).abs() < 1e-4);

    if let Some(buf) = rt.rendered_buffer("bar") {
        maybe_dump_argb8888_ppm("bar_drawer_open", 1920, 24, buf);
    }

    // Switching hover directly from ExpandSign to Disk must keep drawer open!
    rt.enqueue(Msg::Leave(HoverTarget::ExpandSign));
    rt.enqueue(Msg::Hover(HoverTarget::Disk));
    rt.step().unwrap();
    assert!(rt.state().drawer_target);

    // Leaving drawer schedules the grace timer cleanly; step first, then advance past 125ms.
    rt.enqueue(Msg::Leave(HoverTarget::Disk));
    rt.step().unwrap();
    rt.step_at(Instant::now() + Duration::from_millis(200))
        .unwrap();
    assert!(!rt.state().drawer_target);
    for _ in 0..22 {
        rt.enqueue(Msg::DrawerTick);
        rt.step().unwrap();
    }
    assert!(rt.state().drawer_progress <= 1e-4);
}

#[test]
fn tray_deduplicates_aliases_and_tray_menu_highlights_hovered_items() {
    let mut rt = synthetic_rt();
    rt.enqueue(Msg::TrayEvent(Box::new(TrayEvent::ItemRegistered(
        make_tray_snap(":1.193"),
    ))));
    rt.enqueue(Msg::TrayEvent(Box::new(TrayEvent::ItemRegistered(
        make_tray_snap(":1.19"),
    ))));
    rt.step().unwrap();
    assert_eq!(rt.state().tray_items.len(), 1);

    // Open tray menu anchored under click at x = 1580.
    let menu = TrayMenuPopup {
        service: String::from(":1.193/StatusNotifierItem"),
        bus_name: String::from(":1.193"),
        menu_path: String::from("/MenuBar"),
        anchor_x: 1580.0,
        items: vec![
            TrayMenuItem {
                id: 1,
                label: String::from("Keyboard - English (US)"),
                enabled: true,
                visible: true,
                is_separator: false,
                toggle_type: Some(String::from("radio")),
                toggle_state: 1,
                children: Vec::new(),
            },
            TrayMenuItem {
                id: 2,
                label: String::from("Mozc"),
                enabled: true,
                visible: true,
                is_separator: false,
                toggle_type: None,
                toggle_state: 0,
                children: Vec::new(),
            },
            TrayMenuItem {
                id: 3,
                label: String::new(),
                enabled: true,
                visible: true,
                is_separator: true,
                toggle_type: None,
                toggle_state: 0,
                children: Vec::new(),
            },
            TrayMenuItem {
                id: 4,
                label: String::from("Restart"),
                enabled: true,
                visible: true,
                is_separator: false,
                toggle_type: None,
                toggle_state: 0,
                children: Vec::new(),
            },
        ],
    };
    let (left, width) = tray_menu_left_and_width(&menu, rt.state().bar_width);
    assert!(
        (left + width * 0.5 - 1580.0).abs() < 2.0,
        "menu must center under click x=1580: left={left}, width={width}"
    );
    rt.enqueue(Msg::TestSetTrayMenu(menu));
    rt.step().unwrap();
    assert_eq!(rt.surface_count(), 2);

    // Move pointer onto item 2 (`y = 38px`) inside tray menu surface and verify hover fill changes pixels.
    let before = rt.rendered_buffer("bar-tray-menu").unwrap().to_vec();
    assert!(rt.pointer_move_surface_point("bar-tray-menu", 60.0, 38.0));
    rt.step().unwrap();
    assert_eq!(rt.state().hovered, HoverTarget::TrayMenuItem(2));

    let after = rt.rendered_buffer("bar-tray-menu").unwrap();
    assert_ne!(
        before.as_slice(),
        after,
        "hovering menu row must highlight row background"
    );
    // SURFACE0 in little-endian ARGB8888 is [B=68, G=50, R=49, A=255].
    let has_surface0 = after.as_chunks::<4>().0.contains(&[68, 50, 49, 255]);
    assert!(
        has_surface0,
        "hovered tray menu row must paint SURFACE0 highlight"
    );

    let menu_sz = rt.surface_logical_size("bar-tray-menu").unwrap();
    maybe_dump_argb8888_ppm(
        "tray_menu_hovered",
        menu_sz.width.get().round() as u32,
        menu_sz.height.get().round() as u32,
        after,
    );
}

#[test]
fn bluetooth_device_menu_keeps_name_marquee_and_renders_colored_action_buttons() {
    let mut rt = synthetic_rt();
    rt.enqueue(Msg::BluetoothUpdated(BluetoothSnapshot {
        adapters: vec![BluetoothAdapter {
            object_path: String::from("/org/bluez/hci0"),
            powered: true,
            ..BluetoothAdapter::default()
        }],
        devices: vec![BluetoothDevice {
            object_path: String::from("/org/bluez/hci0/dev_AA_BB_CC_DD_EE_FF"),
            adapter_path: String::from("/org/bluez/hci0"),
            alias: String::from("LG V60 DS ThinQ"),
            paired: true,
            ..BluetoothDevice::default()
        }],
    }));
    rt.step().unwrap();

    // Open the popup the same way a bar-icon left-click does, and let the
    // open animation finish so the whole card is visible for rendering.
    rt.enqueue(Msg::BtBarPointerDown(PointerEvent {
        position: Point::new(1500.logical_px(), 12.logical_px()),
        local_position: Point::ZERO,
        button: Some(MouseButton::Left),
        modifiers: Modifiers::empty(),
    }));
    rt.step().unwrap();
    assert!(rt.state().bt_ui.popup_target);
    for _ in 0..20 {
        rt.enqueue(Msg::BtPopupAnimTick);
        rt.step().unwrap();
    }

    // Right-click the device row: the context menu must open...
    let dev_path = rt.state().bluetooth.devices[0].object_path.clone();
    rt.enqueue(Msg::BtDevicePointerDown {
        path: dev_path.clone(),
        ev: PointerEvent {
            position: Point::new(1400.logical_px(), 60.logical_px()),
            local_position: Point::ZERO,
            button: Some(MouseButton::Right),
            modifiers: Modifiers::empty(),
        },
    });
    rt.step().unwrap();
    assert_eq!(rt.state().bt_ui.menu_path.as_deref(), Some(dev_path.as_str()));
    rt.step().unwrap();

    // ...and a second right-click on the menu card must collapse it back to the row.
    rt.enqueue(Msg::BtDevicePointerDown {
        path: dev_path.clone(),
        ev: PointerEvent {
            position: Point::new(1400.logical_px(), 60.logical_px()),
            local_position: Point::ZERO,
            button: Some(MouseButton::Right),
            modifiers: Modifiers::empty(),
        },
    });
    rt.step().unwrap();
    assert_eq!(rt.state().bt_ui.menu_path, None);

    // Re-open and verify the rendered card keeps the device name visible and paints
    // the accent-framed action buttons: Connect (green) and Forget (red) are unique
    // colors in the popup, so finding their pixels proves the buttons are on screen.
    rt.enqueue(Msg::BtDevicePointerDown {
        path: dev_path,
        ev: PointerEvent {
            position: Point::new(1400.logical_px(), 60.logical_px()),
            local_position: Point::ZERO,
            button: Some(MouseButton::Right),
            modifiers: Modifiers::empty(),
        },
    });
    rt.step().unwrap();
    rt.step().unwrap();

    let buf = rt
        .rendered_buffer("bar-bluetooth-popup")
        .expect("bluetooth popup renders headless");
    let sz = rt.surface_logical_size("bar-bluetooth-popup").unwrap();
    let width = sz.width.get().round() as usize;
    let height = sz.height.get().round() as usize;
    assert_eq!(buf.len(), width * height * 4);
    maybe_dump_argb8888_ppm("bt_device_menu", width as u32, height as u32, buf);

    let near = |px: &[u8; 4], rgb: [u8; 3], tol: i16| {
        (i16::from(px[2]) - i16::from(rgb[0])).abs() <= tol
            && (i16::from(px[1]) - i16::from(rgb[1])).abs() <= tol
            && (i16::from(px[0]) - i16::from(rgb[2])).abs() <= tol
            && px[3] == 255
    };
    let has_color = |rgb: [u8; 3]| {
        buf.as_chunks::<4>()
            .0
            .iter()
            .any(|px| near(px, rgb, 12))
    };
    assert!(
        has_color([166, 227, 161]),
        "green Connect/Pai button must render inside the popup"
    );
    assert!(
        has_color([243, 139, 168]),
        "red Forget button must render inside the popup"
    );

    // The menu card must keep the scrolling device name visible: scan the first card
    // region for TEXT-colored pixels (no other popup text uses pure TEXT on MANTLE).
    // Card geometry at full ease: popup y = BAR_HEIGHT - 8 + 18 = 34, header 34px,
    // separator 1px, list padding 6px, so the first row spans y = 75..113.
    let left_x = rt.state().bt_ui.popup_left_x.round() as usize;
    let name_visible = buf
        .as_chunks::<4>()
        .0 // little-endian ARGB8888: [B, G, R, A]
        .iter()
        .enumerate()
        .any(|(i, px)| {
            let x = i % width;
            let y = i / width;
            (75..=112).contains(&y)
                && (left_x + 10..left_x + 106).contains(&x)
                && near(px, [205, 214, 244], 10)
        });
    assert!(name_visible, "device name must stay visible inside the menu card");
}

#[test]
fn mixer_popup_bar_click_opens_right_click_mutes_and_tracks_set_volume() {
    use waywi::audio::AudioStream;

    let mut rt = synthetic_rt();
    rt.enqueue(Msg::AudioUpdated(AudioSnapshot {
        linear: 0.50,
        default_sink: Some(AudioDevice {
            id: 1,
            name: String::from("alsa_output.pci-0000_00_1f.3"),
            description: String::from("Built-in Audio Analog Stereo"),
        }),
        streams: vec![AudioStream {
            id: 7,
            name: String::from("Firefox"),
            binary: None,
            icon_name: None,
            app_id: None,
            sink_id: Some(1),
            linear: 0.40,
            db: None,
            muted: false,
        }],
        ..AudioSnapshot::default()
    }));
    rt.step().unwrap();
    assert!(!rt.state().audio_ui.is_popup_visible());

    // Left-click on the volume module opens the mixer popup anchored to the module.
    rt.enqueue(Msg::MixerBarPointerDown(PointerEvent {
        position: Point::new(1650.logical_px(), 12.logical_px()),
        local_position: Point::ZERO,
        button: Some(MouseButton::Left),
        modifiers: Modifiers::empty(),
    }));
    rt.step().unwrap();
    assert!(rt.state().audio_ui.popup_target);
    assert!(rt.state().audio_ui.popup_left_x > 8.0);

    // Right-click on the module toggles mute for the controlled target (sink).
    rt.enqueue(Msg::MixerBarPointerDown(PointerEvent {
        position: Point::new(1650.logical_px(), 12.logical_px()),
        local_position: Point::ZERO,
        button: Some(MouseButton::Right),
        modifiers: Modifiers::empty(),
    }));
    rt.step().unwrap();
    assert!(rt.state().audio.as_ref().unwrap().muted);
    rt.enqueue(Msg::MixerBarPointerDown(PointerEvent {
        position: Point::new(1650.logical_px(), 12.logical_px()),
        local_position: Point::ZERO,
        button: Some(MouseButton::Right),
        modifiers: Modifiers::empty(),
    }));
    rt.step().unwrap();
    assert!(!rt.state().audio.as_ref().unwrap().muted);

    let scroll_on_master = |delta_y: f32, discrete_y: i32| {
        Msg::MixerMasterScroll(ScrollEvent {
            position: Point::new(1700.logical_px(), 60.logical_px()),
            local_position: Point::ZERO,
            delta_x: 0.0,
            delta_y,
            discrete_x: 0,
            discrete_y,
            modifiers: Modifiers::empty(),
        })
    };

    // Scroll up on the master track raises the sink volume (same 0.01 step as the bar module).
    rt.enqueue(scroll_on_master(-1.0, -1));
    rt.step().unwrap();
    let vol = rt.state().audio.as_ref().unwrap().linear;
    assert!((vol - 0.51).abs() < 1e-4, "expected 0.51, got {vol}");

    // Clicking the middle of the master track pins the volume at exactly 50%.
    rt.enqueue(Msg::MixerMasterSeekClick(PointerEvent {
        position: Point::new(1700.logical_px(), 60.logical_px()),
        local_position: Point::new((crate::theme::MIXER_TRACK_WIDTH / 2.0).logical_px(), 3.logical_px()),
        button: Some(MouseButton::Left),
        modifiers: Modifiers::empty(),
    }));
    rt.step().unwrap();
    let vol = rt.state().audio.as_ref().unwrap().linear;
    assert!((vol - 0.50).abs() < 1e-4, "expected 0.50, got {vol}");

    // Per-app stream: scroll adjusts and click pins row volume.
    rt.enqueue(Msg::MixerStreamScroll {
        id: 7,
        sc: ScrollEvent {
            position: Point::new(1700.logical_px(), 120.logical_px()),
            local_position: Point::ZERO,
            delta_x: 0.0,
            delta_y: 1.0,
            discrete_x: 0,
            discrete_y: 1,
            modifiers: Modifiers::empty(),
        },
    });
    rt.step().unwrap();
    let row_vol = rt.state().audio.as_ref().unwrap().stream_percent(7);
    assert!((row_vol - 39.0).abs() < 1e-3, "expected 39%, got {row_vol}");

    rt.enqueue(Msg::MixerStreamSeekClick {
        id: 7,
        ev: PointerEvent {
            position: Point::new(1700.logical_px(), 120.logical_px()),
            local_position: Point::new((crate::theme::MIXER_TRACK_WIDTH / 4.0).logical_px(), 3.logical_px()),
            button: Some(MouseButton::Left),
            modifiers: Modifiers::empty(),
        },
    });
    rt.step().unwrap();
    let row_vol = rt.state().audio.as_ref().unwrap().stream_percent(7);
    assert!((row_vol - 25.0).abs() < 1e-3, "expected 25%, got {row_vol}");

    // Let the open animation finish, then render the popup headless.
    for _ in 0..20 {
        rt.enqueue(Msg::MixerPopupAnimTick);
        rt.step().unwrap();
    }
    let buf = rt
        .rendered_buffer("bar-mixer-popup")
        .expect("mixer popup renders headless");
    let sz = rt.surface_logical_size("bar-mixer-popup").unwrap();
    let width = sz.width.get().round() as usize;
    let height = sz.height.get().round() as usize;
    assert_eq!(buf.len(), width * height * 4);
    maybe_dump_argb8888_ppm("mixer_popup", width as u32, height as u32, buf);

    // MAUVE fill pixels prove the master fill and app-row tracks render.
    let near = |px: &[u8; 4], rgb: [u8; 3], tol: i16| {
        (i16::from(px[2]) - i16::from(rgb[0])).abs() <= tol
            && (i16::from(px[1]) - i16::from(rgb[1])).abs() <= tol
            && (i16::from(px[0]) - i16::from(rgb[2])).abs() <= tol
            && px[3] == 255
    };
    assert!(
        buf.as_chunks::<4>()
            .0
            .iter()
            .any(|px| near(px, [203, 166, 247], 12)),
        "mauve volume fill must render inside the popup"
    );

    // Closing collapses back to the bar only.
    rt.enqueue(Msg::CloseMixerPopup);
    for _ in 0..20 {
        rt.enqueue(Msg::MixerPopupAnimTick);
        rt.step().unwrap();
    }
    assert!(!rt.state().audio_ui.is_popup_visible());
}
