//! Icon-anchored DBusMenu tray popup with Catppuccin hover highlighting.

use waywi::prelude::*;
use waywi::dbus::TrayMenuItem;

use crate::app::state::{HoverTarget, Msg, State, TrayMenuPopup};
use crate::theme::{
    CRUST, MAUVE, OVERLAY0, POPUP_TOP_MARGIN, SUBTEXT0, SURFACE0, TEXT, font_ui, font_ui_semibold,
};

fn format_menu_label(item: &TrayMenuItem) -> String {
    let mut base = item.label.replace('_', "");
    if item.toggle_state == 1 {
        let mark = match item.toggle_type.as_deref() {
            Some("radio") => "\u{25cf} ",
            Some("checkmark") => "\u{2713} ",
            _ => "",
        };
        if !mark.is_empty() {
            base = format!("{mark}{base}");
        }
    }
    if !item.children.is_empty() {
        base.push_str("  \u{25b8}");
    }
    base
}

/// Computes the horizontal margin and width so tests can verify anchor alignment deterministically.
#[must_use]
pub fn tray_menu_left_and_width(menu: &TrayMenuPopup, bar_width: f32) -> (f32, f32) {
    let max_chars = menu
        .items
        .iter()
        .filter(|i| i.visible && !i.is_separator)
        .map(|i| format_menu_label(i).chars().count())
        .max()
        .unwrap_or(12)
        .max("Close Menu".len());
    let w = (max_chars as f32 * 7.4 + 36.0).max(200.0);
    let max_left = (bar_width - w - 4.0).max(4.0);
    let left = (menu.anchor_x - w * 0.5).clamp(4.0, max_left);
    (left, w)
}

pub fn build_tray_menu_surface(state: &State, menu: &TrayMenuPopup) -> Surface<Msg> {
    let visible_items: Vec<&TrayMenuItem> = menu.items.iter().filter(|i| i.visible).collect();
    let (left_margin, w) = tray_menu_left_and_width(menu, state.bar_width);
    let row_w = w - 12.0;

    let mut col = Column::new()
        .size(Size::auto())
        .padding(Edges::symmetric(4.logical_px(), 6.logical_px()))
        .gap(2.logical_px());

    for item in visible_items {
        if item.is_separator {
            col = col.child(
                Rect::new()
                    .size(Size::new(row_w.logical_px(), 1.logical_px()))
                    .fill(SURFACE0),
            );
            continue;
        }
        let item_id = item.id;
        let label = format_menu_label(item);
        let hovered = item.enabled && state.hovered == HoverTarget::TrayMenuItem(item_id);
        let row_bg = if hovered {
            SURFACE0
        } else {
            Color::TRANSPARENT
        };
        let fg = if !item.enabled {
            OVERLAY0
        } else if hovered {
            MAUVE
        } else {
            TEXT
        };

        let mut row_group = Group::new()
            .key(("tray-menu-item", item_id))
            .size(Size::new(row_w.logical_px(), 22.logical_px()))
            .child(
                Rect::new()
                    .size(Size::fill())
                    .fill(row_bg)
                    .corner_radius(4.logical_px()),
            )
            .child(
                Row::new()
                    .size(Size::fill())
                    .padding(Edges::symmetric(0.logical_px(), 8.logical_px()))
                    .align(Align::Center)
                    .child(
                        Text::new(label)
                            .font(if hovered {
                                font_ui_semibold()
                            } else {
                                font_ui()
                            })
                            .font_size(11.logical_px())
                            .color(fg),
                    ),
            );

        if item.enabled {
            row_group = row_group
                .cursor(CursorShape::Pointer)
                .on_hover(Msg::Hover(HoverTarget::TrayMenuItem(item_id)))
                .on_leave(Msg::Leave(HoverTarget::TrayMenuItem(item_id)))
                .on_click(Msg::TrayMenuClick { item_id });
        } else {
            row_group = row_group.on_click(Msg::CloseTrayMenu);
        }

        col = col.child(row_group);
    }

    let close_hovered = state.hovered == HoverTarget::TrayMenuClose;
    col = col.child(
        Group::new()
            .key("tray-menu-close")
            .size(Size::new(row_w.logical_px(), 20.logical_px()))
            .cursor(CursorShape::Pointer)
            .on_hover(Msg::Hover(HoverTarget::TrayMenuClose))
            .on_leave(Msg::Leave(HoverTarget::TrayMenuClose))
            .on_click(Msg::CloseTrayMenu)
            .child(
                Rect::new()
                    .size(Size::fill())
                    .fill(if close_hovered {
                        SURFACE0
                    } else {
                        Color::TRANSPARENT
                    })
                    .corner_radius(4.logical_px()),
            )
            .child(
                Row::new()
                    .size(Size::fill())
                    .padding(Edges::symmetric(0.logical_px(), 8.logical_px()))
                    .align(Align::Center)
                    .child(
                        Text::new("Close Menu")
                            .font(font_ui())
                            .font_size(10.logical_px())
                            .color(if close_hovered { TEXT } else { SUBTEXT0 }),
                    ),
            ),
    );

    let card = Group::new()
        .size(Size::auto())
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(CRUST)
                .corner_radius(6.logical_px())
                .border(1.logical_px(), OVERLAY0),
        )
        .child(col);

    Surface::floating("bar-tray-menu", Output::Primary)
        .layer(Layer::Overlay)
        .ignore_exclusive_zone()
        .anchor(Anchor::TOP | Anchor::LEFT)
        .margin(Edges::top_left(
            POPUP_TOP_MARGIN.logical_px(),
            left_margin.logical_px(),
        ))
        .size(Size::auto())
        .input_mode(InputMode::Full)
        .keyboard_interactivity(KeyboardInteractivity::None)
        .background(Color::TRANSPARENT)
        .on_close(Msg::CloseTrayMenu)
        .child(card)
}
