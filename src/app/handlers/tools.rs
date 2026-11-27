//! Handlers for native WayWi `screencopy` Color Picker and Screenshot (Area / Window / Full).

use std::sync::Arc;

use waywi::desktop::spawn_detached;
use waywi::prelude::*;
use waywi::system::LocalDateTime;

use crate::app::clipboard::temp_open_image_path;
use crate::app::color_picker::ColorPickerPhase;
use crate::app::msg::Msg;
use crate::app::screenshot::{
    PreviewInit, ScreenshotMode, ScreenshotPhase, ScreenshotStart, WindowTarget,
    crop_and_encode_png, default_screenshot_path,
};
use crate::app::state::{SoundCue, State};
use crate::app::zoom::CAPTURE_SETTLE_DELAY;

use super::events::scroll_direction_y;
use super::ipc::close_other_screen_tools;
use super::zoom::resolve_initial_cursor_focus;

// ============================================================================
// Color Picker
// ============================================================================

pub fn open_color_picker(
    state: &mut State,
    wait_for_compositor_settle: bool,
    ctx: &mut EffectCtx<'_, Msg>,
) {
    close_other_screen_tools(state, Some("color_picker"), ctx);

    if let Some(old_asset) = state.color_picker.close_immediate() {
        ctx.unload_image(old_asset);
    }

    let (fx, fy) = resolve_initial_cursor_focus(state, ctx);
    let seq = state
        .color_picker
        .begin_capture(state.bar_width, state.screen_height, fx, fy);

    if wait_for_compositor_settle {
        ctx.after(CAPTURE_SETTLE_DELAY, Msg::ColorPickerCaptureNow { seq });
    } else {
        dispatch_color_picker_screencopy(seq, ctx);
    }
}

pub fn handle_color_picker_capture_now(
    state: &mut State,
    seq: u64,
    ctx: &mut EffectCtx<'_, Msg>,
) {
    if state.color_picker.phase != ColorPickerPhase::Capturing
        || state.color_picker.capture_seq != seq
    {
        return;
    }
    dispatch_color_picker_screencopy(seq, ctx);
}

fn dispatch_color_picker_screencopy(seq: u64, ctx: &mut EffectCtx<'_, Msg>) {
    ctx.capture_screen(ScreencopyTarget::Output(Output::Primary), false, move |res| {
        Msg::ColorPickerCaptured {
            seq,
            frame: Arc::new(res.map_err(|e| e.to_string())),
        }
    });
}

pub fn handle_color_picker_captured(
    state: &mut State,
    seq: u64,
    frame: &Arc<std::result::Result<ScreencopyFrame, String>>,
    ctx: &mut EffectCtx<'_, Msg>,
) {
    if state.color_picker.phase != ColorPickerPhase::Capturing
        || state.color_picker.capture_seq != seq
    {
        return;
    }
    match frame.as_ref() {
        Ok(sc) => {
            match ctx.load_rgba_image(
                sc.width,
                sc.height,
                sc.stride as usize,
                true,
                sc.as_rgba_bytes(),
            ) {
                Ok(asset) => {
                    let frame_arc = Arc::new(sc.clone());
                    if let Some(prev) = state.color_picker.finish_capture(seq, asset, frame_arc) {
                        ctx.unload_image(prev);
                    }
                    state.play_sound(SoundCue::MenuPopup, ctx);
                }
                Err(err) => {
                    state.color_picker.fail_capture(seq, err.to_string());
                    state.play_sound(SoundCue::DialogError, ctx);
                }
            }
        }
        Err(err) => {
            state.color_picker.fail_capture(seq, err.clone());
            state.play_sound(SoundCue::DialogError, ctx);
        }
    }
}

pub fn handle_color_picker_pointer_move(state: &mut State, ev: PointerEvent) {
    if !state.color_picker.is_active() {
        return;
    }
    state
        .color_picker
        .set_cursor(ev.position.x.get(), ev.position.y.get());
}

pub fn handle_color_picker_pointer_down(
    state: &mut State,
    ev: PointerEvent,
    ctx: &mut EffectCtx<'_, Msg>,
) {
    if !state.color_picker.is_active() {
        return;
    }
    match ev.button {
        Some(MouseButton::Left) => {
            state
                .color_picker
                .set_cursor(ev.position.x.get(), ev.position.y.get());
            commit_color_pick(state, ctx);
        }
        Some(MouseButton::Right) => {
            state.color_picker.format = state.color_picker.format.toggle();
            state.play_sound(SoundCue::TabChanged, ctx);
        }
        _ => {}
    }
}

pub fn handle_color_picker_scroll(
    state: &mut State,
    sc: ScrollEvent,
    ctx: &EffectCtx<'_, Msg>,
) {
    if !state.color_picker.is_active() {
        return;
    }
    if scroll_direction_y(sc) != 0 {
        state.color_picker.format = state.color_picker.format.toggle();
        state.play_sound(SoundCue::TabChanged, ctx);
    }
}

pub fn handle_color_picker_key(state: &mut State, ev: KeyEvent, ctx: &mut EffectCtx<'_, Msg>) {
    if !state.color_picker.is_active() {
        return;
    }
    match ev.key {
        Key::Escape => {
            handle_close_color_picker(state, ctx);
            return;
        }
        Key::Enter => {
            commit_color_pick(state, ctx);
            return;
        }
        _ => {}
    }
    let ch = ev
        .utf8
        .as_deref()
        .and_then(|s| s.chars().next())
        .map(|c| c.to_ascii_lowercase());
    match ch {
        Some('q') => handle_close_color_picker(state, ctx),
        Some('f' | 'r' | 'h') => {
            state.color_picker.format = state.color_picker.format.toggle();
            state.play_sound(SoundCue::TabChanged, ctx);
        }
        _ => {}
    }
}

fn commit_color_pick(state: &mut State, ctx: &mut EffectCtx<'_, Msg>) {
    let text = state.color_picker.formatted_current();
    let _ = ctx.set_clipboard_text(text.clone());
    state.color_picker.last_copied = Some(text);
    if let Some(asset) = state.color_picker.close_immediate() {
        ctx.unload_image(asset);
    }
    ctx.trim_memory();
    state.play_sound(SoundCue::ToggleOn, ctx);
}

pub fn handle_close_color_picker(state: &mut State, ctx: &mut EffectCtx<'_, Msg>) {
    if !state.color_picker.is_open_or_capturing() {
        return;
    }
    let was_active = state.color_picker.is_active();
    if let Some(asset) = state.color_picker.close_immediate() {
        ctx.unload_image(asset);
    }
    ctx.trim_memory();
    if was_active {
        state.play_sound(SoundCue::MenuPopdown, ctx);
    }
}

// ============================================================================
// Screenshot (Area / Window / Full)
// ============================================================================

/// Queries mapped, visible windows on the active workspace in front-to-back focus order.
fn query_active_workspace_windows(state: &State, ctx: &EffectCtx<'_, Msg>) -> Vec<WindowTarget> {
    let Ok(hc) = ctx.hyprland() else {
        return Vec::new();
    };
    let Ok(mut clients) = hc.clients() else {
        return Vec::new();
    };
    let (ox, oy) = ctx.outputs().first().map_or((0.0, 0.0), |o| {
        (o.logical_position.x.get(), o.logical_position.y.get())
    });
    let w = state.bar_width.max(320.0);
    let h = state.screen_height.max(240.0);

    clients.sort_by_key(|c| c.focus_history_id);

    let mut out = Vec::new();
    for c in clients {
        if !c.mapped || c.hidden || c.size.0 <= 2 || c.size.1 <= 2 {
            continue;
        }
        let on_active = if let Some(ref sp) = state.active_special {
            c.workspace.name == *sp
        } else {
            c.workspace.id == state.active_ws_id
        };
        if !on_active {
            continue;
        }
        let rx = (c.at.0 as f32 - ox).clamp(0.0, w);
        let ry = (c.at.1 as f32 - oy).clamp(0.0, h);
        let rw = (c.size.0 as f32).min(w - rx);
        let rh = (c.size.1 as f32).min(h - ry);
        if rw >= 4.0 && rh >= 4.0 {
            out.push(WindowTarget {
                title: c.title,
                class: c.class,
                x: rx,
                y: ry,
                w: rw,
                h: rh,
            });
        }
    }
    out
}

pub fn open_screenshot(
    state: &mut State,
    mode: ScreenshotMode,
    auto_commit_full: bool,
    wait_for_compositor_settle: bool,
    ctx: &mut EffectCtx<'_, Msg>,
) {
    close_other_screen_tools(state, Some("screenshot"), ctx);

    if let Some(old_asset) = state.screenshot.close_immediate() {
        ctx.unload_image(old_asset);
    }
    if let Some(old_thumb) = state.screenshot.clear_preview_immediate()
        && !state.clipboard.uses_asset(old_thumb.id)
    {
        ctx.unload_image(old_thumb);
    }

    let (fx, fy) = resolve_initial_cursor_focus(state, ctx);
    let windows = query_active_workspace_windows(state, ctx);
    let seq = state.screenshot.begin_capture(ScreenshotStart {
        viewport_w: state.bar_width,
        viewport_h: state.screen_height,
        cursor_x: fx,
        cursor_y: fy,
        mode,
        auto_commit_full,
        windows,
    });

    if wait_for_compositor_settle {
        ctx.after(CAPTURE_SETTLE_DELAY, Msg::ScreenshotCaptureNow { seq });
    } else {
        dispatch_screenshot_screencopy(seq, ctx);
    }
}

pub fn handle_screenshot_capture_now(
    state: &mut State,
    seq: u64,
    ctx: &mut EffectCtx<'_, Msg>,
) {
    if state.screenshot.phase != ScreenshotPhase::Capturing || state.screenshot.capture_seq != seq {
        return;
    }
    dispatch_screenshot_screencopy(seq, ctx);
}

fn dispatch_screenshot_screencopy(seq: u64, ctx: &mut EffectCtx<'_, Msg>) {
    ctx.capture_screen(ScreencopyTarget::Output(Output::Primary), false, move |res| {
        Msg::ScreenshotCaptured {
            seq,
            frame: Arc::new(res.map_err(|e| e.to_string())),
        }
    });
}

pub fn handle_screenshot_captured(
    state: &mut State,
    seq: u64,
    frame: &Arc<std::result::Result<ScreencopyFrame, String>>,
    ctx: &mut EffectCtx<'_, Msg>,
) {
    if state.screenshot.phase != ScreenshotPhase::Capturing || state.screenshot.capture_seq != seq {
        return;
    }
    match frame.as_ref() {
        Ok(sc) => {
            if state.screenshot.auto_commit_full {
                let rect = (
                    0.0,
                    0.0,
                    state.screenshot.viewport_w,
                    state.screenshot.viewport_h,
                );
                commit_screenshot_frame(state, sc, rect, ctx);
                return;
            }
            match ctx.load_rgba_image(
                sc.width,
                sc.height,
                sc.stride as usize,
                true,
                sc.as_rgba_bytes(),
            ) {
                Ok(asset) => {
                    let frame_arc = Arc::new(sc.clone());
                    if let Some(prev) = state.screenshot.finish_capture(seq, asset, frame_arc) {
                        ctx.unload_image(prev);
                    }
                    state.play_sound(SoundCue::MenuPopup, ctx);
                }
                Err(err) => {
                    state.screenshot.fail_capture(seq, err.to_string());
                    state.play_sound(SoundCue::DialogError, ctx);
                }
            }
        }
        Err(err) => {
            state.screenshot.fail_capture(seq, err.clone());
            state.play_sound(SoundCue::DialogError, ctx);
        }
    }
}

pub fn handle_screenshot_set_mode(
    state: &mut State,
    mode: ScreenshotMode,
    ctx: &mut EffectCtx<'_, Msg>,
) {
    if !state.screenshot.is_active() {
        return;
    }
    state.screenshot.mode = mode;
    state.screenshot.drag_start = None;
    state.screenshot.drag_current = None;
    if mode == ScreenshotMode::Full {
        handle_screenshot_commit_full(state, ctx);
    } else {
        state.play_sound(SoundCue::TabChanged, ctx);
    }
}

pub fn handle_screenshot_pointer_down(
    state: &mut State,
    ev: PointerEvent,
    ctx: &mut EffectCtx<'_, Msg>,
) {
    if !state.screenshot.is_active() {
        return;
    }
    match ev.button {
        Some(MouseButton::Right) => {
            handle_close_screenshot(state, ctx);
        }
        Some(MouseButton::Left) => {
            let x = ev.position.x.get().clamp(0.0, state.screenshot.viewport_w);
            let y = ev.position.y.get().clamp(0.0, state.screenshot.viewport_h);
            state.screenshot.cursor_pos = (x, y);
            match state.screenshot.mode {
                ScreenshotMode::Area => {
                    state.screenshot.drag_start = Some((x, y));
                    state.screenshot.drag_current = Some((x, y));
                }
                ScreenshotMode::Window => {
                    if let Some(rect) = state.screenshot.active_selection_rect() {
                        commit_current_screenshot(state, rect, ctx);
                    }
                }
                ScreenshotMode::Full => {
                    handle_screenshot_commit_full(state, ctx);
                }
            }
        }
        _ => {}
    }
}

pub fn handle_screenshot_pointer_move(state: &mut State, ev: PointerEvent) {
    if !state.screenshot.is_active() {
        return;
    }
    let x = ev.position.x.get().clamp(0.0, state.screenshot.viewport_w);
    let y = ev.position.y.get().clamp(0.0, state.screenshot.viewport_h);
    state.screenshot.cursor_pos = (x, y);
    if state.screenshot.drag_start.is_some() {
        state.screenshot.drag_current = Some((x, y));
    }
}

pub fn handle_screenshot_pointer_up(
    state: &mut State,
    ev: PointerEvent,
    ctx: &mut EffectCtx<'_, Msg>,
) {
    if !state.screenshot.is_active() || state.screenshot.mode != ScreenshotMode::Area {
        return;
    }
    if state.screenshot.drag_start.is_none() {
        return;
    }
    let x = ev.position.x.get().clamp(0.0, state.screenshot.viewport_w);
    let y = ev.position.y.get().clamp(0.0, state.screenshot.viewport_h);
    state.screenshot.drag_current = Some((x, y));

    if let Some(rect) = state.screenshot.active_selection_rect() {
        commit_current_screenshot(state, rect, ctx);
    } else {
        state.screenshot.drag_start = None;
        state.screenshot.drag_current = None;
    }
}

pub fn handle_screenshot_commit_full(state: &mut State, ctx: &mut EffectCtx<'_, Msg>) {
    if !state.screenshot.is_active() {
        return;
    }
    let rect = (
        0.0,
        0.0,
        state.screenshot.viewport_w,
        state.screenshot.viewport_h,
    );
    commit_current_screenshot(state, rect, ctx);
}

pub fn handle_screenshot_key(state: &mut State, ev: KeyEvent, ctx: &mut EffectCtx<'_, Msg>) {
    if !state.screenshot.is_active() {
        return;
    }
    match ev.key {
        Key::Escape => {
            handle_close_screenshot(state, ctx);
            return;
        }
        Key::Enter => {
            if let Some(rect) = state.screenshot.active_selection_rect() {
                commit_current_screenshot(state, rect, ctx);
            } else {
                handle_screenshot_commit_full(state, ctx);
            }
            return;
        }
        _ => {}
    }

    let ch = ev
        .utf8
        .as_deref()
        .and_then(|s| s.chars().next())
        .map(|c| c.to_ascii_lowercase());
    match ch {
        Some('q') => handle_close_screenshot(state, ctx),
        Some('1' | 'a') => handle_screenshot_set_mode(state, ScreenshotMode::Area, ctx),
        Some('2' | 'w') => handle_screenshot_set_mode(state, ScreenshotMode::Window, ctx),
        Some('3' | 'f') => handle_screenshot_set_mode(state, ScreenshotMode::Full, ctx),
        Some('c') => {
            state.screenshot.copy_to_clipboard = !state.screenshot.copy_to_clipboard;
            if !state.screenshot.copy_to_clipboard && !state.screenshot.save_to_file {
                state.screenshot.save_to_file = true;
            }
        }
        Some('s') => {
            state.screenshot.save_to_file = !state.screenshot.save_to_file;
            if !state.screenshot.copy_to_clipboard && !state.screenshot.save_to_file {
                state.screenshot.copy_to_clipboard = true;
            }
        }
        _ => {}
    }
}

fn commit_current_screenshot(
    state: &mut State,
    rect: (f32, f32, f32, f32),
    ctx: &mut EffectCtx<'_, Msg>,
) {
    let Some(frame) = state.screenshot.frame.clone() else {
        return;
    };
    commit_screenshot_frame(state, &frame, rect, ctx);
}

fn commit_screenshot_frame(
    state: &mut State,
    frame: &ScreencopyFrame,
    rect: (f32, f32, f32, f32),
    ctx: &mut EffectCtx<'_, Msg>,
) {
    let copy = state.screenshot.copy_to_clipboard;
    let save = state.screenshot.save_to_file;
    let vw = state.screenshot.viewport_w;
    let vh = state.screenshot.viewport_h;

    match crop_and_encode_png(frame, rect, vw, vh) {
        Ok((png_bytes, pw, ph)) => {
            let mut saved_path = None;
            if save {
                let path = default_screenshot_path(LocalDateTime::now());
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                if let Err(err) = std::fs::write(&path, &png_bytes) {
                    state.screenshot.last_error = Some(err.to_string());
                } else {
                    state.screenshot.last_saved_path = Some(path.clone());
                    saved_path = Some(path);
                }
            }
            if copy {
                let _ = ctx.set_clipboard(
                    ClipboardSelection::Clipboard,
                    vec![(String::from("image/png"), png_bytes.clone())],
                );
            }
            if let Some(asset) = state.screenshot.close_immediate() {
                ctx.unload_image(asset);
            }
            if let Ok(thumb) = ctx.load_image_bytes(&png_bytes)
                && let Some(old_thumb) = state.screenshot.show_preview(PreviewInit {
                    thumb,
                    width_px: pw,
                    height_px: ph,
                    saved_path,
                    png_bytes,
                    copied: copy,
                })
                && !state.clipboard.uses_asset(old_thumb.id)
            {
                ctx.unload_image(old_thumb);
            }
            ctx.trim_memory();
            state.play_sound(SoundCue::ToggleOn, ctx);
        }
        Err(err) => {
            state.screenshot.last_error = Some(err);
            if let Some(asset) = state.screenshot.close_immediate() {
                ctx.unload_image(asset);
            }
            ctx.trim_memory();
            state.play_sound(SoundCue::DialogError, ctx);
        }
    }
}

pub fn handle_screenshot_preview_open(state: &mut State, ctx: &EffectCtx<'_, Msg>) {
    let Some(preview) = state.screenshot.preview.as_ref() else {
        return;
    };
    let target_path = if let Some(ref saved) = preview.saved_path {
        Some(saved.clone())
    } else {
        let tmp = temp_open_image_path("screenshot-preview", "png");
        std::fs::write(&tmp, &preview.png_bytes).ok().map(|()| tmp)
    };
    if let Some(path) = target_path {
        let _ = spawn_detached("xdg-open", [path.as_os_str()]);
    }
    state.screenshot.dismiss_preview();
    state.play_sound(SoundCue::TabChanged, ctx);
}

pub fn handle_screenshot_preview_folder(state: &mut State, ctx: &EffectCtx<'_, Msg>) {
    if let Some(dir) = state
        .screenshot
        .preview
        .as_ref()
        .and_then(|p| p.saved_path.as_deref())
        .and_then(std::path::Path::parent)
    {
        let _ = spawn_detached("xdg-open", [dir.as_os_str()]);
        state.screenshot.dismiss_preview();
        state.play_sound(SoundCue::TabChanged, ctx);
    }
}

pub fn handle_screenshot_preview_copy(state: &mut State, ctx: &mut EffectCtx<'_, Msg>) {
    if let Some(preview) = state.screenshot.preview.as_mut() {
        let _ = ctx.set_clipboard(
            ClipboardSelection::Clipboard,
            vec![(String::from("image/png"), preview.png_bytes.clone())],
        );
        preview.copied = true;
        state.play_sound(SoundCue::ToggleOn, ctx);
    }
}

pub fn handle_screenshot_preview_delete(state: &mut State, ctx: &EffectCtx<'_, Msg>) {
    let path = state
        .screenshot
        .preview
        .as_mut()
        .and_then(|p| p.saved_path.take());
    state.screenshot.delete_saved_file(path);
    state.screenshot.dismiss_preview();
    state.play_sound(SoundCue::ToggleOff, ctx);
}

pub fn handle_screenshot_preview_dismiss(state: &mut State, ctx: &EffectCtx<'_, Msg>) {
    if state.screenshot.preview.is_some() {
        state.screenshot.dismiss_preview();
        state.play_sound(SoundCue::MenuPopdown, ctx);
    }
}

pub fn handle_screenshot_preview_timeout(state: &mut State, seq: u64) {
    if state
        .screenshot
        .preview
        .as_ref()
        .is_some_and(|p| p.seq == seq && !p.hovered)
    {
        state.screenshot.dismiss_preview();
    }
}

pub fn handle_close_screenshot(state: &mut State, ctx: &mut EffectCtx<'_, Msg>) {
    if !state.screenshot.is_open_or_capturing() {
        return;
    }
    let was_active = state.screenshot.is_active();
    if let Some(asset) = state.screenshot.close_immediate() {
        ctx.unload_image(asset);
    }
    ctx.trim_memory();
    if was_active {
        state.play_sound(SoundCue::MenuPopdown, ctx);
    }
}
