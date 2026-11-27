//! Headless tests for minimalist per-workspace Screen Annotation mode
//! (mouse, pen tablet pressure + physical eraser, touch, vector segment-splitting
//! eraser, Ctrl+Z / Ctrl+Y undo-redo, per-workspace persistence, and memory freeing).

use waywi::prelude::*;

use crate::app::Msg;
use crate::app::annotate::{AnnotateTool, WorkspaceKey};
use crate::app::ipc::{IpcAction, IpcTarget, parse_ipc_command};

use super::support::{settle_ui_anims, synthetic_rt};

fn ptr_ev(x: f32, y: f32, button: Option<MouseButton>) -> PointerEvent {
    PointerEvent {
        position: Point::new(x.logical_px(), y.logical_px()),
        local_position: Point::new(x.logical_px(), y.logical_px()),
        button,
        modifiers: Modifiers::empty(),
    }
}

fn key_ev(key: Key, modifiers: Modifiers, utf8: Option<&str>) -> KeyEvent {
    KeyEvent {
        key,
        modifiers,
        utf8: utf8.map(|s| SharedStr::from(s.to_owned())),
        repeat: false,
    }
}

fn stylus_ev(
    tool_id: u64,
    tool_type: TabletToolType,
    kind: TabletToolEventKind,
    x: f32,
    y: f32,
    pressure: f32,
    tip_down: bool,
) -> TabletEvent {
    TabletEvent::Tool(TabletToolEvent {
        tool_id: TabletToolId(tool_id),
        tablet_id: Some(TabletId(1)),
        tool_type,
        hardware_serial: 0x1234,
        hardware_id_wacom: 0,
        kind,
        position: Point::new(x.logical_px(), y.logical_px()),
        local_position: Point::new(x.logical_px(), y.logical_px()),
        axes: TabletToolAxes {
            pressure,
            ..TabletToolAxes::default()
        },
        tip_down,
        modifiers: Modifiers::empty(),
    })
}

#[test]
fn annotate_ipc_parses_aliases_and_toggles_overlay_exclusively() {
    for raw in ["annotate", "--toggle draw", "open sketch", "paint"] {
        let parsed = parse_ipc_command(raw).expect("valid annotate IPC command");
        assert!(matches!(
            parsed,
            Msg::IpcControl {
                target: IpcTarget::Annotate,
                ..
            }
        ));
    }
    assert!(matches!(
        parse_ipc_command("close annotate"),
        Some(Msg::IpcControl {
            action: IpcAction::Close,
            target: IpcTarget::Annotate
        })
    ));

    let read_rss_kib = || -> u64 {
        std::fs::read_to_string("/proc/self/status")
            .ok()
            .and_then(|s| {
                s.lines()
                    .find(|l| l.starts_with("VmRSS:"))
                    .and_then(|l| l.split_whitespace().nth(1))
                    .and_then(|v| v.parse::<u64>().ok())
            })
            .unwrap_or(0)
    };

    let mut rt = synthetic_rt();
    rt.step().unwrap();
    let rss_before_kib = read_rss_kib();

    rt.enqueue(parse_ipc_command("annotate").unwrap());
    settle_ui_anims(&mut rt);
    assert!(rt.state().annotate.is_active());
    assert!(rt.rendered_buffer("screen-annotate").is_some());
    let rss_open_kib = read_rss_kib();

    // Opening the overlay must NEVER trigger a massive cosmic-text fontdb fallback mmap spike.
    let rss_delta_mib = rss_open_kib.saturating_sub(rss_before_kib) / 1024;
    assert!(
        rss_delta_mib < 64,
        "opening screen-annotate spiked RSS by {rss_delta_mib} MiB (before={rss_before_kib} KiB, open={rss_open_kib} KiB)"
    );

    // Closing unmounts the overlay surface completely (0 idle buffer overhead).
    rt.enqueue(parse_ipc_command("close annotate").unwrap());
    settle_ui_anims(&mut rt);
    assert!(!rt.state().annotate.is_active());
    assert!(rt.rendered_buffer("screen-annotate").is_none());
}

#[test]
fn annotate_mouse_tablet_pressure_eraser_split_undo_redo_and_memory_free() {
    let mut rt = synthetic_rt();
    rt.enqueue(Msg::IpcControl {
        action: IpcAction::Toggle,
        target: IpcTarget::Annotate,
    });
    settle_ui_anims(&mut rt);

    // 1. Switch to Red (color index 1) and draw a long horizontal stroke with pen tablet pressure.
    rt.enqueue(Msg::AnnotateKey(key_ev(
        Key::from_keysym(b'2' as u32),
        Modifiers::empty(),
        Some("2"),
    )));
    rt.enqueue(Msg::AnnotateTablet(stylus_ev(
        1,
        TabletToolType::Pen,
        TabletToolEventKind::Down,
        100.0,
        300.0,
        0.15,
        true,
    )));
    for step in 1..=10 {
        let x = 100.0 + (step as f32) * 30.0;
        let pressure = 0.15 + (step as f32) * 0.085;
        rt.enqueue(Msg::AnnotateTablet(stylus_ev(
            1,
            TabletToolType::Pen,
            TabletToolEventKind::Motion,
            x,
            300.0,
            pressure,
            true,
        )));
    }
    rt.enqueue(Msg::AnnotateTablet(stylus_ev(
        1,
        TabletToolType::Pen,
        TabletToolEventKind::Up,
        400.0,
        300.0,
        0.0,
        false,
    )));
    rt.step().unwrap();

    let ws1 = WorkspaceKey::Numbered(1);
    let board = rt.state().annotate.board(&ws1).expect("WS 1 board created");
    assert_eq!(board.strokes.len(), 1);
    let stroke = &board.strokes[0];
    assert!(stroke.pressure_sensitive);
    assert!(
        stroke.radius_at(0.95) > stroke.radius_at(0.15) * 2.0,
        "pressure must vary stroke width smoothly"
    );

    // Verify red ink pixels actually rendered into the overlay buffer!
    let buf = rt
        .rendered_buffer("screen-annotate")
        .expect("annotate surface rendered");
    let has_red_ink = buf.as_chunks::<4>().0.iter().any(|px| {
        // Little-endian ARGB8888: [B, G, R, A] vs RED = (243, 139, 168)
        px[3] > 200
            && (i16::from(px[2]) - 243).abs() <= 16
            && (i16::from(px[1]) - 139).abs() <= 16
            && (i16::from(px[0]) - 168).abs() <= 16
    });
    assert!(has_red_ink, "red tablet stroke must paint onto overlay");

    // 2. Flip stylus to physical Eraser and slice vertically across the middle (x = 250) -> splits into 2 pieces!
    rt.enqueue(Msg::AnnotateTablet(stylus_ev(
        2,
        TabletToolType::Eraser,
        TabletToolEventKind::Down,
        250.0,
        260.0,
        0.8,
        true,
    )));
    rt.enqueue(Msg::AnnotateTablet(stylus_ev(
        2,
        TabletToolType::Eraser,
        TabletToolEventKind::Motion,
        250.0,
        340.0,
        0.8,
        true,
    )));
    rt.enqueue(Msg::AnnotateTablet(stylus_ev(
        2,
        TabletToolType::Eraser,
        TabletToolEventKind::Up,
        250.0,
        340.0,
        0.0,
        false,
    )));
    rt.step().unwrap();
    assert_eq!(
        rt.state().annotate.board(&ws1).unwrap().strokes.len(),
        2,
        "vector segment-splitting eraser must split the stroke into left and right pieces"
    );

    // 3. Ctrl+Z restores the original single unsplit stroke; Ctrl+Y redoes the split!
    rt.enqueue(Msg::AnnotateKey(key_ev(
        Key::from_keysym(b'z' as u32),
        Modifiers::CTRL,
        Some("z"),
    )));
    rt.step().unwrap();
    assert_eq!(rt.state().annotate.board(&ws1).unwrap().strokes.len(), 1);

    rt.enqueue(Msg::AnnotateKey(key_ev(
        Key::from_keysym(b'y' as u32),
        Modifiers::CTRL,
        Some("y"),
    )));
    rt.step().unwrap();
    assert_eq!(rt.state().annotate.board(&ws1).unwrap().strokes.len(), 2);

    // 4. Exit via Escape: surface unmounts cleanly, strokes remain saved for WS 1.
    rt.enqueue(Msg::AnnotateKey(key_ev(
        Key::Escape,
        Modifiers::empty(),
        None,
    )));
    settle_ui_anims(&mut rt);
    assert!(!rt.state().annotate.is_active());
    assert!(rt.rendered_buffer("screen-annotate").is_none());
    assert_eq!(rt.state().annotate.board(&ws1).unwrap().strokes.len(), 2);

    // Re-enter: WS 1 strokes are still there!
    rt.enqueue(Msg::IpcControl {
        action: IpcAction::Toggle,
        target: IpcTarget::Annotate,
    });
    settle_ui_anims(&mut rt);
    assert_eq!(rt.state().annotate.board(&ws1).unwrap().strokes.len(), 2);

    // 5. Switch to WS 2: clean board; draw with mouse Highlighter + touch stroke.
    rt.enqueue(Msg::SwitchWorkspace(WorkspaceId::new(2)));
    rt.step().unwrap();
    let ws2 = WorkspaceKey::Numbered(2);
    assert!(rt.state().annotate.board(&ws2).is_none());

    rt.enqueue(Msg::AnnotateSetTool(AnnotateTool::Highlighter));
    rt.enqueue(Msg::AnnotatePointerDown(ptr_ev(
        120.0,
        200.0,
        Some(MouseButton::Left),
    )));
    rt.enqueue(Msg::AnnotatePointerMove(ptr_ev(240.0, 200.0, None)));
    rt.enqueue(Msg::AnnotatePointerUp(ptr_ev(
        240.0,
        200.0,
        Some(MouseButton::Left),
    )));
    // Single-finger touch stroke also works cleanly.
    rt.enqueue(Msg::AnnotateTouch(TouchEvent {
        id: 1,
        phase: TouchPhase::Down,
        position: Point::new(150.logical_px(), 400.logical_px()),
        local_position: Point::new(150.logical_px(), 400.logical_px()),
        shape: None,
        orientation: None,
        modifiers: Modifiers::empty(),
    }));
    rt.enqueue(Msg::AnnotateTouch(TouchEvent {
        id: 1,
        phase: TouchPhase::Move,
        position: Point::new(210.logical_px(), 420.logical_px()),
        local_position: Point::new(210.logical_px(), 420.logical_px()),
        shape: None,
        orientation: None,
        modifiers: Modifiers::empty(),
    }));
    rt.enqueue(Msg::AnnotateTouch(TouchEvent {
        id: 1,
        phase: TouchPhase::Up,
        position: Point::new(210.logical_px(), 420.logical_px()),
        local_position: Point::new(210.logical_px(), 420.logical_px()),
        shape: None,
        orientation: None,
        modifiers: Modifiers::empty(),
    }));
    rt.step().unwrap();
    assert_eq!(rt.state().annotate.board(&ws2).unwrap().strokes.len(), 2);
    assert_eq!(rt.state().annotate.board(&ws1).unwrap().strokes.len(), 2);

    // 6. Pin visible (`K`): exiting keeps surface mounted in PassThrough mode for WS 2,
    //    and unmounts when switching to empty WS 3.
    rt.enqueue(Msg::AnnotateKey(key_ev(
        Key::from_keysym(b'k' as u32),
        Modifiers::empty(),
        Some("k"),
    )));
    rt.enqueue(Msg::CloseAnnotate);
    settle_ui_anims(&mut rt);
    assert!(rt.state().annotate.pinned_visible);
    assert!(rt.rendered_buffer("screen-annotate").is_some());

    rt.enqueue(Msg::SwitchWorkspace(WorkspaceId::new(3)));
    rt.step().unwrap();
    assert!(
        rt.rendered_buffer("screen-annotate").is_none(),
        "empty workspace must not mount pinned annotation overlay"
    );

    // 7. Clear Workspace frees only current workspace; Clear All frees every board and capacity.
    rt.enqueue(Msg::SwitchWorkspace(WorkspaceId::new(2)));
    rt.enqueue(Msg::IpcControl {
        action: IpcAction::Toggle,
        target: IpcTarget::Annotate,
    });
    rt.enqueue(Msg::AnnotateClearWorkspace);
    rt.step().unwrap();
    assert!(rt.state().annotate.board(&ws2).is_none());
    assert_eq!(rt.state().annotate.board(&ws1).unwrap().strokes.len(), 2);

    rt.enqueue(Msg::AnnotateClearAll);
    rt.step().unwrap();
    assert!(rt.state().annotate.boards.is_empty());
    assert_eq!(
        rt.state().annotate.boards.capacity(),
        0,
        "clear_all must shrink board HashMap capacity to 0 to free memory"
    );
}
