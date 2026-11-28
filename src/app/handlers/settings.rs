//! Live settings mutation, dynamic zero-idle-RAM wallpaper loading, and TOML config persistence.

use waywi::prelude::*;

use crate::app::config::{
    DesktopPoseMode, MemoryProfile, MetricPlacement, RendererMode, RightMetricId, SettingStepperId,
    SettingToggleId, SettingsDropdown, SettingsSection, WallpaperConfig, WallpaperFit,
    WidgetConfig, expand_tilde_path,
};
use crate::app::handlers::events::scroll_direction_y;
use crate::app::handlers::init::load_embedded_wallpaper;
use crate::app::state::{SoundCue, State};
use crate::widgets::quick_menu::settings::{
    settings_list_content_height, settings_list_view_h, settings_rail_content_height,
    settings_rail_view_h,
};

const SCROLL_STEP_PX: f32 = 28.0;

/// Loads the configured wallpaper (`AssetSource::Path` for custom files or embedded fallback) and
/// marks `AssetRetention::ReclaimAfterPresent` so idle RAM usage stays at 0 bytes after commit.
pub fn load_configured_wallpaper(
    cfg: &WallpaperConfig,
    ctx: &mut EffectCtx<'_, crate::app::msg::Msg>,
) -> std::result::Result<Option<ImageAsset>, String> {
    if !cfg.enabled {
        return Ok(None);
    }
    let trimmed = cfg.path.trim();
    if trimmed.is_empty() {
        return load_embedded_wallpaper(ctx)
            .map(Some)
            .ok_or_else(|| String::from("Built-in wallpaper failed to decode"));
    }
    let resolved = expand_tilde_path(trimmed);
    if !resolved.is_file() {
        return Err(format!("Not found: {}", resolved.display()));
    }
    let wp = ctx
        .load_image(&resolved)
        .map_err(|e| format!("Load failed: {e}"))?;
    let _ = ctx.set_image_retention(wp, AssetRetention::ReclaimAfterPresent);
    Ok(Some(wp))
}

fn replace_wallpaper_asset(state: &mut State, next: Option<ImageAsset>, ctx: &mut EffectCtx<'_, crate::app::msg::Msg>) {
    if let Some(prev) = state.wallpaper_asset
        && next.is_none_or(|n| n.id != prev.id)
    {
        let _ = ctx.evict_image_pixels(prev);
    }
    state.wallpaper_asset = next;
}

pub fn handle_toggle_settings(state: &mut State, ctx: &EffectCtx<'_, crate::app::msg::Msg>) {
    state.quick_menu.toggle_settings_menu();
    if state.quick_menu.settings_open {
        state.settings_ui.wallpaper_error = None;
        state.settings_ui.unfocus_inputs();
        state.play_sound(SoundCue::ToggleOn, ctx);
    } else {
        state.settings_ui.open_dropdown = None;
        state.settings_ui.unfocus_inputs();
        state.play_sound(SoundCue::ToggleOff, ctx);
    }
}

pub fn handle_select_section(state: &mut State, section: SettingsSection, ctx: &EffectCtx<'_, crate::app::msg::Msg>) {
    if state.settings_ui.active_section != section {
        state.settings_ui.select_section(section);
        state.play_sound(SoundCue::TabChanged, ctx);
    }
}

pub fn handle_rail_scroll(state: &mut State, sc: ScrollEvent) {
    let step = scroll_direction_y(sc);
    if step == 0 {
        return;
    }
    let max_scroll = (settings_rail_content_height() - settings_rail_view_h(state)).max(0.0);
    state.settings_ui.rail_scroll_y =
        (state.settings_ui.rail_scroll_y + (step as f32) * SCROLL_STEP_PX).clamp(0.0, max_scroll);
}

pub fn handle_list_scroll(state: &mut State, sc: ScrollEvent) {
    let step = scroll_direction_y(sc);
    if step == 0 {
        return;
    }
    let content_h = settings_list_content_height(state);
    let max_scroll = (content_h - settings_list_view_h(state)).max(0.0);
    state.settings_ui.list_scroll_y =
        (state.settings_ui.list_scroll_y + (step as f32) * SCROLL_STEP_PX).clamp(0.0, max_scroll);
}

pub fn handle_toggle_setting(state: &mut State, id: SettingToggleId, ctx: &mut EffectCtx<'_, crate::app::msg::Msg>) {
    state.settings_ui.unfocus_inputs();
    let new_val = match id {
        SettingToggleId::Clock24h => {
            state.config.general.clock_24h = !state.config.general.clock_24h;
            state.config.general.clock_24h
        }
        SettingToggleId::ClockAltDefault => {
            state.config.general.clock_alt_default = !state.config.general.clock_alt_default;
            state.clock_alt = state.config.general.clock_alt_default;
            state.config.general.clock_alt_default
        }
        SettingToggleId::ForegroundBooster => {
            state.config.general.foreground_booster = !state.config.general.foreground_booster;
            if state.config.general.foreground_booster {
                state.booster.on_focus_change(
                    state.window_info.cgroup.as_ref(),
                    state.memory.as_ref(),
                    ctx,
                );
            } else {
                state.booster.clear_active(ctx);
            }
            state.config.general.foreground_booster
        }
        SettingToggleId::XdgSoundsEnabled => {
            state.config.audio.xdg_sounds_enabled = !state.config.audio.xdg_sounds_enabled;
            state.sounds.apply_config(&state.config.audio);
            state.config.audio.xdg_sounds_enabled
        }
        SettingToggleId::UiFeedbackSounds => {
            state.config.audio.ui_feedback_sounds = !state.config.audio.ui_feedback_sounds;
            state.sounds.apply_config(&state.config.audio);
            state.config.audio.ui_feedback_sounds
        }
        SettingToggleId::NotificationSounds => {
            state.config.audio.notification_sounds = !state.config.audio.notification_sounds;
            state.sounds.apply_config(&state.config.audio);
            state.config.audio.notification_sounds
        }
        SettingToggleId::WallpaperEnabled => {
            state.config.wallpaper.enabled = !state.config.wallpaper.enabled;
            state.settings_ui.wallpaper_error = None;
            if state.config.wallpaper.enabled {
                match load_configured_wallpaper(&state.config.wallpaper, ctx) {
                    Ok(asset) => replace_wallpaper_asset(state, asset, ctx),
                    Err(err) => {
                        state.settings_ui.wallpaper_error = Some(err);
                        let fallback = load_embedded_wallpaper(ctx);
                        replace_wallpaper_asset(state, fallback, ctx);
                    }
                }
            } else {
                replace_wallpaper_asset(state, None, ctx);
            }
            state.config.wallpaper.enabled
        }
        SettingToggleId::SpriteDesktopEnabled => {
            state.config.sprite.desktop_enabled = !state.config.sprite.desktop_enabled;
            if state.config.sprite.desktop_enabled {
                if let DesktopPoseMode::Pinned(idx) = state.config.sprite.desktop_pose {
                    state.desktop_sprite.active_idx = idx;
                }
                state.desktop_sprite.ensure_loaded(ctx);
            } else {
                state.desktop_sprite.unload_and_evict(ctx);
            }
            state.config.sprite.desktop_enabled
        }
        SettingToggleId::SpriteRotateOnWorkspace => {
            state.config.sprite.rotate_on_workspace = !state.config.sprite.rotate_on_workspace;
            state.config.sprite.rotate_on_workspace
        }
        SettingToggleId::SpriteMouthOnNotification => {
            state.config.sprite.mouth_on_notification = !state.config.sprite.mouth_on_notification;
            state.config.sprite.mouth_on_notification
        }
        SettingToggleId::WidgetSysInfo => {
            state.config.widgets.show_sysinfo = !state.config.widgets.show_sysinfo;
            state.config.widgets.show_sysinfo
        }
        SettingToggleId::WidgetWorkspaces => {
            state.config.widgets.show_workspaces = !state.config.widgets.show_workspaces;
            state.config.widgets.show_workspaces
        }
        SettingToggleId::WidgetWindowInfo => {
            state.config.widgets.show_window_info = !state.config.widgets.show_window_info;
            state.config.widgets.show_window_info
        }
        SettingToggleId::WidgetNotifications => {
            state.config.widgets.show_notifications = !state.config.widgets.show_notifications;
            if !state.config.widgets.show_notifications {
                state.notifications.close_center();
            }
            state.config.widgets.show_notifications
        }
        SettingToggleId::WidgetMpris => {
            state.config.widgets.show_mpris = !state.config.widgets.show_mpris;
            if !state.config.widgets.show_mpris {
                state.mpris.close_popup();
            }
            state.config.widgets.show_mpris
        }
        SettingToggleId::WidgetTray => {
            state.config.widgets.show_tray = !state.config.widgets.show_tray;
            if !state.config.widgets.show_tray {
                state.tray_menu = None;
            }
            state.config.widgets.show_tray
        }
        SettingToggleId::WidgetVisualizer => {
            state.config.widgets.show_visualizer = !state.config.widgets.show_visualizer;
            if !state.config.widgets.show_visualizer {
                state.visualizer.reset_immediate();
            }
            state.config.widgets.show_visualizer
        }
    };

    state.save_config();
    let cue = if new_val {
        SoundCue::ToggleOn
    } else {
        SoundCue::ToggleOff
    };
    state.play_sound(cue, ctx);
}

pub fn handle_cycle_metric_placement(
    state: &mut State,
    id: RightMetricId,
    ctx: &EffectCtx<'_, crate::app::msg::Msg>,
) {
    state.settings_ui.unfocus_inputs();
    let next = state.config.widgets.cycle_metric_placement(id);
    if next == MetricPlacement::Hidden {
        match id {
            RightMetricId::Audio => state.audio_ui.close_popup(),
            RightMetricId::Disk => state.disk_ui.close_popup(),
            RightMetricId::Bluetooth => state.bt_ui.close_popup(),
            RightMetricId::Cpu
            | RightMetricId::Memory
            | RightMetricId::Gpu
            | RightMetricId::Network
                if state.telemetry_graph.active.metric_id() == id =>
            {
                state.telemetry_graph.close();
            }
            _ => {}
        }
    }
    if state.config.widgets.drawer_metrics().is_empty() {
        state.drawer_anim.snap_closed();
        state.drawer_leave_deadline = None;
    }
    state.save_config();
    let cue = match next {
        MetricPlacement::Bar | MetricPlacement::Drawer => SoundCue::ToggleOn,
        MetricPlacement::Hidden => SoundCue::ToggleOff,
    };
    state.play_sound(cue, ctx);
}

pub fn handle_move_metric(
    state: &mut State,
    id: RightMetricId,
    delta: i32,
    ctx: &EffectCtx<'_, crate::app::msg::Msg>,
) {
    state.settings_ui.unfocus_inputs();
    if state.config.widgets.move_metric(id, delta) {
        state.save_config();
        state.play_sound(SoundCue::TabChanged, ctx);
    }
}

pub fn handle_step_setting(
    state: &mut State,
    id: SettingStepperId,
    delta: i32,
    ctx: &EffectCtx<'_, crate::app::msg::Msg>,
) {
    state.settings_ui.unfocus_inputs();
    let changed = match id {
        SettingStepperId::PowerCountdownSecs => {
            let cur = i32::from(state.config.general.power_countdown_secs);
            let next = (cur + delta).clamp(0, 10) as u8;
            let diff = next != state.config.general.power_countdown_secs;
            state.config.general.power_countdown_secs = next;
            diff
        }
        SettingStepperId::SoundVolumePct => {
            let cur = i32::from(state.config.audio.sound_volume_pct);
            let next = (cur + delta * 10).clamp(10, 150) as u16;
            let diff = next != state.config.audio.sound_volume_pct;
            state.config.audio.sound_volume_pct = next;
            state.sounds.apply_config(&state.config.audio);
            diff
        }
        SettingStepperId::DesktopScalePct => {
            let cur = i32::from(state.config.sprite.desktop_scale_pct);
            let next = (cur + delta * 5).clamp(70, 115) as u16;
            let diff = next != state.config.sprite.desktop_scale_pct;
            state.config.sprite.desktop_scale_pct = next;
            diff
        }
        SettingStepperId::SpriteRotateIntervalSecs => {
            let cur = i32::from(state.config.sprite.rotate_interval_secs);
            let next = (cur + delta * 30).clamp(30, 600) as u16;
            let diff = next != state.config.sprite.rotate_interval_secs;
            state.config.sprite.rotate_interval_secs = next;
            diff
        }
    };

    if changed {
        state.save_config();
        let cue = if id == SettingStepperId::SoundVolumePct {
            SoundCue::VolumeChange
        } else {
            SoundCue::TabChanged
        };
        state.play_sound(cue, ctx);
    }
}

pub fn handle_toggle_dropdown(
    state: &mut State,
    dropdown: SettingsDropdown,
    ctx: &EffectCtx<'_, crate::app::msg::Msg>,
) {
    state.settings_ui.unfocus_inputs();
    if state.settings_ui.open_dropdown == Some(dropdown) {
        state.settings_ui.open_dropdown = None;
        state.play_sound(SoundCue::MenuPopdown, ctx);
    } else {
        state.settings_ui.open_dropdown = Some(dropdown);
        state.play_sound(SoundCue::MenuPopup, ctx);
    }
    // Keep scroll offset clamped whenever dropdown height changes.
    let max_scroll = (settings_list_content_height(state) - settings_list_view_h(state)).max(0.0);
    state.settings_ui.list_scroll_y = state.settings_ui.list_scroll_y.clamp(0.0, max_scroll);
}

pub fn handle_select_dropdown(
    state: &mut State,
    dropdown: SettingsDropdown,
    index: usize,
    ctx: &mut EffectCtx<'_, crate::app::msg::Msg>,
) {
    state.settings_ui.open_dropdown = None;
    match dropdown {
        SettingsDropdown::Renderer => {
            state.config.general.renderer = RendererMode::from_index(index);
        }
        SettingsDropdown::MemoryProfile => {
            state.config.general.memory_profile = MemoryProfile::from_index(index);
        }
        SettingsDropdown::WallpaperFit => {
            state.config.wallpaper.fit = WallpaperFit::from_index(index);
        }
        SettingsDropdown::DesktopPose => {
            let mode = DesktopPoseMode::from_index(index);
            state.config.sprite.desktop_pose = mode;
            if let DesktopPoseMode::Pinned(pose_idx) = mode {
                state.desktop_sprite.set_pose_instant(pose_idx, ctx);
            }
        }
        SettingsDropdown::QuickMenuPose => {
            let pose = index % 3;
            state.config.sprite.quick_menu_pose = pose;
            state.quick_menu.set_sprite_pose(pose, ctx);
        }
    }
    let max_scroll = (settings_list_content_height(state) - settings_list_view_h(state)).max(0.0);
    state.settings_ui.list_scroll_y = state.settings_ui.list_scroll_y.clamp(0.0, max_scroll);
    state.save_config();
    state.play_sound(SoundCue::TabChanged, ctx);
}

pub fn handle_wallpaper_path_focus(state: &mut State, ctx: &mut EffectCtx<'_, crate::app::msg::Msg>) {
    state.settings_ui.open_dropdown = None;
    state.settings_ui.wallpaper_focused = true;
    state.settings_ui.remeasure_wallpaper_cursor(ctx);
}

pub fn handle_wallpaper_path_apply(state: &mut State, ctx: &mut EffectCtx<'_, crate::app::msg::Msg>) {
    let candidate = state.settings_ui.wallpaper_input.text().trim().to_owned();
    let probe_cfg = WallpaperConfig {
        enabled: true,
        path: candidate.clone(),
        fit: state.config.wallpaper.fit,
    };

    if state.config.wallpaper.enabled {
        match load_configured_wallpaper(&probe_cfg, ctx) {
            Ok(asset) => {
                state.config.wallpaper.path = candidate;
                state.settings_ui.wallpaper_error = None;
                state.settings_ui.wallpaper_focused = false;
                replace_wallpaper_asset(state, asset, ctx);
                state.save_config();
                state.play_sound(SoundCue::ToggleOn, ctx);
            }
            Err(err) => {
                state.settings_ui.wallpaper_error = Some(err);
                state.play_sound(SoundCue::DialogError, ctx);
            }
        }
    } else {
        // Wallpaper is currently disabled: validate path existence when non-empty before saving.
        if !candidate.is_empty() && !expand_tilde_path(&candidate).is_file() {
            state.settings_ui.wallpaper_error =
                Some(format!("Not found: {}", expand_tilde_path(&candidate).display()));
            state.play_sound(SoundCue::DialogError, ctx);
            return;
        }
        state.config.wallpaper.path = candidate;
        state.settings_ui.wallpaper_error = None;
        state.settings_ui.wallpaper_focused = false;
        state.save_config();
        state.play_sound(SoundCue::ToggleOn, ctx);
    }
}

pub fn handle_wallpaper_path_clear(state: &mut State, ctx: &mut EffectCtx<'_, crate::app::msg::Msg>) {
    state.settings_ui.wallpaper_input = TextInputState::new();
    state.settings_ui.wallpaper_cursor_x = 0.0;
    state.settings_ui.wallpaper_focused = false;
    state.settings_ui.wallpaper_error = None;
    state.config.wallpaper.path.clear();
    if state.config.wallpaper.enabled {
        let fallback = load_embedded_wallpaper(ctx);
        replace_wallpaper_asset(state, fallback, ctx);
    }
    state.save_config();
    state.play_sound(SoundCue::DialogCancel, ctx);
}

pub fn handle_settings_key(state: &mut State, ev: KeyEvent, ctx: &mut EffectCtx<'_, crate::app::msg::Msg>) {
    if !state.quick_menu.settings_open || !state.settings_ui.wallpaper_focused {
        if ev.key == Key::Escape {
            if state.settings_ui.open_dropdown.is_some() {
                state.settings_ui.open_dropdown = None;
                state.play_sound(SoundCue::MenuPopdown, ctx);
            } else {
                let was_open = state.quick_menu.is_popup_open();
                state.quick_menu.close_popup();
                state.settings_ui.unfocus_inputs();
                state.settings_ui.open_dropdown = None;
                if !state.hovered.is_clock_group() && !state.is_clock_slide_pinned() {
                    state.clock_menu_anim.set_open(false);
                }
                if was_open {
                    state.play_sound(SoundCue::MenuPopdown, ctx);
                }
            }
        }
        return;
    }

    match state.settings_ui.wallpaper_input.handle_key(&ev) {
        TextInputOutcome::Cancelled => {
            state.settings_ui.wallpaper_focused = false;
            state.settings_ui.wallpaper_error = None;
            state.settings_ui.wallpaper_input = TextInputState::new();
            state
                .settings_ui
                .wallpaper_input
                .set_text(&state.config.wallpaper.path);
            state.play_sound(SoundCue::DialogCancel, ctx);
        }
        TextInputOutcome::Submitted => handle_wallpaper_path_apply(state, ctx),
        TextInputOutcome::Edited | TextInputOutcome::CursorMoved => {
            state.settings_ui.wallpaper_error = None;
            state.settings_ui.remeasure_wallpaper_cursor(ctx);
        }
        TextInputOutcome::Unchanged => {}
    }
}

pub fn handle_settings_ime(state: &mut State, ev: ImeEvent, ctx: &mut EffectCtx<'_, crate::app::msg::Msg>) {
    if !state.quick_menu.settings_open || !state.settings_ui.wallpaper_focused {
        return;
    }
    let outcome = state.settings_ui.wallpaper_input.handle_ime(&ev);
    if outcome.needs_redraw() {
        state.settings_ui.wallpaper_error = None;
        state.settings_ui.remeasure_wallpaper_cursor(ctx);
    }
}

pub fn handle_preview_sound(state: &mut State, ctx: &EffectCtx<'_, crate::app::msg::Msg>) {
    state.play_sound(SoundCue::Bell, ctx);
}

pub fn handle_reset_defaults(state: &mut State, ctx: &mut EffectCtx<'_, crate::app::msg::Msg>) {
    state.config = WidgetConfig::default();
    state.settings_ui.sync_from_config(&state.config);
    state.settings_ui.open_dropdown = None;
    state.settings_ui.list_scroll_y = 0.0;

    state.clock_alt = state.config.general.clock_alt_default;
    state.visualizer.mode = state.config.widgets.visualizer_mode;
    state.sounds.apply_config(&state.config.audio);
    state
        .quick_menu
        .set_sprite_pose(state.config.sprite.quick_menu_pose, ctx);

    #[cfg(not(test))]
    {
        if let Ok(asset) = load_configured_wallpaper(&state.config.wallpaper, ctx) {
            replace_wallpaper_asset(state, asset, ctx);
        }
        state.desktop_sprite.ensure_loaded(ctx);
    }

    if state.config.general.foreground_booster {
        state.booster.on_focus_change(
            state.window_info.cgroup.as_ref(),
            state.memory.as_ref(),
            ctx,
        );
    }

    state.save_config();
    state.play_sound(SoundCue::ToggleOn, ctx);
}
