//! Handlers for Toolbox launching and the animated Screen Magnifier (Snapshot + Hyprland Live).

use std::sync::Arc;
use std::time::Instant;

use waywi::prelude::*;

use crate::app::msg::Msg;
use crate::app::screenshot::ScreenshotMode;
use crate::app::state::{SoundCue, State};
use crate::app::toolbox::ToolboxTool;
use crate::app::zoom::{CAPTURE_SETTLE_DELAY, DEFAULT_ZOOM, MIN_ZOOM, ZoomMode, ZoomPhase};

use super::annotate::handle_toggle_annotate;
use super::events::scroll_direction_y;
use super::ipc::close_other_screen_tools;
use super::tools::{open_color_picker, open_screenshot};

/// Resolves the current global cursor position relative to the primary Wayland output.
pub fn resolve_initial_cursor_focus(state: &State, ctx: &EffectCtx<'_, Msg>) -> (f32, f32) {
    let w = state.bar_width.max(320.0);
    let h = state.screen_height.max(240.0);
    let (ox, oy) = ctx.outputs().first().map_or((0.0, 0.0), |o| {
        (o.logical_position.x.get(), o.logical_position.y.get())
    });
    if let Ok(hc) = ctx.hyprland()
        && let Ok(pos) = hc.cursor_pos()
    {
        return (
            (pos.x as f32 - ox).clamp(0.0, w),
            (pos.y as f32 - oy).clamp(0.0, h),
        );
    }
    (w * 0.5, h * 0.5)
}

/// True when another fullscreen screen tool is currently mounted and must unmount before capture.
pub fn any_screen_tool_mounted(state: &State) -> bool {
    state.annotate.is_active()
        || state.zoom.should_mount_surface()
        || state.color_picker.should_mount_surface()
        || state.screenshot.should_mount_surface()
}

pub fn handle_toolbox_launch(
    state: &mut State,
    tool: ToolboxTool,
    ctx: &mut EffectCtx<'_, Msg>,
) {
    let was_qm_visible = state.quick_menu.is_popup_visible();
    match tool {
        ToolboxTool::Annotate => {
            state.quick_menu.close_popup();
            state.settings_ui.unfocus_inputs();
            state.settings_ui.open_dropdown = None;
            if !state.hovered.is_clock_group() {
                state.clock_menu_anim.set_open(false);
            }
            handle_toggle_annotate(state, ctx);
        }
        ToolboxTool::Zoom => {
            hide_quick_menu_for_capture(state, ctx);
            open_zoom(state, was_qm_visible, None, ctx);
        }
        ToolboxTool::ColorPicker => {
            hide_quick_menu_for_capture(state, ctx);
            open_color_picker(state, was_qm_visible, ctx);
        }
        ToolboxTool::Screenshot => {
            hide_quick_menu_for_capture(state, ctx);
            open_screenshot(state, ScreenshotMode::Area, false, was_qm_visible, ctx);
        }
    }
}

/// Hides Quick Menu instantaneously (with sprite pixel eviction) so it never ghosts into screencopy.
pub fn hide_quick_menu_for_capture(state: &mut State, ctx: &mut EffectCtx<'_, Msg>) {
    state.quick_menu.close_popup_immediate();
    state.quick_menu.evict_sprite_pixels(ctx);
    state.settings_ui.unfocus_inputs();
    state.settings_ui.open_dropdown = None;
    if !state.hovered.is_clock_group() {
        state.clock_menu_anim.set_open(false);
    }
}

pub fn open_zoom(
    state: &mut State,
    wait_for_compositor_settle: bool,
    restore: Option<(f32, f32, f32)>,
    ctx: &mut EffectCtx<'_, Msg>,
) {
    // If transitioning out of Live mode, ensure compositor zoom resets cleanly before screencopy.
    if state.zoom.is_live() {
        reset_hyprland_live_zoom(ctx);
    }
    close_other_screen_tools(state, Some("zoom"), ctx);

    if let Some(old_asset) = state.zoom.snapshot.take() {
        ctx.unload_image(old_asset);
    }

    let (fx, fy) = resolve_initial_cursor_focus(state, ctx);
    let seq = state.zoom.begin_capture(
        state.bar_width,
        state.screen_height,
        fx,
        fy,
        restore,
    );

    if wait_for_compositor_settle {
        ctx.after(CAPTURE_SETTLE_DELAY, Msg::ZoomCaptureNow { seq });
    } else {
        dispatch_screencopy(seq, ctx);
    }
}

/// Enters Hyprland native Live compositor zoom (`cursor:zoom_factor`), animated smoothly via VSync.
pub fn open_zoom_live(state: &mut State, target_zoom: f32, ctx: &mut EffectCtx<'_, Msg>) {
    close_other_screen_tools(state, Some("zoom"), ctx);

    if let Ok(hc) = ctx.hyprland() {
        let _ = hc.keyword("cursor:zoom_detached_camera", "true");
        let _ = hc.keyword("cursor:zoom_rigid", "false");
        let _ = hc.keyword(
            "cursor:zoom_disable_aa",
            if state.zoom.crisp_pixels { "true" } else { "false" },
        );
    }

    if let Some(old_asset) = state.zoom.begin_live(
        state.bar_width,
        state.screen_height,
        target_zoom,
        Instant::now(),
    ) {
        ctx.unload_image(old_asset);
    }
    state.play_sound(SoundCue::ToggleOn, ctx);
}

pub fn reset_hyprland_live_zoom(ctx: &EffectCtx<'_, Msg>) {
    if let Ok(hc) = ctx.hyprland() {
        let _ = hc.keyword("cursor:zoom_factor", "1.0");
        let _ = hc.keyword("cursor:zoom_disable_aa", "false");
    }
}

pub fn handle_zoom_toggle_live(state: &mut State, ctx: &mut EffectCtx<'_, Msg>) {
    if !state.zoom.is_open_or_capturing() {
        open_zoom_live(state, DEFAULT_ZOOM, ctx);
        return;
    }
    if !state.zoom.is_active() {
        return;
    }
    match state.zoom.mode {
        ZoomMode::Snapshot => {
            let target = state.zoom.target_zoom.max(DEFAULT_ZOOM.min(2.0));
            open_zoom_live(state, target, ctx);
        }
        ZoomMode::Live => {
            let saved_zoom = state.zoom.target_zoom.max(DEFAULT_ZOOM);
            reset_hyprland_live_zoom(ctx);
            let (fx, fy) = resolve_initial_cursor_focus(state, ctx);
            open_zoom(state, true, Some((saved_zoom, fx, fy)), ctx);
            state.play_sound(SoundCue::ToggleOff, ctx);
        }
    }
}

pub fn handle_zoom_capture_now(state: &mut State, seq: u64, ctx: &mut EffectCtx<'_, Msg>) {
    if state.zoom.phase != ZoomPhase::Capturing || state.zoom.capture_seq != seq {
        return;
    }
    dispatch_screencopy(seq, ctx);
}

fn dispatch_screencopy(seq: u64, ctx: &mut EffectCtx<'_, Msg>) {
    ctx.capture_screen(ScreencopyTarget::Output(Output::Primary), false, move |res| {
        Msg::ZoomCaptured {
            seq,
            frame: Arc::new(res.map_err(|e| e.to_string())),
        }
    });
}

pub fn handle_zoom_captured(
    state: &mut State,
    seq: u64,
    frame: &Arc<std::result::Result<ScreencopyFrame, String>>,
    ctx: &mut EffectCtx<'_, Msg>,
) {
    if state.zoom.phase != ZoomPhase::Capturing || state.zoom.capture_seq != seq {
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
                    let is_restore = state.zoom.pending_restore_zoom.is_some();
                    if let Some(prev) = state.zoom.finish_capture(seq, asset, Instant::now()) {
                        ctx.unload_image(prev);
                    }
                    if !is_restore {
                        state.play_sound(SoundCue::MenuPopup, ctx);
                    }
                }
                Err(err) => {
                    state.zoom.fail_capture(seq, err.to_string());
                    state.play_sound(SoundCue::DialogError, ctx);
                }
            }
        }
        Err(err) => {
            state.zoom.fail_capture(seq, err.clone());
            state.play_sound(SoundCue::DialogError, ctx);
        }
    }
}

pub fn handle_close_zoom(state: &mut State, ctx: &mut EffectCtx<'_, Msg>) {
    if !state.zoom.is_open_or_capturing() {
        return;
    }
    let was_active = state.zoom.is_active();
    let was_live = state.zoom.is_live();
    state.zoom.begin_close(Instant::now());
    if !state.zoom.should_mount_surface() {
        if was_live {
            reset_hyprland_live_zoom(ctx);
        }
        if let Some(asset) = state.zoom.close_immediate() {
            ctx.unload_image(asset);
        }
    }
    if was_active {
        state.play_sound(SoundCue::MenuPopdown, ctx);
    }
}

pub fn handle_zoom_pointer_down(
    state: &mut State,
    ev: PointerEvent,
    ctx: &mut EffectCtx<'_, Msg>,
) {
    if !state.zoom.is_active() || state.zoom.mode != ZoomMode::Snapshot {
        return;
    }
    match ev.button {
        Some(MouseButton::Left) => {
            state.zoom.begin_drag(ev.position.x.get(), ev.position.y.get());
        }
        Some(MouseButton::Right) => {
            handle_close_zoom(state, ctx);
        }
        Some(MouseButton::Middle) => {
            state.zoom.reset_view(Instant::now());
            state.play_sound(SoundCue::TabChanged, ctx);
        }
        _ => {}
    }
}

pub fn handle_zoom_pointer_move(state: &mut State, ev: PointerEvent) {
    if !state.zoom.is_active() || state.zoom.mode != ZoomMode::Snapshot {
        return;
    }
    state
        .zoom
        .on_pointer_move(ev.position.x.get(), ev.position.y.get(), Instant::now());
}

pub fn handle_zoom_pointer_up(state: &mut State) {
    state.zoom.end_drag();
}

pub fn handle_zoom_scroll(state: &mut State, sc: ScrollEvent) {
    if !state.zoom.is_active() {
        return;
    }
    let dir = scroll_direction_y(sc);
    if dir != 0 {
        let step = if dir < 0 { 1 } else { -1 };
        state.zoom.step_zoom(
            step,
            Some((sc.position.x.get(), sc.position.y.get())),
            Instant::now(),
        );
    }
}

pub fn handle_zoom_step(state: &mut State, delta: i32, ctx: &mut EffectCtx<'_, Msg>) {
    if !state.zoom.is_open_or_capturing() {
        if delta > 0 {
            let had_tool = any_screen_tool_mounted(state);
            open_zoom(state, had_tool, None, ctx);
        }
        return;
    }
    if state.zoom.is_active() {
        state.zoom.step_zoom(delta, None, Instant::now());
    }
}

pub fn handle_zoom_reset(state: &mut State, ctx: &EffectCtx<'_, Msg>) {
    if state.zoom.is_active() {
        state.zoom.reset_view(Instant::now());
        state.play_sound(SoundCue::TabChanged, ctx);
    }
}

pub fn handle_zoom_toggle_filter(state: &mut State, ctx: &EffectCtx<'_, Msg>) {
    if state.zoom.is_active() {
        state.zoom.crisp_pixels = !state.zoom.crisp_pixels;
        if state.zoom.mode == ZoomMode::Live
            && let Ok(hc) = ctx.hyprland()
        {
            let _ = hc.keyword(
                "cursor:zoom_disable_aa",
                if state.zoom.crisp_pixels { "true" } else { "false" },
            );
        }
        state.play_sound(
            if state.zoom.crisp_pixels {
                SoundCue::ToggleOn
            } else {
                SoundCue::ToggleOff
            },
            ctx,
        );
    }
}

pub fn handle_zoom_recapture(state: &mut State, ctx: &mut EffectCtx<'_, Msg>) {
    if !state.zoom.is_active() || state.zoom.mode != ZoomMode::Snapshot {
        return;
    }
    let restore = Some((
        state.zoom.target_zoom,
        state.zoom.target_cam_x,
        state.zoom.target_cam_y,
    ));
    open_zoom(state, true, restore, ctx);
    state.play_sound(SoundCue::TabChanged, ctx);
}

pub fn handle_zoom_key(state: &mut State, ev: KeyEvent, ctx: &mut EffectCtx<'_, Msg>) {
    if !state.zoom.is_active() {
        return;
    }

    match ev.key {
        Key::Escape => {
            handle_close_zoom(state, ctx);
            return;
        }
        Key::Left => {
            pan_by_key(state, -120.0, 0.0);
            return;
        }
        Key::Right => {
            pan_by_key(state, 120.0, 0.0);
            return;
        }
        Key::Up => {
            pan_by_key(state, 0.0, -120.0);
            return;
        }
        Key::Down => {
            pan_by_key(state, 0.0, 120.0);
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
        Some('q') => handle_close_zoom(state, ctx),
        Some('+' | '=') => state.zoom.step_zoom(1, None, Instant::now()),
        Some('-' | '_') => state.zoom.step_zoom(-1, None, Instant::now()),
        Some('0' | ' ') => handle_zoom_reset(state, ctx),
        Some('p') => handle_zoom_toggle_filter(state, ctx),
        Some('l') => handle_zoom_toggle_live(state, ctx),
        Some('r') => handle_zoom_recapture(state, ctx),
        _ => {}
    }
}

fn pan_by_key(state: &mut State, dx_screen: f32, dy_screen: f32) {
    if state.zoom.mode != ZoomMode::Snapshot {
        return;
    }
    let inv = 1.0 / state.zoom.zoom.max(MIN_ZOOM);
    let (nx, ny) = crate::app::zoom::clamp_camera(
        state.zoom.target_cam_x + dx_screen * inv,
        state.zoom.target_cam_y + dy_screen * inv,
        state.zoom.target_zoom,
        state.zoom.viewport_w,
        state.zoom.viewport_h,
    );
    state.zoom.target_cam_x = nx;
    state.zoom.target_cam_y = ny;
    state.zoom.last_tick.get_or_insert_with(Instant::now);
}
