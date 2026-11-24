//! Minimalist MPRIS Control + Seek Bar floating overlay popup.

mod controls;
mod seek;

use waywi::prelude::*;

use crate::app::state::{HoverTarget, Msg, State};
use crate::theme::{BAR_HEIGHT, MPRIS_POPUP_WIDTH, NOTIF_CENTER_TOP_MARGIN};

use controls::build_control_box;
use seek::build_seek_box;

const POPUP_FULL_HEIGHT: f32 = 114.0;

pub fn build_mpris_popup_surface(state: &State) -> Surface<Msg> {
    let p = state.mpris.popup_progress.clamp(0.0, 1.0);
    let ease = 1.0 - (1.0 - p).powi(3);

    let fallback_left = {
        let right_margin = HoverTarget::TrayItem(state.tray_items.len().saturating_sub(1))
            .right_margin_hint(state.clock_alt, state.tray_items.len())
            + 210.0;
        (state.bar_width - right_margin).max(8.0)
    };
    let raw_left = if state.mpris.popup_left_x > 0.0 {
        state.mpris.popup_left_x
    } else {
        fallback_left
    };
    let left_x = raw_left.clamp(8.0, (state.bar_width - MPRIS_POPUP_WIDTH - 8.0).max(8.0));
    let animated_y = (BAR_HEIGHT - 8.0) + (NOTIF_CENTER_TOP_MARGIN - (BAR_HEIGHT - 8.0)) * ease;
    let animated_h = (POPUP_FULL_HEIGHT * ease).max(1.0);

    let control_box = build_control_box(state);
    let seek_box = build_seek_box(state);

    let popup_col = Column::new()
        .size(Size::new(
            MPRIS_POPUP_WIDTH.logical_px(),
            POPUP_FULL_HEIGHT.logical_px(),
        ))
        .gap(6.logical_px())
        .child(control_box)
        .child(seek_box);

    let animated_popup = Clip::new(popup_col)
        .key("mpris-popup-column")
        .position(Position::absolute(
            left_x.logical_px(),
            animated_y.logical_px(),
        ))
        .size(Size::new(
            MPRIS_POPUP_WIDTH.logical_px(),
            animated_h.logical_px(),
        ))
        .on_pointer_down(|_| Msg::Noop);

    let backdrop = Group::new()
        .key("mpris-popup-backdrop")
        .size(Size::fill())
        .on_pointer_down(|_| Msg::CloseMprisPopup)
        .child(animated_popup);

    Surface::floating("bar-mpris-popup", Output::Primary)
        .layer(Layer::Overlay)
        .ignore_exclusive_zone()
        .anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT)
        .size(Size::fill())
        .input_mode(InputMode::Full)
        .keyboard_interactivity(KeyboardInteractivity::OnDemand)
        .background(Color::TRANSPARENT)
        .on_key(|ev| {
            if ev.key == Key::Escape {
                Some(Msg::CloseMprisPopup)
            } else {
                None
            }
        })
        .child(backdrop)
}
