//! Headless tests for native `drun` launcher and external `Control` IPC triggers.

use std::path::PathBuf;

use waywi::prelude::*;

use crate::app::Msg;
use crate::app::ipc::{IpcAction, IpcTarget, parse_ipc_command};

use super::support::synthetic_rt;

fn sample_desktop_entry(id: &str, name: &str) -> DesktopEntry {
    DesktopEntry {
        id: id.to_owned(),
        path: PathBuf::from(format!("/usr/share/applications/{id}")),
        name: name.to_owned(),
        generic_name: None,
        comment: None,
        exec: Some(String::from("true")),
        try_exec: None,
        working_dir: None,
        icon: None,
        categories: Vec::new(),
        keywords: Vec::new(),
        terminal: false,
        no_display: false,
        hidden: false,
        only_show_in: Vec::new(),
        not_show_in: Vec::new(),
        startup_wm_class: None,
        dbus_activatable: false,
        actions: Vec::new(),
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

#[test]
fn ipc_command_parser_handles_all_widgets_and_actions() {
    assert_eq!(
        parse_ipc_command("drun").map(|m| matches!(
            m,
            Msg::IpcControl {
                action: IpcAction::Toggle,
                target: IpcTarget::Drun
            }
        )),
        Some(true)
    );
    assert_eq!(
        parse_ipc_command("--open notifications").map(|m| matches!(
            m,
            Msg::IpcControl {
                action: IpcAction::Open,
                target: IpcTarget::Notifications
            }
        )),
        Some(true)
    );
    assert_eq!(
        parse_ipc_command("toggle dnd").map(|m| matches!(
            m,
            Msg::IpcControl {
                action: IpcAction::Toggle,
                target: IpcTarget::Dnd
            }
        )),
        Some(true)
    );
    assert_eq!(
        parse_ipc_command("volume").map(|m| matches!(
            m,
            Msg::IpcControl {
                action: IpcAction::Toggle,
                target: IpcTarget::Volume
            }
        )),
        Some(true)
    );
    assert_eq!(
        parse_ipc_command("bluetooth").map(|m| matches!(
            m,
            Msg::IpcControl {
                action: IpcAction::Toggle,
                target: IpcTarget::Bluetooth
            }
        )),
        Some(true)
    );
    assert_eq!(
        parse_ipc_command("disk").map(|m| matches!(
            m,
            Msg::IpcControl {
                action: IpcAction::Toggle,
                target: IpcTarget::Disk
            }
        )),
        Some(true)
    );
    assert_eq!(
        parse_ipc_command("mpris").map(|m| matches!(
            m,
            Msg::IpcControl {
                action: IpcAction::Toggle,
                target: IpcTarget::Mpris
            }
        )),
        Some(true)
    );
    assert_eq!(
        parse_ipc_command("close").map(|m| matches!(
            m,
            Msg::IpcControl {
                action: IpcAction::Close,
                target: IpcTarget::All
            }
        )),
        Some(true)
    );
    assert!(parse_ipc_command("unknown-cmd").is_none());
}

#[test]
fn ipc_triggers_toggle_overlays_exclusively() {
    let mut rt = synthetic_rt();
    rt.step().unwrap();

    rt.enqueue(parse_ipc_command("volume").unwrap());
    rt.step().unwrap();
    assert!(rt.state().audio_ui.is_popup_open());

    rt.enqueue(parse_ipc_command("bluetooth").unwrap());
    rt.step().unwrap();
    assert!(rt.state().bt_ui.is_popup_open());
    assert!(!rt.state().audio_ui.is_popup_open());

    rt.enqueue(parse_ipc_command("disk").unwrap());
    rt.step().unwrap();
    assert!(rt.state().disk_ui.is_popup_open());
    assert!(!rt.state().bt_ui.is_popup_open());

    rt.enqueue(parse_ipc_command("mpris").unwrap());
    rt.step().unwrap();
    assert!(rt.state().mpris.is_popup_open());
    assert!(!rt.state().disk_ui.is_popup_open());

    rt.enqueue(parse_ipc_command("notifications").unwrap());
    rt.step().unwrap();
    assert!(rt.state().notifications.is_center_open());
    assert!(!rt.state().mpris.is_popup_open());

    rt.enqueue(parse_ipc_command("dnd").unwrap());
    rt.step().unwrap();
    assert!(rt.state().notifications.dnd);

    rt.enqueue(parse_ipc_command("close").unwrap());
    rt.step().unwrap();
    assert!(!rt.state().notifications.is_center_open());
}

#[test]
fn drun_filters_grows_input_navigates_past_top5_and_renders() {
    let mut rt = synthetic_rt();
    rt.enqueue(Msg::TestSetDrunEntries(vec![
        sample_desktop_entry("alacritty.desktop", "Alacritty"),
        sample_desktop_entry("firefox.desktop", "Firefox"),
        sample_desktop_entry("foot.desktop", "Foot Terminal"),
        sample_desktop_entry("gimp.desktop", "GNU Image Manipulation Program"),
        sample_desktop_entry("thunar.desktop", "Thunar"),
        sample_desktop_entry("wezterm.desktop", "WezTerm"),
    ]));
    rt.enqueue(Msg::ToggleDrun);
    rt.step().unwrap();
    assert!(rt.state().drun.is_open());
    assert_eq!(rt.state().drun.visible_match_indices().len(), 5);
    assert!(
        rt.rendered_buffer("bar-drun").is_some(),
        "bar-drun overlay surface must render"
    );

    // Arrow navigation past top 5 slides the 5-entry window.
    for _ in 0..5 {
        rt.enqueue(Msg::DrunKey(key_ev(Key::Down, None)));
    }
    rt.step().unwrap();
    assert_eq!(rt.state().drun.selected, 5);
    assert_eq!(rt.state().drun.scroll_start, 1);
    assert_eq!(rt.state().drun.visible_match_indices().len(), 5);

    // Typing filters down to 1 entry ("Thunar") and grows measured text width.
    let w_before = rt.state().drun.input_box_width(rt.state().bar_width);
    for ch in "thunar".chars() {
        let s = ch.to_string();
        rt.enqueue(Msg::DrunKey(key_ev(Key::Space, Some(&s))));
    }
    rt.step().unwrap();
    assert_eq!(rt.state().drun.visible_match_indices().len(), 1);
    assert_eq!(
        rt.state().drun.selected_entry().map(|e| e.name.as_str()),
        Some("Thunar")
    );
    assert!(rt.state().drun.measured_text_w > 0.0);
    assert!(rt.state().drun.input_box_width(rt.state().bar_width) >= w_before);

    // Escape closes drun.
    rt.enqueue(Msg::DrunKey(key_ev(Key::Escape, None)));
    rt.step().unwrap();
    assert!(!rt.state().drun.is_open());
}
