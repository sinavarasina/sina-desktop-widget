//! Minimalist per-app volume mixer popup (master, expandable device/port/profile & balance
//! routing card, and per-app playback/capture rows).

pub mod routing;

use waywi::prelude::*;

use crate::app::state::{AudioControlTarget, HoverTarget, Msg, State};
use crate::theme::{
    BAR_HEIGHT, BLUE, CRUST, MANTLE, MAUVE, MIXER_POPUP_WIDTH, MIXER_TRACK_WIDTH,
    NOTIF_CENTER_TOP_MARGIN, OVERLAY0, SUBTEXT1, SURFACE0, SURFACE1, TEXT, font_ui, font_ui_bold,
    font_ui_semibold,
};
use crate::widgets::text::marquee_slice;

use crate::app::audio::AudioSnapshotExt;

use routing::{build_routing_card, format_balance_short, port_icon, target_supports_balance};

/// The currently selected port/route on a device, resolved for its description.
fn active_port_of(dev: &AudioDevice) -> Option<&AudioPort> {
    let name = dev.active_port.as_deref()?;
    dev.ports.iter().find(|p| p.name == name)
}

const MASTER_CARD_HEIGHT: f32 = 80.0;
const APPS_HEADER_HEIGHT: f32 = 24.0;
const ROW_HEIGHT: f32 = 36.0;
const ROW_GAP: f32 = 4.0;
const CARD_PAD_H: f32 = 10.0;
const TEXT_COL_WIDTH: f32 = MIXER_TRACK_WIDTH;
const NAME_CLIP_WIDTH: f32 = TEXT_COL_WIDTH - 34.0;
const MASTER_NAME_WIDTH: f32 = 104.0;
const ICON_BADGE_PX: f32 = 18.0;
const ICON_PX: f32 = 16.0;

pub fn build_mixer_popup_surface(state: &State) -> Surface<Msg> {
    let p = state.audio_ui.popup_progress.clamp(0.0, 1.0);
    let ease = 1.0 - (1.0 - p).powi(3);

    let fallback_left = (state.bar_width - MIXER_POPUP_WIDTH - 8.0).max(8.0);
    let raw_left = if state.audio_ui.popup_left_x > 0.0 {
        state.audio_ui.popup_left_x
    } else {
        fallback_left
    };
    let left_x = raw_left.clamp(8.0, (state.bar_width - MIXER_POPUP_WIDTH - 8.0).max(8.0));

    let routing_card = if state.audio_ui.routing_open {
        Some(build_routing_card(state))
    } else {
        None
    };
    let routing_extra_h = routing_card.as_ref().map_or(0.0, |(_, h)| *h + 6.0);

    let max_total_height = (state.screen_height - NOTIF_CENTER_TOP_MARGIN - 16.0).max(240.0);
    let max_body_height = (max_total_height
        - MASTER_CARD_HEIGHT
        - routing_extra_h
        - 6.0
        - APPS_HEADER_HEIGHT
        - 14.0)
        .max(ROW_HEIGHT);

    let (rows, body_height) = build_app_rows(state, max_body_height);

    let master_card = build_master_card(state);
    let apps_card = build_apps_card(rows, body_height);

    let full_height =
        MASTER_CARD_HEIGHT + routing_extra_h + 6.0 + APPS_HEADER_HEIGHT + body_height;
    let animated_h = (full_height * ease).max(1.0);
    let animated_y = (BAR_HEIGHT - 8.0) + (NOTIF_CENTER_TOP_MARGIN - (BAR_HEIGHT - 8.0)) * ease;

    let mut popup_col = Column::new()
        .size(Size::new(
            MIXER_POPUP_WIDTH.logical_px(),
            full_height.logical_px(),
        ))
        .gap(6.logical_px())
        .child(master_card);

    if let Some((card, _)) = routing_card {
        popup_col = popup_col.child(card);
    }

    popup_col = popup_col.child(apps_card);

    let animated_popup = Clip::new(popup_col)
        .key("mixer-popup-column")
        .position(Position::absolute(
            left_x.logical_px(),
            animated_y.logical_px(),
        ))
        .size(Size::new(
            MIXER_POPUP_WIDTH.logical_px(),
            animated_h.logical_px(),
        ))
        .on_pointer_down(|_| Msg::Noop);

    let backdrop = Group::new()
        .key("mixer-popup-backdrop")
        .size(Size::fill())
        .on_pointer_down(|_| Msg::CloseMixerPopup)
        .child(animated_popup);

    Surface::floating("bar-mixer-popup", Output::Primary)
        .layer(Layer::Overlay)
        .ignore_exclusive_zone()
        .anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT)
        .size(Size::fill())
        .input_mode(InputMode::Full)
        .keyboard_interactivity(KeyboardInteractivity::OnDemand)
        .background(Color::TRANSPARENT)
        .on_key(|ev| {
            if ev.key == Key::Escape {
                Some(Msg::CloseMixerPopup)
            } else {
                None
            }
        })
        .child(backdrop)
}

/// Master card: `Master: <device>` with output/input toggles above the volume track.
fn build_master_card(state: &State) -> Element<Msg> {
    let out_active = state.audio_target == AudioControlTarget::Sink;

    let is_source = state.audio_target == AudioControlTarget::Source;
    let device_name = match state.audio.as_ref().and_then(|a| a.target_device(is_source)) {
        Some(d) => {
            if d.description.is_empty() {
                d.name.as_str()
            } else {
                d.description.as_str()
            }
        }
        None if state.audio.is_some() => {
            if is_source {
                "Default Source"
            } else {
                "Default Sink"
            }
        }
        None => "PipeWire",
    };
    let name_text = marquee_slice(
        device_name,
        (MASTER_NAME_WIDTH / 5.2) as usize,
        state.audio_ui.marquee_tick,
    );

    let muted = state
        .audio
        .as_ref()
        .and_then(|a| a.target_volume(is_source))
        .is_some_and(|v| v.muted);
    let master_linear = state
        .audio
        .as_ref()
        .and_then(|a| a.target_volume(is_source))
        .map_or(0.0, |v| v.linear);
    let pct = (master_linear * 100.0).clamp(0.0, 150.0).round() as u16;
    let fill_w = MIXER_TRACK_WIDTH * master_linear.clamp(0.0, 1.0);
    let footer_row = build_master_footer(state, !out_active);

    Group::new()
        .key("mixer-master-card")
        .size(Size::new(
            MIXER_POPUP_WIDTH.logical_px(),
            MASTER_CARD_HEIGHT.logical_px(),
        ))
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(CRUST)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), SURFACE1),
        )
        .child(
            Column::new()
                .size(Size::fill())
                .padding(Edges::symmetric(6.logical_px(), CARD_PAD_H.logical_px()))
                .gap(6.logical_px())
                .align(Align::Center)
                .child(
                    Row::new()
                        .size(Size::new(Length::Fill, Length::Auto))
                        .gap(6.logical_px())
                        .align(Align::Center)
                        .child(
                            Text::new("Master:")
                                .font(font_ui_bold())
                                .font_size(10.5.logical_px())
                                .color(MAUVE),
                        )
                        .child(
                            Clip::new(
                                Text::new(name_text)
                                    .font(font_ui_semibold())
                                    .font_size(10.5.logical_px())
                                    .color(TEXT),
                            )
                            .size(Size::new(MASTER_NAME_WIDTH.logical_px(), 14.logical_px())),
                        )
                        .child(target_btn(
                            "mixer-btn-output",
                            "output",
                            AudioControlTarget::Sink,
                            HoverTarget::MixerOutput,
                            state.hovered == HoverTarget::MixerOutput,
                            out_active,
                        ))
                        .child(target_btn(
                            "mixer-btn-input",
                            "input",
                            AudioControlTarget::Source,
                            HoverTarget::MixerInput,
                            state.hovered == HoverTarget::MixerInput,
                            !out_active,
                        )),
                )
                .child(
                    Row::new()
                        .size(Size::new(Length::Fill, Length::Auto))
                        .gap(8.logical_px())
                        .align(Align::Center)
                        .child(
                            Group::new()
                                .key("mixer-master-track")
                                .size(Size::new(MIXER_TRACK_WIDTH.logical_px(), 6.logical_px()))
                                .on_hover(Msg::Hover(HoverTarget::MixerMaster))
                                .on_leave(Msg::Leave(HoverTarget::MixerMaster))
                                .on_pointer_down(Msg::MixerMasterSeekClick)
                                .on_scroll(Msg::MixerMasterScroll)
                                .child(Rect::new().size(Size::fill()).fill(SURFACE0))
                                .child(
                                    Rect::new()
                                        .size(Size::new(fill_w.logical_px(), 6.logical_px()))
                                        .fill(if muted { OVERLAY0 } else { MAUVE }),
                                ),
                        )
                        .child(
                            Text::new(if muted {
                                String::from("muted")
                            } else {
                                format!("{pct}%")
                            })
                            .font(font_ui_semibold())
                            .font_size(10.logical_px())
                            .color(SUBTEXT1),
                        ),
                )
                .child(footer_row),
        )
        .into()
}

/// Compact master footer showing active port/routing toggle on the left and L/R balance readout
/// on the right (scroll to nudge balance, right-click to snap center).
fn build_master_footer(state: &State, is_source: bool) -> Element<Msg> {
    let active_port = state
        .audio
        .as_ref()
        .and_then(|a| a.target_device(is_source))
        .and_then(active_port_of);
    let (route_icon, route_label) = match active_port {
        Some(p) => {
            let desc = if p.description.is_empty() {
                p.name.as_str()
            } else {
                p.description.as_str()
            };
            (port_icon(&p.name, is_source), desc)
        }
        None => (
            if is_source { "\u{f130}" } else { "\u{f028}" },
            "Devices & Ports",
        ),
    };

    let open = state.audio_ui.routing_open;
    let route_hovered = state.hovered == HoverTarget::MixerRoutingToggle;
    let route_border = if open {
        MAUVE
    } else if route_hovered {
        SURFACE1
    } else {
        SURFACE0
    };
    let route_fg = if open {
        MAUVE
    } else if route_hovered {
        TEXT
    } else {
        SUBTEXT1
    };
    let caret = if open { "\u{f0d8}" } else { "\u{f0d7}" };
    let short_label = marquee_slice(route_label, 22, state.audio_ui.marquee_tick);
    let inner_w = MIXER_POPUP_WIDTH - CARD_PAD_H * 2.0;

    let route_btn = Group::new()
        .key("mixer-routing-toggle")
        .position(Position::absolute(0.logical_px(), 0.logical_px()))
        .size(Size::auto())
        .on_hover(Msg::Hover(HoverTarget::MixerRoutingToggle))
        .on_leave(Msg::Leave(HoverTarget::MixerRoutingToggle))
        .on_click(Msg::MixerToggleRouting)
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(if open || route_hovered {
                    SURFACE0
                } else {
                    MANTLE
                })
                .corner_radius(0.logical_px())
                .border(1.logical_px(), route_border),
        )
        .child(
            Row::new()
                .padding(Edges::symmetric(2.logical_px(), 7.logical_px()))
                .gap(5.logical_px())
                .align(Align::Center)
                .child(
                    Text::new(route_icon)
                        .font(font_ui())
                        .font_size(9.5.logical_px())
                        .bounds_mode(TextBoundsMode::Optical)
                        .color(route_fg),
                )
                .child(
                    Text::new(short_label)
                        .font(font_ui_semibold())
                        .font_size(9.logical_px())
                        .color(route_fg),
                )
                .child(
                    Text::new(caret)
                        .font(font_ui())
                        .font_size(8.5.logical_px())
                        .bounds_mode(TextBoundsMode::Optical)
                        .color(route_fg),
                ),
        );

    let mut bar = Group::new()
        .size(Size::new(inner_w.logical_px(), 20.logical_px()))
        .child(route_btn);

    if target_supports_balance(state) {
        let bal = state
            .audio
            .as_ref()
            .map_or(0.0, |a| a.target_balance(is_source));
        let panned = bal.abs() >= 0.01;
        let bal_hovered = state.hovered == HoverTarget::MixerBalanceBar;
        let bal_text = format!("Bal {}", format_balance_short(bal));
        let bal_col = if panned {
            BLUE
        } else if bal_hovered {
            TEXT
        } else {
            SUBTEXT1
        };

        let bal_pill = Group::new()
            .key("mixer-balance-pill")
            .position(Position::absolute(inner_w.logical_px(), 0.logical_px()))
            .anchor(ElementAnchor::TopRight)
            .size(Size::auto())
            .on_hover(Msg::Hover(HoverTarget::MixerBalanceBar))
            .on_leave(Msg::Leave(HoverTarget::MixerBalanceBar))
            .on_pointer_down(|ev| match ev.button {
                Some(MouseButton::Right | MouseButton::Middle) => Msg::MixerBalanceSeekClick(ev),
                _ => Msg::MixerToggleRouting,
            })
            .on_scroll(Msg::MixerBalanceScroll)
            .child(
                Rect::new()
                    .size(Size::fill())
                    .fill(if bal_hovered { SURFACE0 } else { MANTLE })
                    .corner_radius(0.logical_px())
                    .border(
                        1.logical_px(),
                        if panned {
                            BLUE
                        } else if bal_hovered {
                            SURFACE1
                        } else {
                            SURFACE0
                        },
                    ),
            )
            .child(
                Row::new()
                    .padding(Edges::symmetric(2.logical_px(), 7.logical_px()))
                    .align(Align::Center)
                    .child(
                        Text::new(bal_text)
                            .font(font_ui_semibold())
                            .font_size(9.logical_px())
                            .color(bal_col),
                    ),
            );
        bar = bar.child(bal_pill);
    }

    bar.into()
}

/// Output/input pill toggle matching the bluetooth popup button style and hover feedback.
fn target_btn(
    key: &'static str,
    label: &'static str,
    target: AudioControlTarget,
    hover_target: HoverTarget,
    hovered: bool,
    active: bool,
) -> Element<Msg> {
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
    } else if hovered {
        TEXT
    } else {
        SUBTEXT1
    };
    Group::new()
        .key(key)
        .size(Size::auto())
        .on_hover(Msg::Hover(hover_target))
        .on_leave(Msg::Leave(hover_target))
        .on_click(Msg::MixerSetTarget(target))
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(bg)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), border),
        )
        .child(
            Row::new()
                .padding(Edges::symmetric(2.logical_px(), 6.logical_px()))
                .align(Align::Center)
                .child(
                    Text::new(label)
                        .font(font_ui_semibold())
                        .font_size(10.logical_px())
                        .color(fg),
                ),
        )
        .into()
}

fn build_apps_card(rows: Vec<Element<Msg>>, body_height: f32) -> Element<Msg> {
    Group::new()
        .key("mixer-apps-card")
        .size(Size::new(Length::Fill, Length::Auto))
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(CRUST)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), SURFACE1),
        )
        .child(
            Column::new()
                .size(Size::new(Length::Fill, Length::Auto))
                .child(
                    Row::new()
                        .size(Size::new(Length::Fill, APPS_HEADER_HEIGHT.logical_px()))
                        .padding(Edges::symmetric(4.logical_px(), CARD_PAD_H.logical_px()))
                        .gap(6.logical_px())
                        .align(Align::Center)
                        .child(
                            Text::new("Per-apps:")
                                .font(font_ui_bold())
                                .font_size(10.5.logical_px())
                                .color(MAUVE),
                        ),
                )
                .child(
                    Clip::new(
                        Column::new()
                            .size(Size::new(Length::Fill, Length::Auto))
                            .padding(Edges::symmetric(2.logical_px(), 6.logical_px()))
                            .gap(ROW_GAP.logical_px())
                            .children(rows),
                    )
                    .size(Size::new(Length::Fill, body_height.logical_px()))
                    .on_scroll(|_| Msg::Noop),
                ),
        )
        .into()
}

struct AppRow {
    id: u32,
    name: String,
    linear: f32,
    muted: bool,
    icon: Option<ImageAsset>,
    capture: bool,
}

/// Rows follow the active master target: only playback streams are listed while the sink is
/// controlled and only capture streams while the source is, so an app is never listed against
/// the wrong direction (nor does scrolling one move the other).
fn build_app_rows(state: &State, max_body_height: f32) -> (Vec<Element<Msg>>, f32) {
    let mut app_rows: Vec<AppRow> = Vec::new();
    if let Some(audio) = state.audio.as_ref() {
        if state.audio_target == AudioControlTarget::Source {
            for c in audio.capture_streams() {
                let name = c.binary_name.clone().unwrap_or_else(|| c.name.clone());
                app_rows.push(AppRow {
                    id: c.id,
                    name,
                    linear: c.volume.linear,
                    muted: c.volume.muted,
                    icon: state.audio_ui.capture_icons.get(&c.id).copied().flatten(),
                    capture: true,
                });
            }
        } else {
            for s in audio.playback_streams() {
                let name = s.binary_name.clone().unwrap_or_else(|| s.name.clone());
                app_rows.push(AppRow {
                    id: s.id,
                    name,
                    linear: s.volume.linear,
                    muted: s.volume.muted,
                    icon: state.audio_ui.stream_icons.get(&s.id).copied().flatten(),
                    capture: false,
                });
            }
        }
    }

    if app_rows.is_empty() {
        let height = ROW_HEIGHT;
        let rows = vec![
            Row::new()
                .size(Size::new(Length::Fill, ROW_HEIGHT.logical_px()))
                .padding(Edges::symmetric(0.logical_px(), 8.logical_px()))
                .align(Align::Center)
                .child(
                    Text::new("No active apps")
                        .font(font_ui())
                        .font_size(11.logical_px())
                        .color(OVERLAY0),
                )
                .into(),
        ];
        return (rows, height);
    }

    let visible_slots =
        (((max_body_height - ROW_GAP) / (ROW_HEIGHT + ROW_GAP)).floor() as usize).max(1);
    let start = state
        .audio_ui
        .scroll_index
        .min(app_rows.len().saturating_sub(1));
    let shown: Vec<&AppRow> = app_rows.iter().skip(start).take(visible_slots).collect();
    let count = shown.len();
    let body_height =
        ((count as f32) * ROW_HEIGHT + (count.saturating_sub(1) as f32) * ROW_GAP + 14.0)
            .min(max_body_height);

    let rows = shown
        .iter()
        .map(|r| build_app_row(r, state.audio_ui.marquee_tick))
        .collect();
    (rows, body_height)
}

fn build_app_row(r: &AppRow, tick: u32) -> Element<Msg> {
    let name_text = marquee_slice(&r.name, (NAME_CLIP_WIDTH / 5.6) as usize, tick);
    let pct = (r.linear * 100.0).clamp(0.0, 150.0).round() as u16;
    let ratio = r.linear.clamp(0.0, 1.0);
    let fill_w = MIXER_TRACK_WIDTH * ratio;

    let icon: Element<Msg> = match r.icon {
        Some(img) => Image::new(img)
            .size(Size::new(ICON_PX.logical_px(), ICON_PX.logical_px()))
            .fit(ImageFit::Contain)
            .position(Position::relative_to_parent(
                Percent::new(50.0),
                Percent::new(50.0),
            ))
            .anchor(ElementAnchor::Center)
            .into(),
        None => Text::new("\u{f001}")
            .font(font_ui())
            .font_size(10.logical_px())
            .bounds_mode(TextBoundsMode::Optical)
            .color(OVERLAY0)
            .position(Position::relative_to_parent(
                Percent::new(50.0),
                Percent::new(50.0),
            ))
            .anchor(ElementAnchor::Center)
            .into(),
    };

    let capture = r.capture;
    let scroll = {
        let id = r.id;
        move |sc: ScrollEvent| {
            if capture {
                Msg::MixerCaptureScroll { id, sc }
            } else {
                Msg::MixerStreamScroll { id, sc }
            }
        }
    };
    let pointer = {
        let id = r.id;
        move |ev: PointerEvent| {
            if capture {
                Msg::MixerCaptureSeekClick { id, ev }
            } else {
                Msg::MixerStreamSeekClick { id, ev }
            }
        }
    };

    let key = if r.capture {
        ("mixer-capture-row", r.id)
    } else {
        ("mixer-stream-row", r.id)
    };

    Group::new()
        .key(key)
        .size(Size::new(Length::Fill, ROW_HEIGHT.logical_px()))
        .on_scroll(scroll)
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(MANTLE)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), SURFACE0),
        )
        .child(
            Row::new()
                .size(Size::fill())
                .padding(Edges::symmetric(0.logical_px(), 8.logical_px()))
                .gap(8.logical_px())
                .align(Align::Center)
                .child(
                    Group::new()
                        .size(Size::new(
                            ICON_BADGE_PX.logical_px(),
                            ICON_BADGE_PX.logical_px(),
                        ))
                        .child(icon),
                )
                .child(
                    Column::new()
                        .size(Size::new(Length::Fill, Length::Auto))
                        .gap(3.logical_px())
                        .child(
                            Row::new()
                                .size(Size::new(Length::Fill, Length::Auto))
                                .align(Align::Center)
                                .child(
                                    Clip::new(
                                        Text::new(name_text)
                                            .font(font_ui_semibold())
                                            .font_size(10.logical_px())
                                            .color(TEXT),
                                    )
                                    .size(Size::new(NAME_CLIP_WIDTH.logical_px(), 13.logical_px())),
                                )
                                .child(
                                    Text::new(if r.muted {
                                        String::from("muted")
                                    } else {
                                        format!("{pct}%")
                                    })
                                    .font(font_ui_semibold())
                                    .font_size(9.5.logical_px())
                                    .color(if r.muted {
                                        OVERLAY0
                                    } else {
                                        SUBTEXT1
                                    }),
                                ),
                        )
                        .child(
                            Group::new()
                                .size(Size::new(MIXER_TRACK_WIDTH.logical_px(), 5.logical_px()))
                                .on_pointer_down(pointer)
                                .on_scroll(scroll)
                                .child(Rect::new().size(Size::fill()).fill(SURFACE0))
                                .child(
                                    Rect::new()
                                        .size(Size::new(fill_w.logical_px(), 5.logical_px()))
                                        .fill(if r.muted { OVERLAY0 } else { MAUVE }),
                                ),
                        ),
                ),
        )
        .into()
}
