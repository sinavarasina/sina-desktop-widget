//! Top-level bar surface composition and module submodules.

pub mod clock;
pub mod drawer;
pub mod left;
pub mod mpris;
pub mod right;
pub mod tray;

use waywi::prelude::*;

use crate::app::state::{Msg, State};
use crate::theme::BAR_HEIGHT;
use crate::widgets::bluetooth_popup::build_bluetooth_popup_surface;
use crate::widgets::disk_popup::build_disk_popup_surface;
use crate::widgets::drun_bar::build_drun_surface;
use crate::widgets::mixer_popup::build_mixer_popup_surface;
use crate::widgets::mpris_popup::build_mpris_popup_surface;
use crate::widgets::notification_center::build_notification_center_surface;
use crate::widgets::tooltip::build_tooltip_surface;
use crate::widgets::tray_menu::build_tray_menu_surface;

pub fn build_surfaces(state: &State) -> Surfaces<Msg> {
    let mut surfaces = Surfaces::new();
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
    if state.drun.is_visible() {
        surfaces.push(build_drun_surface(state));
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
        .layer(Layer::Bottom)
        .anchor(Anchor::Top | Anchor::Left | Anchor::Right)
        .size(Size::new(Length::Fill, BAR_HEIGHT.logical_px()))
        .exclusive_zone(BAR_HEIGHT.logical_px())
        .input_mode(InputMode::Selective)
        .keyboard_interactivity(KeyboardInteractivity::None)
        .background(Color::TRANSPARENT)
        .child(root)
}
