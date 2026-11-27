//! Input, tablet, touch, keyboard shortcut, and lifecycle handlers for Screen Annotation mode.

use waywi::prelude::*;

use crate::app::annotate::{ANNOTATE_PALETTE, ActiveInputSource, AnnotateTool, StrokeInputStart};
use crate::app::state::{SoundCue, State};

use super::events::scroll_direction_y;
use super::ipc::close_other_screen_tools;

/// Linux input code for primary stylus side/barrel button (`BTN_STYLUS`).
const BTN_STYLUS: u32 = 0x14b;

pub fn handle_toggle_annotate(state: &mut State, ctx: &mut EffectCtx<'_, crate::app::msg::Msg>) {
    let ws = state.active_workspace_key();
    if state.annotate.is_active() {
        state.annotate.close(&ws);
        if !state.annotate.should_mount_surface(&ws) {
            ctx.trim_memory();
        }
        state.play_sound(SoundCue::MenuPopdown, ctx);
    } else {
        close_other_screen_tools(state, Some("annotate"), ctx);
        state.annotate.open();
        state.play_sound(SoundCue::MenuPopup, ctx);
    }
}

pub fn handle_close_annotate(state: &mut State, ctx: &mut EffectCtx<'_, crate::app::msg::Msg>) {
    if state.annotate.is_active() {
        let ws = state.active_workspace_key();
        state.annotate.close(&ws);
        if !state.annotate.should_mount_surface(&ws) {
            ctx.trim_memory();
        }
        state.play_sound(SoundCue::MenuPopdown, ctx);
    }
}

pub fn handle_annotate_pointer_down(state: &mut State, ev: PointerEvent) {
    if !state.annotate.is_active() {
        return;
    }
    let force_eraser = match ev.button {
        Some(MouseButton::Left) => false,
        Some(MouseButton::Right) => true,
        _ => return,
    };
    let ws = state.active_workspace_key();
    state.annotate.begin_input(
        &ws,
        StrokeInputStart {
            source: ActiveInputSource::Pointer,
            x: ev.position.x.get(),
            y: ev.position.y.get(),
            pressure: 1.0,
            pressure_sensitive: false,
            force_eraser,
        },
    );
}

pub fn handle_annotate_pointer_move(state: &mut State, ev: PointerEvent) {
    if !state.annotate.is_active() {
        return;
    }
    let ws = state.active_workspace_key();
    state.annotate.move_input(
        &ws,
        ActiveInputSource::Pointer,
        ev.position.x.get(),
        ev.position.y.get(),
        1.0,
    );
}

pub fn handle_annotate_pointer_up(state: &mut State, _ev: PointerEvent) {
    let ws = state.active_workspace_key();
    state
        .annotate
        .end_input(&ws, Some(ActiveInputSource::Pointer));
}

pub fn handle_annotate_scroll(state: &mut State, sc: ScrollEvent) {
    if !state.annotate.is_active() {
        return;
    }
    let dir = scroll_direction_y(sc);
    if dir != 0 {
        // Scroll up (`dir < 0`) grows brush; scroll down (`dir > 0`) shrinks brush.
        state.annotate.step_brush_size(if dir < 0 { 1.0 } else { -1.0 });
    }
}

pub fn handle_annotate_tablet(
    state: &mut State,
    ev: TabletEvent,
    ctx: &mut EffectCtx<'_, crate::app::msg::Msg>,
) {
    if !state.annotate.is_active() {
        return;
    }
    let ws = state.active_workspace_key();
    match ev {
        TabletEvent::Tool(tool_ev) => {
            let source = ActiveInputSource::Tablet(tool_ev.tool_id);
            let is_physical_eraser = tool_ev.tool_type == TabletToolType::Eraser;
            match tool_ev.kind {
                TabletToolEventKind::Down => {
                    let p = if tool_ev.axes.pressure > 0.0 {
                        tool_ev.axes.pressure
                    } else {
                        0.5
                    };
                    state.annotate.begin_input(
                        &ws,
                        StrokeInputStart {
                            source,
                            x: tool_ev.position.x.get(),
                            y: tool_ev.position.y.get(),
                            pressure: p,
                            pressure_sensitive: true,
                            force_eraser: is_physical_eraser,
                        },
                    );
                }
                TabletToolEventKind::Motion => {
                    let p = if tool_ev.axes.pressure > 0.0 {
                        tool_ev.axes.pressure
                    } else {
                        0.5
                    };
                    if tool_ev.tip_down
                        && state.annotate.active_stroke.is_none()
                        && state.annotate.active_eraser.is_none()
                    {
                        state.annotate.begin_input(
                            &ws,
                            StrokeInputStart {
                                source,
                                x: tool_ev.position.x.get(),
                                y: tool_ev.position.y.get(),
                                pressure: p,
                                pressure_sensitive: true,
                                force_eraser: is_physical_eraser,
                            },
                        );
                    } else {
                        state.annotate.move_input(
                            &ws,
                            source,
                            tool_ev.position.x.get(),
                            tool_ev.position.y.get(),
                            p,
                        );
                    }
                }
                TabletToolEventKind::Up | TabletToolEventKind::ProximityOut => {
                    state.annotate.end_input(&ws, Some(source));
                }
                TabletToolEventKind::Button { button, pressed } if button == BTN_STYLUS => {
                    state.annotate.stylus_barrel_eraser = pressed;
                }
                TabletToolEventKind::ProximityIn | TabletToolEventKind::Button { .. } => {
                    state.annotate.cursor_pos =
                        Some((tool_ev.position.x.get(), tool_ev.position.y.get()));
                }
            }
        }
        TabletEvent::Pad(pad_ev) => match pad_ev.kind {
            TabletPadEventKind::Button {
                button: 0,
                pressed: true,
            } => handle_annotate_undo(state, ctx),
            TabletPadEventKind::Button {
                button: 1,
                pressed: true,
            } => handle_annotate_redo(state, ctx),
            TabletPadEventKind::Dial { delta, .. } if delta.abs() > 0.05 => {
                state
                    .annotate
                    .step_brush_size(if delta > 0.0 { 1.0 } else { -1.0 });
            }
            _ => {}
        },
    }
}

pub fn handle_annotate_touch(state: &mut State, ev: TouchEvent) {
    if !state.annotate.is_active() {
        return;
    }
    let ws = state.active_workspace_key();
    let source = ActiveInputSource::Touch(ev.id);
    match ev.phase {
        TouchPhase::Down => {
            // Ignore secondary fingers while an active stroke/eraser is in progress.
            if state.annotate.active_stroke.is_some() || state.annotate.active_eraser.is_some() {
                return;
            }
            state.annotate.begin_input(
                &ws,
                StrokeInputStart {
                    source,
                    x: ev.position.x.get(),
                    y: ev.position.y.get(),
                    pressure: 1.0,
                    pressure_sensitive: false,
                    force_eraser: false,
                },
            );
        }
        TouchPhase::Move => {
            state.annotate.move_input(
                &ws,
                source,
                ev.position.x.get(),
                ev.position.y.get(),
                1.0,
            );
        }
        TouchPhase::Up | TouchPhase::Cancel => {
            state.annotate.end_input(&ws, Some(source));
        }
    }
}

pub fn handle_annotate_key(
    state: &mut State,
    ev: KeyEvent,
    ctx: &mut EffectCtx<'_, crate::app::msg::Msg>,
) {
    if !state.annotate.is_active() {
        return;
    }

    if ev.key == Key::Escape {
        handle_close_annotate(state, ctx);
        return;
    }

    let ctrl = ev.modifiers.contains(Modifiers::CTRL);
    let shift = ev.modifiers.contains(Modifiers::SHIFT);

    // Character shortcuts via utf8 or keysym matching so layout & modifiers work reliably.
    let ch = ev
        .utf8
        .as_deref()
        .and_then(|s| s.chars().next())
        .map(|c| c.to_ascii_lowercase());

    if ctrl {
        if ev.key == Key::from_keysym(b'z' as u32)
            || ev.key == Key::from_keysym(b'Z' as u32)
            || ch == Some('z')
            || ch == Some('\u{1a}')
        {
            if shift {
                handle_annotate_redo(state, ctx);
            } else {
                handle_annotate_undo(state, ctx);
            }
            return;
        }
        if ev.key == Key::from_keysym(b'y' as u32)
            || ev.key == Key::from_keysym(b'Y' as u32)
            || ch == Some('y')
            || ch == Some('\u{19}')
        {
            handle_annotate_redo(state, ctx);
            return;
        }
        if shift && ev.key == Key::Delete {
            handle_annotate_clear_all(state, ctx);
            return;
        }
        return;
    }

    match ch {
        Some('p') => handle_annotate_set_tool(state, AnnotateTool::Pen, ctx),
        Some('h') => handle_annotate_set_tool(state, AnnotateTool::Highlighter, ctx),
        Some('e') => handle_annotate_set_tool(state, AnnotateTool::Eraser, ctx),
        Some('k') => handle_annotate_toggle_pin(state, ctx),
        Some('c') if shift => handle_annotate_clear_all(state, ctx),
        Some('c') => handle_annotate_clear_workspace(state, ctx),
        Some('[' | '-') => state.annotate.step_brush_size(-1.0),
        Some(']' | '=' | '+') => state.annotate.step_brush_size(1.0),
        Some(d @ '1'..='6') => {
            let idx = (d as u8 - b'1') as usize;
            if idx < ANNOTATE_PALETTE.len() {
                handle_annotate_set_color(state, idx, ctx);
            }
        }
        _ => {}
    }
}

pub fn handle_annotate_set_tool(
    state: &mut State,
    tool: AnnotateTool,
    ctx: &EffectCtx<'_, crate::app::msg::Msg>,
) {
    if state.annotate.tool != tool {
        let ws = state.active_workspace_key();
        state.annotate.finish_active_interaction(&ws);
        state.annotate.tool = tool;
        state.play_sound(SoundCue::TabChanged, ctx);
    }
}

pub fn handle_annotate_set_color(
    state: &mut State,
    idx: usize,
    ctx: &EffectCtx<'_, crate::app::msg::Msg>,
) {
    let clamped = idx % ANNOTATE_PALETTE.len();
    state.annotate.color_idx = clamped;
    if state.annotate.tool == AnnotateTool::Eraser {
        state.annotate.tool = AnnotateTool::Pen;
    }
    state.play_sound(SoundCue::TabChanged, ctx);
}

pub fn handle_annotate_undo(state: &mut State, ctx: &EffectCtx<'_, crate::app::msg::Msg>) {
    let ws = state.active_workspace_key();
    if state.annotate.undo(&ws) {
        state.play_sound(SoundCue::ToggleOff, ctx);
    }
}

pub fn handle_annotate_redo(state: &mut State, ctx: &EffectCtx<'_, crate::app::msg::Msg>) {
    let ws = state.active_workspace_key();
    if state.annotate.redo(&ws) {
        state.play_sound(SoundCue::ToggleOn, ctx);
    }
}

pub fn handle_annotate_clear_workspace(
    state: &mut State,
    ctx: &mut EffectCtx<'_, crate::app::msg::Msg>,
) {
    let ws = state.active_workspace_key();
    if state.annotate.clear_workspace(&ws) {
        // Free cached Canvas pixmaps/textures alongside the dropped vector buffers.
        ctx.trim_memory();
        state.play_sound(SoundCue::TrashEmpty, ctx);
    }
}

pub fn handle_annotate_clear_all(
    state: &mut State,
    ctx: &mut EffectCtx<'_, crate::app::msg::Msg>,
) {
    if state.annotate.clear_all() {
        ctx.trim_memory();
        state.play_sound(SoundCue::TrashEmpty, ctx);
    }
}

pub fn handle_annotate_toggle_pin(state: &mut State, ctx: &EffectCtx<'_, crate::app::msg::Msg>) {
    state.annotate.pinned_visible = !state.annotate.pinned_visible;
    let cue = if state.annotate.pinned_visible {
        SoundCue::ToggleOn
    } else {
        SoundCue::ToggleOff
    };
    state.play_sound(cue, ctx);
}
