//! Left bar group: Gentoo launcher logo, Hyprland workspaces, and active window scroller.

mod logo;
pub mod notifications;
mod sysinfo;
mod window_info;
mod window_scroll;
pub mod workspaces;

use waywi::prelude::*;

use crate::app::state::{Msg, State};
use crate::theme::{BAR_HEIGHT, CRUST, MANTLE, SURFACE0};
use crate::widgets::separators::{SepShape, separator};

use logo::build_logo;
use notifications::build_notification_segment;
use sysinfo::build_sysinfo;
use window_info::build_window_info;
use window_scroll::build_window_scroll;
use workspaces::build_workspaces;

pub fn build_left_modules(state: &State) -> Row<Msg> {
    let w = &state.config.widgets;
    let mut row = Row::new()
        .key("modules-left")
        .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
        .align(Align::Center)
        .child(build_logo(state));

    if w.show_sysinfo {
        row = row.child(build_sysinfo(state));
    } else {
        row = row.child(separator(
            "sep-l-logo-r",
            SepShape::ArrowR,
            CRUST,
            Color::TRANSPARENT,
        ));
    }

    if w.show_workspaces {
        row = row
            .child(separator(
                "sep-l-sys-l",
                SepShape::CurveL,
                MANTLE,
                Color::TRANSPARENT,
            ))
            .child(build_workspaces(state))
            .child(separator(
                "sep-l-4",
                SepShape::SlashR,
                MANTLE,
                Color::TRANSPARENT,
            ));
    }

    if w.show_window_info {
        row = row
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
            .child(build_window_info(state));
    }

    if w.show_notifications {
        row = row
            .child(separator(
                "sep-l-8",
                SepShape::SlashRThin,
                SURFACE0,
                Color::TRANSPARENT,
            ))
            .child(build_notification_segment(state));
    }

    row
}
