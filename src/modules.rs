//! Top-level bar surface composition and module submodules.

pub mod clock;
pub mod drawer;
pub mod left;
pub mod right;
pub mod tray;

use waywi::prelude::*;

use crate::app::state::{Msg, State};
use crate::theme::BAR_HEIGHT;
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
