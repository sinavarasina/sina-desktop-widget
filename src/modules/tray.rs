//! StatusNotifierItem tray bar widget with passive dimming, attention blink, and hover fill.

use waywi::prelude::*;

use crate::app::state::{HoverTarget, Msg, State};
use crate::theme::{
    BAR_HEIGHT, CRUST, MANTLE, MAUVE, SURFACE0, TEXT, TRAY_ICON_SIZE, TRAY_PAD_H, TRAY_SPACING,
    font_ui_bold,
};

pub fn build_tray(state: &State) -> Element<Msg> {
    if state.tray_items.is_empty() {
        return Rect::new()
            .key("mod-tray")
            .size(Size::px(0.logical_px(), BAR_HEIGHT.logical_px()))
            .into();
    }

    let mut items_row = Row::new()
        .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
        .padding(Edges::symmetric(0.logical_px(), TRAY_PAD_H.logical_px()))
        .gap(TRAY_SPACING.logical_px())
        .align(Align::Center);

    for (idx, entry) in state.tray_items.iter().enumerate() {
        let opacity = if entry.snapshot.status == TrayStatus::Passive {
            0.6
        } else {
            1.0
        };
        let is_hovered = state.hovered == HoverTarget::TrayItem(idx);
        let item_bg = if entry.snapshot.status == TrayStatus::NeedsAttention {
            if state.blink_phase { MAUVE } else { SURFACE0 }
        } else if is_hovered {
            SURFACE0
        } else {
            Color::TRANSPARENT
        };
        let icon_el: Element<Msg> = if let Some(asset) = entry.icon {
            Image::new(asset)
                .size(Size::px(
                    TRAY_ICON_SIZE.logical_px(),
                    TRAY_ICON_SIZE.logical_px(),
                ))
                .fit(ImageFit::Contain)
                .into()
        } else {
            let letter = entry
                .snapshot
                .title
                .chars()
                .next()
                .or_else(|| entry.snapshot.id.chars().next())
                .unwrap_or('?');
            Text::new(letter.to_uppercase().to_string())
                .font(font_ui_bold())
                .font_size(10.logical_px())
                .color(
                    if entry.snapshot.status == TrayStatus::NeedsAttention && state.blink_phase {
                        CRUST
                    } else {
                        TEXT
                    },
                )
                .into()
        };

        items_row = items_row.child(
            Group::new()
                .key(("tray-item", idx))
                .size(Size::px(18.logical_px(), 18.logical_px()))
                .opacity(opacity)
                .on_hover(Msg::Hover(HoverTarget::TrayItem(idx)))
                .on_leave(Msg::Leave(HoverTarget::TrayItem(idx)))
                .on_pointer_down(move |ev| Msg::TrayClick { index: idx, ev })
                .child(
                    Rect::new()
                        .size(Size::fill())
                        .fill(item_bg)
                        .corner_radius(4.logical_px()),
                )
                .child(icon_el_centered(icon_el)),
        );
    }

    // `box-shadow: inset -2px 0 0 0 @surface0` rendered as a crisp right-edge 2px hairline.
    let container_row = Row::new()
        .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
        .align(Align::Center)
        .child(items_row)
        .child(
            Rect::new()
                .size(Size::px(2.logical_px(), BAR_HEIGHT.logical_px()))
                .fill(SURFACE0),
        );

    Group::new()
        .key("mod-tray")
        .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
        .child(Rect::new().size(Size::fill()).fill(MANTLE))
        .child(container_row)
        .into()
}

fn icon_el_centered(inner: Element<Msg>) -> Element<Msg> {
    Row::new()
        .size(Size::fill())
        .align(Align::Center)
        .padding(Edges::symmetric(0.logical_px(), 1.logical_px()))
        .child(inner)
        .into()
}
