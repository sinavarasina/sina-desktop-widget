//! Minimalist Bluetooth Management Popup overlay on the right side of the screen.

use waywi::prelude::*;

use crate::app::bluetooth::BtDeviceMenuAction;
use crate::app::state::{HoverTarget, Msg, State};
use crate::widgets::text::marquee_slice;
use crate::theme::{
    BAR_HEIGHT, BLUE, CRUST, GREEN, MANTLE, MAUVE, NOTIF_CENTER_TOP_MARGIN, OVERLAY0, RED,
    SUBTEXT1, SURFACE0, SURFACE1, TEXT, font_ui, font_ui_bold, font_ui_semibold,
};

const BT_POPUP_WIDTH: f32 = 310.0;
const BT_HEADER_HEIGHT: f32 = 34.0;
const BT_ROW_HEIGHT: f32 = 38.0;
const BT_ROW_GAP: f32 = 4.0;

pub fn build_bluetooth_popup_surface(state: &State) -> Surface<Msg> {
    let p = state.bt_ui.popup_progress.clamp(0.0, 1.0);
    let ease = 1.0 - (1.0 - p).powi(3);

    let fallback_left = (state.bar_width - BT_POPUP_WIDTH - 68.0).max(8.0);
    let raw_left = if state.bt_ui.popup_left_x > 0.0 {
        state.bt_ui.popup_left_x
    } else {
        fallback_left
    };
    let left_x = raw_left.clamp(8.0, (state.bar_width - BT_POPUP_WIDTH - 8.0).max(8.0));

    let max_total_height = (state.screen_height - NOTIF_CENTER_TOP_MARGIN - 16.0).max(200.0);
    let max_body_height = (max_total_height - BT_HEADER_HEIGHT - 44.0).max(120.0);
    let visible_slots =
        (((max_body_height - 12.0 + BT_ROW_GAP) / (BT_ROW_HEIGHT + BT_ROW_GAP)).floor() as usize)
            .max(1);

    let mut sorted_devs: Vec<&BluetoothDevice> = state.bluetooth.devices.iter().collect();
    sorted_devs.sort_by_key(|d| (!d.connected, !d.paired, d.alias.as_str()));

    let start = state.bt_ui.scroll_index.min(sorted_devs.len());
    let shown_devs: Vec<&BluetoothDevice> = sorted_devs
        .into_iter()
        .skip(start)
        .take(visible_slots)
        .collect();

    let mut list_col = Column::new()
        .size(Size::new(Length::Fill, Length::Auto))
        .gap(BT_ROW_GAP.logical_px())
        .padding(Edges::all(6.logical_px()));

    if shown_devs.is_empty() {
        list_col = list_col.child(
            Row::new()
                .size(Size::new(Length::Fill, 36.logical_px()))
                .padding(Edges::symmetric(0.logical_px(), 8.logical_px()))
                .align(Align::Center)
                .child(
                    Text::new("No devices")
                        .font(font_ui())
                        .font_size(11.logical_px())
                        .color(OVERLAY0),
                ),
        );
    } else {
        for (idx, dev) in shown_devs.iter().enumerate() {
            let elem = if state.bt_ui.menu_path.as_deref() == Some(dev.object_path.as_str()) {
                build_device_menu(state, dev, start + idx)
            } else {
                build_device_row(state, dev, start + idx)
            };
            list_col = list_col.child(elem);
        }
    }

    let body_height = if shown_devs.is_empty() {
        48.0
    } else {
        let count = shown_devs.len();
        let gaps = count.saturating_sub(1) as f32 * BT_ROW_GAP;
        ((count as f32) * BT_ROW_HEIGHT + gaps + 14.0).min(max_body_height)
    };

    let prompt_height = if state.bt_ui.pending_prompt.is_some() {
        34.0
    } else {
        0.0
    };
    let full_height = BT_HEADER_HEIGHT + 1.0 + prompt_height + body_height;
    let animated_h = (full_height * ease).max(1.0);
    let animated_y = (BAR_HEIGHT - 8.0) + (NOTIF_CENTER_TOP_MARGIN - (BAR_HEIGHT - 8.0)) * ease;

    let mut inner_col = Column::new()
        .size(Size::new(Length::Fill, Length::Auto))
        .child(build_header(state))
        .child(
            Rect::new()
                .size(Size::new(Length::Fill, 1.logical_px()))
                .fill(SURFACE1),
        );

    if let Some(prompt) = &state.bt_ui.pending_prompt {
        inner_col = inner_col.child(build_prompt_banner(prompt.request_id, &prompt.label));
    }

    inner_col = inner_col.child(
        Clip::new(list_col)
            .size(Size::new(Length::Fill, body_height.logical_px()))
            .on_scroll(Msg::BtPopupScroll),
    );

    let box_group = Group::new()
        .size(Size::new(
            BT_POPUP_WIDTH.logical_px(),
            full_height.logical_px(),
        ))
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(CRUST)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), SURFACE1),
        )
        .child(inner_col);

    let animated_box = Clip::new(box_group)
        .key("bluetooth-popup-box")
        .position(Position::absolute(left_x.logical_px(), animated_y.logical_px()))
        .size(Size::new(
            BT_POPUP_WIDTH.logical_px(),
            animated_h.logical_px(),
        ))
        .on_pointer_down(|_| Msg::Noop);

    let backdrop = Group::new()
        .key("bluetooth-popup-backdrop")
        .size(Size::fill())
        .on_pointer_down(|_| Msg::CloseBtPopup)
        .child(animated_box);

    Surface::floating("bar-bluetooth-popup", Output::Primary)
        .layer(Layer::Overlay)
        .ignore_exclusive_zone()
        .anchor(Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right)
        .size(Size::fill())
        .input_mode(InputMode::Full)
        .keyboard_interactivity(KeyboardInteractivity::OnDemand)
        .background(Color::TRANSPARENT)
        .on_key(|ev| {
            if ev.key == Key::Escape {
                Some(Msg::CloseBtPopup)
            } else {
                None
            }
        })
        .child(backdrop)
}

fn build_header(state: &State) -> Element<Msg> {
    let adapter = state.bluetooth.adapters.first();
    let powered = adapter.is_some_and(|a| a.powered);
    let discovering = adapter.is_some_and(|a| a.discovering);
    let discoverable = adapter.is_some_and(|a| a.discoverable);

    let power_btn = header_btn(
        "bt-btn-power",
        if powered { "On" } else { "Off" },
        HoverTarget::BtPower,
        state.hovered == HoverTarget::BtPower,
        powered,
        MAUVE,
        Msg::BtTogglePower,
    );
    let scan_btn = header_btn(
        "bt-btn-scan",
        if discovering { "Scanning" } else { "Scan" },
        HoverTarget::BtScan,
        state.hovered == HoverTarget::BtScan,
        discovering,
        MAUVE,
        Msg::BtToggleScan,
    );
    let discoverable_btn = header_btn(
        "bt-btn-discoverable",
        if discoverable { "Visible" } else { "Hidden" },
        HoverTarget::BtDiscoverable,
        state.hovered == HoverTarget::BtDiscoverable,
        discoverable,
        MAUVE,
        Msg::BtToggleDiscoverable,
    );

    Row::new()
        .size(Size::new(Length::Fill, BT_HEADER_HEIGHT.logical_px()))
        .padding(Edges::symmetric(0.logical_px(), 10.logical_px()))
        .gap(8.logical_px())
        .align(Align::Center)
        .child(
            Text::new("Bluetooth")
                .font(font_ui_bold())
                .font_size(12.logical_px())
                .color(MAUVE),
        )
        .child(power_btn)
        .child(scan_btn)
        .child(discoverable_btn)
        .into()
}

/// Compact pill button shared by the header toggles and the device context menu.
/// Same geometry, fonts, and colors everywhere: hover paints `SURFACE0`, the active
/// state uses the `accent` color for both border and text.
fn header_btn(
    key: &'static str,
    label: &'static str,
    target: HoverTarget,
    hovered: bool,
    active: bool,
    accent: Color,
    msg: Msg,
) -> Element<Msg> {
    let bg = if hovered { SURFACE0 } else { MANTLE };
    let fg = if active { accent } else { SUBTEXT1 };
    Group::new()
        .key(key)
        .size(Size::auto())
        .on_hover(Msg::Hover(target))
        .on_leave(Msg::Leave(target))
        .on_click(msg)
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(bg)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), if active { accent } else { SURFACE0 }),
        )
        .child(
            Row::new()
                .padding(Edges::symmetric(3.logical_px(), 8.logical_px()))
                .align(Align::Center)
                .child(
                    Text::new(label)
                        .font(font_ui_semibold())
                        .font_size(10.5.logical_px())
                        .color(fg),
                ),
        )
        .into()
}

/// Context-menu pill button. Same geometry, fonts, and hover behavior as `header_btn`,
/// but always framed in the caller's accent color (Connect green / Trust mauve / Forget red).
fn menu_btn(
    key: &'static str,
    label: &'static str,
    target: HoverTarget,
    hovered: bool,
    accent: Color,
    msg: Msg,
) -> Element<Msg> {
    let bg = if hovered { SURFACE0 } else { MANTLE };
    Group::new()
        .key(key)
        .size(Size::auto())
        .on_hover(Msg::Hover(target))
        .on_leave(Msg::Leave(target))
        .on_click(msg)
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(bg)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), accent),
        )
        .child(
            Row::new()
                .padding(Edges::symmetric(2.logical_px(), 6.logical_px()))
                .align(Align::Center)
                .child(
                    Text::new(label)
                        .font(font_ui_semibold())
                        .font_size(10.logical_px())
                        .color(accent),
                ),
        )
        .into()
}

fn build_prompt_banner(request_id: u64, label: &str) -> Element<Msg> {
    let text = marquee_slice(label, 24, 0);
    Row::new()
        .key(("bt-prompt", request_id))
        .size(Size::new(Length::Fill, 34.logical_px()))
        .padding(Edges::symmetric(4.logical_px(), 10.logical_px()))
        .gap(6.logical_px())
        .align(Align::Center)
        .child(
            Text::new(text)
                .font(font_ui_semibold())
                .font_size(10.5.logical_px())
                .color(TEXT),
        )
        .child(prompt_btn("bt-accept", "Accept", MAUVE, Msg::BtAgentDecision {
            request_id,
            accept: true,
        }))
        .child(prompt_btn("bt-reject", "Reject", RED, Msg::BtAgentDecision {
            request_id,
            accept: false,
        }))
        .into()
}

fn prompt_btn(key: &'static str, label: &'static str, col: Color, msg: Msg) -> Element<Msg> {
    Group::new()
        .key(key)
        .size(Size::auto())
        .on_click(msg)
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(MANTLE)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), col),
        )
        .child(
            Row::new()
                .padding(Edges::symmetric(2.logical_px(), 6.logical_px()))
                .align(Align::Center)
                .child(
                    Text::new(label)
                        .font(font_ui_semibold())
                        .font_size(10.logical_px())
                        .color(col),
                ),
        )
        .into()
}

fn build_device_row(state: &State, dev: &BluetoothDevice, idx: usize) -> Element<Msg> {
    let hovered = state.hovered == HoverTarget::BtDevice(idx);
    let bg = if hovered { SURFACE0 } else { MANTLE };
    let border_col = if dev.connected {
        BLUE
    } else if dev.paired {
        SURFACE1
    } else {
        SURFACE0
    };

    let raw_name = if !dev.alias.trim().is_empty() {
        dev.alias.as_str()
    } else if !dev.name.trim().is_empty() {
        dev.name.as_str()
    } else {
        dev.address.as_str()
    };
    let name_text = marquee_slice(raw_name, 22, state.mpris.marquee_tick);

    let status_text = if dev.connected {
        dev.battery_percentage
            .map_or_else(|| String::from("connected"), |b| format!("connected {b}%"))
    } else if dev.paired {
        String::from("paired")
    } else {
        String::from("pair")
    };
    let status_col = if dev.connected { MAUVE } else { SUBTEXT1 };

    let path = dev.object_path.clone();
    Group::new()
        .key(("bt-dev", idx))
        .size(Size::new(Length::Fill, BT_ROW_HEIGHT.logical_px()))
        .on_hover(Msg::Hover(HoverTarget::BtDevice(idx)))
        .on_leave(Msg::Leave(HoverTarget::BtDevice(idx)))
        .on_pointer_down(move |ev| Msg::BtDevicePointerDown {
            path: path.clone(),
            ev,
        })
        // Same WayWi scroll routing as the notification cards: the topmost hit-test node
        // captures scroll without bubbling, so each row must carry the handler itself.
        .on_scroll(Msg::BtPopupScroll)
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(bg)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), border_col),
        )
        .child(
            Row::new()
                .size(Size::fill())
                .padding(Edges::symmetric(0.logical_px(), 10.logical_px()))
                .gap(8.logical_px())
                .align(Align::Center)
                .child(
                    Text::new(if dev.connected { "\u{f00b1}" } else { "\u{f294}" })
                        .font(font_ui())
                        .font_size(12.logical_px())
                        .color(if dev.connected { MAUVE } else { OVERLAY0 }),
                )
                .child(
                    Clip::new(
                        Text::new(name_text)
                            .font(font_ui_semibold())
                            .font_size(11.logical_px())
                            .color(TEXT),
                    )
                    .size(Size::px(168.logical_px(), 16.logical_px())),
                )
                .child(
                    Text::new(status_text)
                        .font(font_ui_semibold())
                        .font_size(10.logical_px())
                        .color(status_col),
                )
                .child(
                    Text::new("\u{f00c}")
                        .font(font_ui())
                        .font_size(9.logical_px())
                        .color(MAUVE)
                        .visible(dev.trusted),
                ),
        )
        .into()
}

/// Device context menu shown in place of the row (right-click): name stays visible with its
/// marquee so the device is identifiable, plus Connect / Trust / Forget pill buttons using the
/// same geometry and hover behavior as the header toggles, each with its own accent color.
fn build_device_menu(state: &State, dev: &BluetoothDevice, idx: usize) -> Element<Msg> {
    let connect_label: &'static str = if dev.connected {
        "Disconnect"
    } else if dev.paired {
        "Connect"
    } else {
        "Pair"
    };
    let trust_label: &'static str = if dev.trusted { "Untrust" } else { "Trust" };
    let raw_name = if !dev.alias.trim().is_empty() {
        dev.alias.as_str()
    } else if !dev.name.trim().is_empty() {
        dev.name.as_str()
    } else {
        dev.address.as_str()
    };
    let name_text = marquee_slice(raw_name, 10, state.mpris.marquee_tick);

    let path = dev.object_path.clone();
    let trust_path = dev.object_path.clone();
    let forget_path = dev.object_path.clone();
    let toggle_path = dev.object_path.clone();
    let border_col = if dev.connected { BLUE } else { SURFACE1 };

    Group::new()
        .key(("bt-dev-menu", idx))
        .size(Size::new(Length::Fill, BT_ROW_HEIGHT.logical_px()))
        .on_hover(Msg::Hover(HoverTarget::BtDevice(idx)))
        .on_leave(Msg::Leave(HoverTarget::BtDevice(idx)))
        // Forward right-click so a second right-click collapses the menu back to the
        // device row (left-click on the card background does nothing).
        .on_pointer_down(move |ev| {
            if ev.button == Some(MouseButton::Right) {
                Msg::BtDevicePointerDown {
                    path: toggle_path.clone(),
                    ev,
                }
            } else {
                Msg::Noop
            }
        })
        .on_scroll(Msg::BtPopupScroll)
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(MANTLE)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), border_col),
        )
        .child(
            Row::new()
                .size(Size::fill())
                .padding(Edges::symmetric(0.logical_px(), 10.logical_px()))
                .gap(4.logical_px())
                .align(Align::Center)
                .child(
                    Text::new(if dev.connected { "\u{f00b1}" } else { "\u{f294}" })
                        .font(font_ui())
                        .font_size(12.logical_px())
                        .color(if dev.connected { MAUVE } else { OVERLAY0 }),
                )
                .child(
                    Clip::new(
                        Text::new(name_text)
                            .font(font_ui_semibold())
                            .font_size(11.logical_px())
                            .color(TEXT),
                    )
                    .size(Size::px(72.logical_px(), 16.logical_px())),
                )
                .child(menu_btn(
                    "bt-menu-connect",
                    connect_label,
                    HoverTarget::BtMenuConnect,
                    state.hovered == HoverTarget::BtMenuConnect,
                    GREEN,
                    Msg::BtDeviceMenuAction {
                        path,
                        action: BtDeviceMenuAction::ToggleConnect,
                    },
                ))
                .child(menu_btn(
                    "bt-menu-trust",
                    trust_label,
                    HoverTarget::BtMenuTrust,
                    state.hovered == HoverTarget::BtMenuTrust,
                    MAUVE,
                    Msg::BtDeviceMenuAction {
                        path: trust_path,
                        action: BtDeviceMenuAction::ToggleTrust,
                    },
                ))
                .child(menu_btn(
                    "bt-menu-forget",
                    "Forget",
                    HoverTarget::BtMenuForget,
                    state.hovered == HoverTarget::BtMenuForget,
                    RED,
                    Msg::BtDeviceMenuAction {
                        path: forget_path,
                        action: BtDeviceMenuAction::Forget,
                    },
                )),
        )
        .into()
}
