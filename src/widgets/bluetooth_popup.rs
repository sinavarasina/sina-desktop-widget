//! Minimalist Bluetooth Management Popup overlay on the right side of the screen.

use waywi::prelude::*;

use crate::app::state::{HoverTarget, Msg, State};
use crate::modules::mpris::marquee_slice;
use crate::theme::{
    BAR_HEIGHT, BLUE, CRUST, MANTLE, MAUVE, NOTIF_CENTER_TOP_MARGIN, OVERLAY0, RED, SUBTEXT1,
    SURFACE0, SURFACE1, TEXT, font_ui, font_ui_bold, font_ui_semibold,
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
            list_col = list_col.child(build_device_row(state, dev, start + idx));
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

    let power_btn = header_btn(
        "bt-btn-power",
        if powered { "On" } else { "Off" },
        HoverTarget::BtPower,
        state.hovered == HoverTarget::BtPower,
        powered,
        Msg::BtTogglePower,
    );
    let scan_btn = header_btn(
        "bt-btn-scan",
        if discovering { "Scanning" } else { "Scan" },
        HoverTarget::BtScan,
        state.hovered == HoverTarget::BtScan,
        discovering,
        Msg::BtToggleScan,
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
        .into()
}

fn header_btn(
    key: &'static str,
    label: &'static str,
    target: HoverTarget,
    hovered: bool,
    active: bool,
    msg: Msg,
) -> Element<Msg> {
    let bg = if hovered { SURFACE0 } else { MANTLE };
    let fg = if active { MAUVE } else { SUBTEXT1 };
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
                .border(1.logical_px(), if active { MAUVE } else { SURFACE0 }),
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
    let confirming = state.bt_ui.confirm_remove_path.as_deref() == Some(dev.object_path.as_str());
    let bg = if hovered { SURFACE0 } else { MANTLE };
    let border_col = if confirming {
        RED
    } else if dev.connected {
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
    let name_text = marquee_slice(raw_name, if confirming { 14 } else { 22 }, state.mpris.marquee_tick);

    let status_text = if confirming {
        String::from("Remove? (R:Yes / L:No)")
    } else if dev.connected {
        dev.battery_percentage
            .map_or_else(|| String::from("connected"), |b| format!("connected {b}%"))
    } else if dev.paired {
        String::from("paired")
    } else {
        String::from("pair")
    };
    let status_col = if confirming {
        RED
    } else if dev.connected {
        MAUVE
    } else {
        SUBTEXT1
    };
    let name_w = if confirming { 104.0 } else { 168.0 };

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
                    .size(Size::px(name_w.logical_px(), 16.logical_px())),
                )
                .child(
                    Text::new(status_text)
                        .font(font_ui_semibold())
                        .font_size(10.logical_px())
                        .color(status_col),
                ),
        )
        .into()
}
