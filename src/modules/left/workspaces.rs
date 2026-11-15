//! Hyprland active and inactive workspace pills.

use waywi::prelude::*;

use crate::app::state::{HoverTarget, Msg, State};
use crate::theme::{BAR_HEIGHT, MANTLE, MAUVE, OVERLAY0, TEXT, font_ui, font_ui_bold};

pub fn resolve_active_ws_id(state: &State) -> i32 {
    let active_ws = state.workspaces.iter().find(|w| w.focused).or_else(|| {
        state
            .workspaces
            .iter()
            .find(|w| w.id.get() == state.active_ws_id)
    });
    active_ws.map_or(state.active_ws_id, |w| w.id.get())
}

pub fn build_active_ws(active_id: i32) -> Element<Msg> {
    Group::new()
        .key("ws-active")
        .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
        .on_hover(Msg::Hover(HoverTarget::ActiveWs))
        .on_leave(Msg::Leave(HoverTarget::ActiveWs))
        .on_click(Msg::SwitchWorkspace(WorkspaceId::new(active_id)))
        .child(Rect::new().size(Size::fill()).fill(MANTLE))
        .child(
            Row::new()
                .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
                .padding(Edges::symmetric(0.logical_px(), 4.logical_px()))
                .align(Align::Center)
                .child(
                    Text::new(format!("[{active_id}]:"))
                        .font(font_ui_bold())
                        .font_size(12.logical_px())
                        .line_height(1.15)
                        .color(MAUVE),
                ),
        )
        .into()
}

pub fn build_inactive_ws(state: &State, active_id: i32) -> Element<Msg> {
    let mut inactive_row = Row::new()
        .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
        .align(Align::Center);
    let mut inactive_count = 0usize;
    for ws in state
        .workspaces
        .iter()
        .filter(|w| !w.focused && w.id.get() != active_id && w.id.get() > 0)
    {
        inactive_count += 1;
        let id = ws.id;
        let is_hovered = state.hovered == HoverTarget::InactiveWs(id.get());
        let fg = if is_hovered { TEXT } else { OVERLAY0 };
        inactive_row = inactive_row.child(
            Row::new()
                .key(("ws-inactive", id.get()))
                .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
                .padding(Edges::symmetric(0.logical_px(), 4.logical_px()))
                .align(Align::Center)
                .on_hover(Msg::Hover(HoverTarget::InactiveWs(id.get())))
                .on_leave(Msg::Leave(HoverTarget::InactiveWs(id.get())))
                .on_click(Msg::SwitchWorkspace(id))
                .child(
                    Text::new(format!("{}", id.get()))
                        .font(font_ui())
                        .font_size(12.logical_px())
                        .line_height(1.15)
                        .color(fg),
                ),
        );
    }

    if inactive_count > 0 {
        Group::new()
            .key("ws-inactive-group")
            .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
            .child(Rect::new().size(Size::fill()).fill(MANTLE))
            .child(inactive_row)
            .into()
    } else {
        Rect::<Msg>::new()
            .size(Size::px(0.logical_px(), BAR_HEIGHT.logical_px()))
            .into()
    }
}
