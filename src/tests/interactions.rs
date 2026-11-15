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
