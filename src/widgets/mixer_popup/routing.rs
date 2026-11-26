//! Expandable Routing & Balance card inside the Mixer Popup: output/input device switching,
//! hardware port/route switching, collapsible sound card profile picker, and center-anchored
//! stereo/surround L/R balance slider.

use waywi::prelude::*;

use crate::app::audio::AudioSnapshotExt;
use crate::app::state::{AudioControlTarget, HoverTarget, Msg, State};
use crate::theme::{
    BLUE, CRUST, MANTLE, MAUVE, MIXER_POPUP_WIDTH, OVERLAY0, SUBTEXT1, SURFACE0, SURFACE1, TEXT,
    font_ui, font_ui_semibold,
};
use crate::widgets::text::marquee_slice;

pub const BALANCE_TRACK_W: f32 = 196.0;
const CARD_PAD: f32 = 8.0;
const SCROLLBAR_RESERVE: f32 = 6.0;
const CONTENT_W: f32 = MIXER_POPUP_WIDTH - CARD_PAD * 2.0 - SCROLLBAR_RESERVE;
const MAX_ROUTING_VIEW_H: f32 = 216.0;

const SUBSECTION_H: f32 = 16.0;
const ITEM_H: f32 = 26.0;
const PROFILE_OPT_H: f32 = 24.0;
const BALANCE_ROW_H: f32 = 30.0;
const ROW_GAP: f32 = 4.0;
const OPT_GAP: f32 = 3.0;

/// Returns a crisp Nerd Font icon matching an audio port/route name.
#[must_use]
pub fn port_icon(port_name: &str, is_source: bool) -> &'static str {
    let lower = port_name.to_ascii_lowercase();
    if is_source {
        if lower.contains("headset") || lower.contains("headphone") {
            "\u{f025}"
        } else {
            "\u{f130}"
        }
    } else if lower.contains("headphone") || lower.contains("headset") {
        "\u{f025}"
    } else if lower.contains("hdmi") || lower.contains("displayport") || lower.contains("dp") {
        "\u{f26c}"
    } else if lower.contains("bluetooth") || lower.contains("bluez") {
        "\u{f294}"
    } else {
        "\u{f028}"
    }
}

/// Formats L/R balance (`-1.0..=1.0`) as a compact badge (`C`, `L25`, `R40`).
#[must_use]
pub fn format_balance_short(bal: f32) -> String {
    if bal.abs() < 0.01 {
        String::from("C")
    } else {
        let pct = (bal.abs() * 100.0).round() as u8;
        if bal < 0.0 {
            format!("L{pct}")
        } else {
            format!("R{pct}")
        }
    }
}

/// Whether the active output sink or input source supports L/R stereo/surround balance.
#[must_use]
pub fn target_supports_balance(state: &State) -> bool {
    let Some(audio) = state.audio.as_ref() else {
        return false;
    };
    let Some(volume) = audio.target_volume(state.audio_target == AudioControlTarget::Source)
    else {
        return false;
    };
    let (channels, map) = (&volume.channels, &volume.channel_map);
    if !map.is_empty() {
        let has_l = map.iter().any(|p| p.is_left());
        let has_r = map.iter().any(|p| p.is_right());
        has_l && has_r
    } else {
        channels.len() != 1
    }
}

fn target_devices(state: &State) -> (&[AudioDevice], Option<&AudioDevice>) {
    let Some(audio) = state.audio.as_ref() else {
        return (&[], None);
    };
    let is_source = state.audio_target == AudioControlTarget::Source;
    let device = audio.target_device(is_source);
    if is_source {
        (audio.sources.as_slice(), device)
    } else {
        (audio.sinks.as_slice(), device)
    }
}

fn target_ports(state: &State) -> (&[AudioPort], Option<&str>) {
    let Some(audio) = state.audio.as_ref() else {
        return (&[], None);
    };
    match audio.target_device(state.audio_target == AudioControlTarget::Source) {
        Some(d) => (d.ports.as_slice(), d.active_port.as_deref()),
        None => (&[], None),
    }
}

fn target_card(state: &State) -> Option<&AudioCard> {
    let audio = state.audio.as_ref()?;
    let is_source = state.audio_target == AudioControlTarget::Source;
    audio.target_card(is_source)
}

fn available_profile_count(card: &AudioCard) -> usize {
    let active = card.active_profile.as_deref();
    card.profiles
        .iter()
        .filter(|p| p.available || active == Some(p.name.as_str()))
        .count()
}

fn available_profiles(card: &AudioCard) -> Vec<&AudioProfile> {
    let active = card.active_profile.as_deref();
    card.profiles
        .iter()
        .filter(|p| p.available || active == Some(p.name.as_str()))
        .collect()
}

/// Computes exact logical height of all controls inside the Routing & Balance card (zero-alloc).
#[must_use]
pub fn routing_content_height(state: &State) -> f32 {
    let mut sum = 0.0_f32;
    let mut count = 0usize;
    let mut push = |h: f32, n: usize| {
        sum += h * (n as f32);
        count += n;
    };

    let (devices, _) = target_devices(state);
    if devices.len() > 1 {
        push(SUBSECTION_H, 1);
        push(ITEM_H, devices.len());
    }

    let (ports, _) = target_ports(state);
    if !ports.is_empty() {
        push(SUBSECTION_H, 1);
        push(ITEM_H, ports.len());
    }

    if let Some(card) = target_card(state) {
        let prof_count = available_profile_count(card);
        if prof_count > 1 {
            push(SUBSECTION_H, 1);
            push(ITEM_H, 1);
            if state.audio_ui.profiles_open {
                let block_h = (prof_count as f32) * PROFILE_OPT_H
                    + (prof_count.saturating_sub(1) as f32) * OPT_GAP;
                push(block_h, 1);
            }
        }
    }

    if target_supports_balance(state) {
        push(SUBSECTION_H, 1);
        push(BALANCE_ROW_H, 1);
    }

    if count == 0 {
        return ITEM_H;
    }

    sum + (count.saturating_sub(1) as f32) * ROW_GAP
}

#[must_use]
pub fn routing_view_height(state: &State) -> f32 {
    routing_content_height(state).min(MAX_ROUTING_VIEW_H)
}

/// Builds the expandable Routing & Balance card and returns `(card_element, full_card_height)`.
pub fn build_routing_card(state: &State) -> (Element<Msg>, f32) {
    let is_source = state.audio_target == AudioControlTarget::Source;
    let content_h = routing_content_height(state);
    let view_h = content_h.min(MAX_ROUTING_VIEW_H);
    let max_scroll = (content_h - view_h).max(0.0);
    let scroll_y = state.audio_ui.routing_scroll_y.clamp(0.0, max_scroll);
    let inner_w = MIXER_POPUP_WIDTH - CARD_PAD * 2.0;

    let mut rows: Vec<Element<Msg>> = Vec::new();

    // 1. Output / Input Devices
    let (devices, active_dev) = target_devices(state);
    let active_dev_id = active_dev.map(|d| d.id);
    if devices.len() > 1 {
        rows.push(subsection_header(
            "mixer-sub-dev",
            if is_source {
                "INPUT DEVICES"
            } else {
                "OUTPUT DEVICES"
            },
        ));
        for dev in devices {
            let active = active_dev_id == Some(dev.id);
            let label = if dev.description.is_empty() {
                dev.name.as_str()
            } else {
                dev.description.as_str()
            };
            let icon = dev
                .active_port
                .as_deref()
                .map_or(if is_source { "\u{f130}" } else { "\u{f028}" }, |p| {
                    port_icon(p, is_source)
                });
            rows.push(build_select_row(
                state,
                ("mixer-dev", dev.id),
                HoverTarget::MixerDevice(dev.id),
                icon,
                label,
                None,
                active,
                Msg::MixerSelectDevice(dev.id),
            ));
        }
    }

    // 2. Hardware Ports / Routes on Active Device
    let (ports, active_port_name) = target_ports(state);
    if !ports.is_empty() {
        rows.push(subsection_header("mixer-sub-port", "PORTS"));
        for (idx, port) in ports.iter().enumerate() {
            let active = active_port_name == Some(port.name.as_str());
            let label = if port.description.is_empty() {
                port.name.as_str()
            } else {
                port.description.as_str()
            };
            let badge = (!port.availability.is_available()).then_some("unplugged");
            rows.push(build_select_row(
                state,
                ("mixer-port", idx as u32),
                HoverTarget::MixerPort(idx),
                port_icon(&port.name, is_source),
                label,
                badge,
                active,
                Msg::MixerSelectPort(port.name.clone()),
            ));
        }
    }

    // 3. Card Profile Dropdown
    if let Some(card) = target_card(state) {
        let profs = available_profiles(card);
        if profs.len() > 1 {
            rows.push(subsection_header("mixer-sub-prof", "CARD PROFILE"));
            let active_name = card.active_profile.as_deref().unwrap_or("");
            let active_desc = profs
                .iter()
                .find(|p| p.name == active_name)
                .map(|p| {
                    if p.description.is_empty() {
                        p.name.as_str()
                    } else {
                        p.description.as_str()
                    }
                })
                .unwrap_or("Default");
            rows.push(build_profile_trigger(state, active_desc));
            if state.audio_ui.profiles_open {
                rows.push(build_profile_options(state, card.id, &profs, active_name));
            }
        }
    }

    // 4. Stereo / Surround L/R Balance Slider
    if target_supports_balance(state) {
        rows.push(subsection_header("mixer-sub-bal", "BALANCE"));
        rows.push(build_balance_row(state));
    }

    if rows.is_empty() {
        rows.push(
            Row::new()
                .size(Size::new(CONTENT_W.logical_px(), ITEM_H.logical_px()))
                .padding(Edges::symmetric(0.logical_px(), 6.logical_px()))
                .align(Align::Center)
                .child(
                    Text::new("No routing options")
                        .font(font_ui())
                        .font_size(10.logical_px())
                        .color(OVERLAY0),
                )
                .into(),
        );
    }

    let mut list_col = Column::new()
        .size(Size::new(CONTENT_W.logical_px(), content_h.logical_px()))
        .gap(ROW_GAP.logical_px())
        .position(Position::absolute(0.logical_px(), (-scroll_y).logical_px()));
    for r in rows {
        list_col = list_col.child(r);
    }

    let mut viewport = Group::new()
        .position(Position::absolute(
            CARD_PAD.logical_px(),
            CARD_PAD.logical_px(),
        ))
        .size(Size::new(inner_w.logical_px(), view_h.logical_px()))
        .child(
            Clip::new(
                Group::new()
                    .size(Size::new(CONTENT_W.logical_px(), view_h.logical_px()))
                    .child(list_col),
            )
            .size(Size::new(CONTENT_W.logical_px(), view_h.logical_px()))
            .on_scroll(Msg::MixerRoutingScroll),
        );

    if max_scroll > 0.5 {
        viewport = viewport.child(build_scrollbar_thumb(
            inner_w - 2.0,
            view_h,
            content_h,
            scroll_y,
        ));
    }

    let card_h = view_h + CARD_PAD * 2.0;
    let card = Group::new()
        .key("mixer-routing-card")
        .size(Size::new(MIXER_POPUP_WIDTH.logical_px(), card_h.logical_px()))
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(CRUST)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), SURFACE1),
        )
        .child(viewport)
        .into();

    (card, card_h)
}

fn subsection_header(key: &'static str, title: &'static str) -> Element<Msg> {
    Row::new()
        .key(key)
        .size(Size::new(CONTENT_W.logical_px(), SUBSECTION_H.logical_px()))
        .gap(6.logical_px())
        .align(Align::Center)
        .child(
            Rect::new()
                .size(Size::new(2.logical_px(), 9.logical_px()))
                .fill(MAUVE),
        )
        .child(
            Text::new(title)
                .font(font_ui_semibold())
                .font_size(8.5.logical_px())
                .color(MAUVE),
        )
        .child(
            Rect::new()
                .size(Size::new(Length::Fill, 1.logical_px()))
                .fill(SURFACE0),
        )
        .into()
}

#[allow(clippy::too_many_arguments)]
fn build_select_row(
    state: &State,
    key: (&'static str, u32),
    target: HoverTarget,
    icon: &'static str,
    label: &str,
    badge: Option<&'static str>,
    active: bool,
    on_click: Msg,
) -> Element<Msg> {
    let hovered = state.hovered == target;
    let bg = if hovered { SURFACE0 } else { MANTLE };
    let border = if active {
        MAUVE
    } else if hovered {
        SURFACE1
    } else {
        SURFACE0
    };
    let fg = if active {
        MAUVE
    } else if badge.is_some() {
        OVERLAY0
    } else if hovered {
        TEXT
    } else {
        SUBTEXT1
    };

    let max_chars = if badge.is_some() { 26 } else { 36 };
    let label_text = marquee_slice(label, max_chars, state.audio_ui.marquee_tick);

    let mut row = Group::new()
        .key(key)
        .size(Size::new(CONTENT_W.logical_px(), ITEM_H.logical_px()))
        .cursor(CursorShape::Pointer)
        .on_hover(Msg::Hover(target))
        .on_leave(Msg::Leave(target))
        .on_click(on_click)
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(bg)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), border),
        );

    if active {
        row = row.child(
            Rect::new()
                .position(Position::absolute(0.logical_px(), 0.logical_px()))
                .size(Size::new(2.logical_px(), ITEM_H.logical_px()))
                .fill(MAUVE),
        );
    }

    row = row
        .child(
            Text::new(icon)
                .font(font_ui())
                .font_size(10.5.logical_px())
                .bounds_mode(TextBoundsMode::Optical)
                .color(if active { MAUVE } else { fg })
                .position(Position::absolute(9.logical_px(), (ITEM_H * 0.5).logical_px()))
                .anchor(ElementAnchor::CenterLeft),
        )
        .child(
            Text::new(label_text)
                .font(if active {
                    font_ui_semibold()
                } else {
                    font_ui()
                })
                .font_size(9.5.logical_px())
                .color(fg)
                .position(Position::absolute(
                    26.logical_px(),
                    (ITEM_H * 0.5).logical_px(),
                ))
                .anchor(ElementAnchor::CenterLeft),
        );

    if let Some(tag) = badge {
        row = row.child(
            Text::new(tag)
                .font(font_ui())
                .font_size(8.5.logical_px())
                .color(OVERLAY0)
                .position(Position::absolute(
                    (CONTENT_W - 8.0).logical_px(),
                    (ITEM_H * 0.5).logical_px(),
                ))
                .anchor(ElementAnchor::CenterRight),
        );
    } else if active {
        row = row.child(
            Text::new("\u{f00c}")
                .font(font_ui())
                .font_size(9.logical_px())
                .bounds_mode(TextBoundsMode::Optical)
                .color(MAUVE)
                .position(Position::absolute(
                    (CONTENT_W - 8.0).logical_px(),
                    (ITEM_H * 0.5).logical_px(),
                ))
                .anchor(ElementAnchor::CenterRight),
        );
    }

    row.into()
}

fn build_profile_trigger(state: &State, active_desc: &str) -> Element<Msg> {
    let hovered = state.hovered == HoverTarget::MixerProfileToggle;
    let open = state.audio_ui.profiles_open;
    let caret = if open { "\u{f0d8}" } else { "\u{f0d7}" };
    let border = if open {
        MAUVE
    } else if hovered {
        SURFACE1
    } else {
        SURFACE0
    };
    let text = marquee_slice(active_desc, 34, state.audio_ui.marquee_tick);

    Group::new()
        .key("mixer-profile-toggle")
        .size(Size::new(CONTENT_W.logical_px(), ITEM_H.logical_px()))
        .cursor(CursorShape::Pointer)
        .on_hover(Msg::Hover(HoverTarget::MixerProfileToggle))
        .on_leave(Msg::Leave(HoverTarget::MixerProfileToggle))
        .on_click(Msg::MixerToggleProfiles)
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(if open || hovered { SURFACE0 } else { MANTLE })
                .corner_radius(0.logical_px())
                .border(1.logical_px(), border),
        )
        .child(
            Text::new(text)
                .font(font_ui_semibold())
                .font_size(9.5.logical_px())
                .color(if open { MAUVE } else { TEXT })
                .position(Position::absolute(8.logical_px(), (ITEM_H * 0.5).logical_px()))
                .anchor(ElementAnchor::CenterLeft),
        )
        .child(
            Text::new(caret)
                .font(font_ui())
                .font_size(9.5.logical_px())
                .bounds_mode(TextBoundsMode::Optical)
                .color(if open { MAUVE } else { SUBTEXT1 })
                .position(Position::absolute(
                    (CONTENT_W - 8.0).logical_px(),
                    (ITEM_H * 0.5).logical_px(),
                ))
                .anchor(ElementAnchor::CenterRight),
        )
        .into()
}

fn build_profile_options(
    state: &State,
    card_id: u32,
    profiles: &[&AudioProfile],
    active_name: &str,
) -> Element<Msg> {
    let block_h = (profiles.len() as f32) * PROFILE_OPT_H
        + (profiles.len().saturating_sub(1) as f32) * OPT_GAP;

    let mut col = Column::new()
        .key("mixer-profile-options")
        .size(Size::new(CONTENT_W.logical_px(), block_h.logical_px()))
        .gap(OPT_GAP.logical_px());

    for (idx, prof) in profiles.iter().enumerate() {
        let target = HoverTarget::MixerProfile(idx);
        let hovered = state.hovered == target;
        let selected = prof.name == active_name;
        let label = if prof.description.is_empty() {
            prof.name.as_str()
        } else {
            prof.description.as_str()
        };
        let display = marquee_slice(label, 36, state.audio_ui.marquee_tick);
        let bg = if selected || hovered { SURFACE0 } else { CRUST };
        let border = if selected {
            MAUVE
        } else if hovered {
            SURFACE1
        } else {
            SURFACE0
        };

        let opt = Group::new()
            .key(("mixer-prof-opt", idx))
            .size(Size::new(CONTENT_W.logical_px(), PROFILE_OPT_H.logical_px()))
            .cursor(CursorShape::Pointer)
            .on_hover(Msg::Hover(target))
            .on_leave(Msg::Leave(target))
            .on_click(Msg::MixerSelectProfile {
                card_id,
                profile_name: prof.name.clone(),
            })
            .child(
                Rect::new()
                    .size(Size::fill())
                    .fill(bg)
                    .corner_radius(0.logical_px())
                    .border(1.logical_px(), border),
            )
            .child(
                Text::new(display)
                    .font(if selected {
                        font_ui_semibold()
                    } else {
                        font_ui()
                    })
                    .font_size(9.logical_px())
                    .color(if selected {
                        MAUVE
                    } else if hovered {
                        TEXT
                    } else {
                        SUBTEXT1
                    })
                    .position(Position::absolute(
                        10.logical_px(),
                        (PROFILE_OPT_H * 0.5).logical_px(),
                    ))
                    .anchor(ElementAnchor::CenterLeft),
            );
        col = col.child(opt);
    }

    col.into()
}

fn build_balance_row(state: &State) -> Element<Msg> {
    let bal = state
        .audio
        .as_ref()
        .map_or(0.0, |a| {
            a.target_balance(state.audio_target == AudioControlTarget::Source)
        })
        .clamp(-1.0, 1.0);
    let hovered = state.hovered == HoverTarget::MixerBalanceBar;
    let half_w = BALANCE_TRACK_W * 0.5;
    let fill_w = (bal.abs() * half_w).round();
    let fill_x = if bal < 0.0 { half_w - fill_w } else { half_w };
    let readout = format_balance_short(bal);
    let panned = bal.abs() >= 0.01;

    let mut bar = Group::new()
        .position(Position::absolute(0.logical_px(), 6.logical_px()))
        .size(Size::new(BALANCE_TRACK_W.logical_px(), 6.logical_px()))
        .child(Rect::new().size(Size::fill()).fill(SURFACE0));

    if fill_w >= 1.0 {
        bar = bar.child(
            Rect::new()
                .position(Position::absolute(fill_x.logical_px(), 0.logical_px()))
                .size(Size::new(fill_w.logical_px(), 6.logical_px()))
                .fill(BLUE),
        );
    }

    // Center detent tick so `0.0` is unmistakably visible.
    bar = bar.child(
        Rect::new()
            .position(Position::absolute((half_w - 0.5).logical_px(), 0.logical_px()))
            .size(Size::new(1.logical_px(), 6.logical_px()))
            .fill(if panned { TEXT } else { MAUVE }),
    );

    let track = Group::new()
        .key("mixer-balance-track")
        .position(Position::absolute(
            22.logical_px(),
            ((BALANCE_ROW_H - 18.0) * 0.5).logical_px(),
        ))
        .size(Size::new(BALANCE_TRACK_W.logical_px(), 18.logical_px()))
        .cursor(CursorShape::EwResize)
        .on_hover(Msg::Hover(HoverTarget::MixerBalanceBar))
        .on_leave(Msg::Leave(HoverTarget::MixerBalanceBar))
        .on_pointer_down(Msg::MixerBalanceSeekClick)
        .child(bar);

    Group::new()
        .key("mixer-balance-row")
        .size(Size::new(CONTENT_W.logical_px(), BALANCE_ROW_H.logical_px()))
        .on_scroll(Msg::MixerBalanceScroll)
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(MANTLE)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), if hovered { SURFACE1 } else { SURFACE0 }),
        )
        .child(
            Text::new("L")
                .font(font_ui_semibold())
                .font_size(9.logical_px())
                .color(if bal < -0.01 { BLUE } else { SUBTEXT1 })
                .position(Position::absolute(
                    8.logical_px(),
                    (BALANCE_ROW_H * 0.5).logical_px(),
                ))
                .anchor(ElementAnchor::CenterLeft),
        )
        .child(track)
        .child(
            Text::new("R")
                .font(font_ui_semibold())
                .font_size(9.logical_px())
                .color(if bal > 0.01 { BLUE } else { SUBTEXT1 })
                .position(Position::absolute(
                    (22.0 + BALANCE_TRACK_W + 6.0).logical_px(),
                    (BALANCE_ROW_H * 0.5).logical_px(),
                ))
                .anchor(ElementAnchor::CenterLeft),
        )
        .child(
            Text::new(readout)
                .font(font_ui_semibold())
                .font_size(9.logical_px())
                .color(if panned { BLUE } else { SUBTEXT1 })
                .position(Position::absolute(
                    (CONTENT_W - 8.0).logical_px(),
                    (BALANCE_ROW_H * 0.5).logical_px(),
                ))
                .anchor(ElementAnchor::CenterRight),
        )
        .into()
}

fn build_scrollbar_thumb(x: f32, view_h: f32, content_h: f32, scroll_y: f32) -> Element<Msg> {
    let ratio = (view_h / content_h).clamp(0.18, 1.0);
    let thumb_h = (view_h * ratio).max(18.0);
    let max_scroll = (content_h - view_h).max(1.0);
    let track_span = (view_h - thumb_h).max(0.0);
    let thumb_y = (scroll_y / max_scroll) * track_span;

    Group::new()
        .position(Position::absolute(x.logical_px(), 0.logical_px()))
        .size(Size::new(2.logical_px(), view_h.logical_px()))
        .child(Rect::new().size(Size::fill()).fill(SURFACE0))
        .child(
            Rect::new()
                .position(Position::absolute(0.logical_px(), thumb_y.logical_px()))
                .size(Size::new(2.logical_px(), thumb_h.logical_px()))
                .fill(MAUVE),
        )
        .into()
}
