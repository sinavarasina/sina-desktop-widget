//! Left bar group: Gentoo launcher logo, Hyprland workspaces, and active window scroller.

mod logo;
pub mod notifications;
mod sysinfo;
mod window_info;
mod window_scroll;
mod workspaces;

use waywi::prelude::*;

use crate::app::state::{Msg, State};
use crate::theme::{BAR_HEIGHT, CRUST, MANTLE, SURFACE0};
use crate::widgets::separators::{SepShape, separator};

use logo::build_logo;
use notifications::build_notification_segment;
use sysinfo::build_sysinfo;
use window_info::build_window_info;
use window_scroll::build_window_scroll;
use workspaces::{build_active_ws, build_inactive_ws, resolve_active_ws_id};

pub fn build_left_modules(state: &State) -> Row<Msg> {
    let active_id = resolve_active_ws_id(state);
    let inactive_block = build_inactive_ws(state, active_id);

    Row::new()
        .key("modules-left")
        .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
        .align(Align::Center)
        .child(build_logo(state))
        .child(build_sysinfo(state))
        .child(separator(
            "sep-l-sys-l",
            SepShape::CurveL,
            MANTLE,
            Color::TRANSPARENT,
        ))
        .child(build_active_ws(active_id))
        .child(separator(
            "sep-l-2",
            SepShape::ArrowR,
            MANTLE,
            Color::TRANSPARENT,
        ))
        .child(separator(
            "sep-l-3",
            SepShape::CurveL,
            MANTLE,
            Color::TRANSPARENT,
        ))
        .child(inactive_block)
        .child(separator(
            "sep-l-4",
            SepShape::SlashR,
            MANTLE,
            Color::TRANSPARENT,
        ))
        .child(separator(
            "sep-l-5",
            SepShape::SlashRThin,
            SURFACE0,
            Color::TRANSPARENT,
        ))
        .child(separator(
            "sep-l-6",
            SepShape::SlashLInv,
            CRUST,
            Color::TRANSPARENT,
        ))
        .child(build_window_scroll(state))
        .child(build_window_info(state))
        .child(separator(
            "sep-l-8",
            SepShape::SlashRThin,
            SURFACE0,
            Color::TRANSPARENT,
        ))
        .child(build_notification_segment(state))
}
