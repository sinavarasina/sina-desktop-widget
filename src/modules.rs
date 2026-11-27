//! Top-level bar surface composition and module submodules.

pub mod clock;
pub mod drawer;
pub mod left;
pub mod mpris;
pub mod right;
pub mod status_pill;
pub mod tray;

use waywi::prelude::*;

use crate::app::state::{Msg, State};
use crate::theme::BAR_HEIGHT;
use crate::widgets::annotate_overlay::build_annotate_surface;
use crate::widgets::bluetooth_popup::build_bluetooth_popup_surface;
use crate::widgets::calendar::build_calendar_tooltip_surface;
use crate::widgets::clipboard_popup::build_clipboard_popup_surface;
use crate::widgets::color_picker_overlay::build_color_picker_surface;
use crate::widgets::desktop_sprite::build_desktop_sprite_surface;
use crate::widgets::disk_popup::build_disk_popup_surface;
use crate::widgets::drun_bar::build_drun_surface;
use crate::widgets::mixer_popup::build_mixer_popup_surface;
use crate::widgets::mpris_popup::build_mpris_popup_surface;
use crate::widgets::notification_center::build_notification_center_surface;
use crate::widgets::quick_menu::build_quick_menu_surface;
use crate::widgets::screenshot_overlay::{
    build_screenshot_preview_surface, build_screenshot_surface,
};
use crate::widgets::telemetry_popup::build_telemetry_popup_surface;
use crate::widgets::tooltip::build_tooltip_surface;
use crate::widgets::tray_menu::build_tray_menu_surface;
use crate::widgets::wallpaper::build_wallpaper_surface;
use crate::widgets::zoom_overlay::build_zoom_surface;

pub fn build_surfaces(state: &State) -> Surfaces<Msg> {
    let mut surfaces = Surfaces::new();
    if state.config.wallpaper.enabled
        && let Some(wp) = state.wallpaper_asset
    {
        surfaces.push(build_wallpaper_surface(wp, state.config.wallpaper.fit));
    }
    if let Some(sprite) = build_desktop_sprite_surface(state) {
        surfaces.push(sprite);
    }
    surfaces.push(build_main_panel(state));
    if let Some(tip) = build_tooltip_surface(state) {
        surfaces.push(tip);
    }
    if let Some(menu) = &state.tray_menu {
        surfaces.push(build_tray_menu_surface(state, menu));
    }
    if state.notifications.is_center_visible() {
        surfaces.push(build_notification_center_surface(state));
    }
    if state.clipboard.is_popup_visible() {
        surfaces.push(build_clipboard_popup_surface(state));
    }
    if state.mpris.is_popup_visible() {
        surfaces.push(build_mpris_popup_surface(state));
    }
    if state.bt_ui.is_popup_visible() {
        surfaces.push(build_bluetooth_popup_surface(state));
    }
    if state.disk_ui.is_popup_visible() {
        surfaces.push(build_disk_popup_surface(state));
    }
    if state.audio_ui.is_popup_visible() {
        surfaces.push(build_mixer_popup_surface(state));
    }
    if state.telemetry_graph.is_popup_visible() {
        surfaces.push(build_telemetry_popup_surface(state));
    }
    if state.calendar_open {
        surfaces.push(build_calendar_tooltip_surface(state));
    }
    if state.quick_menu.is_popup_visible() {
        surfaces.push(build_quick_menu_surface(state));
    }
    if state.drun.is_visible() {
        surfaces.push(build_drun_surface(state));
    }
    let active_ws = state.active_workspace_key();
    if state.annotate.should_mount_surface(&active_ws) {
        surfaces.push(build_annotate_surface(state));
    }
    if state.zoom.should_mount_surface()
        && let Some(zoom_surface) = build_zoom_surface(state)
    {
        surfaces.push(zoom_surface);
    }
    if state.color_picker.should_mount_surface()
        && let Some(cp_surface) = build_color_picker_surface(state)
    {
        surfaces.push(cp_surface);
    }
    if state.screenshot.should_mount_surface()
        && let Some(ss_surface) = build_screenshot_surface(state)
    {
        surfaces.push(ss_surface);
    }
    if state.screenshot.has_visible_preview()
        && let Some(preview_surface) = build_screenshot_preview_surface(state)
    {
        surfaces.push(preview_surface);
    }
    surfaces
}

fn build_main_panel(state: &State) -> Surface<Msg> {
    let root = Group::new()
        .size(Size::fill())
        .child(left::build_left_modules(state))
        .child(
            right::build_right_modules(state)
                .position(Position::relative_to_parent(
                    Percent::new(100.0),
                    Percent::new(50.0),
                ))
                .anchor(ElementAnchor::CenterRight),
        );

    Surface::panel("bar", Output::Primary)
        .layer(Layer::Top)
        .inhibit_idle(state.caffeine_active)
        .anchor(Anchor::TOP | Anchor::LEFT | Anchor::RIGHT)
        .size(Size::new(Length::Fill, BAR_HEIGHT.logical_px()))
        .exclusive_zone(BAR_HEIGHT.logical_px())
        .input_mode(InputMode::Selective)
        .keyboard_interactivity(KeyboardInteractivity::None)
        .background(Color::TRANSPARENT)
        .child(root)
}
