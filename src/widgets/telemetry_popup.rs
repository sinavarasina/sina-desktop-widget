//! Reusable Catppuccin Mocha live telemetry graph popup (`CPU`, `Memory`, `GPU`, `Network`).
//!
//! Built around a generic [`build_line_chart`] `Canvas` primitive so all four telemetry views share
//! one crisp vector area/line renderer, one tab switcher, and one uniform stat-card grid.

use waywi::geometry::Bounds as GeoRect;
use waywi::prelude::*;
use waywi::system::{DataRate, FreqUnit, NetInterfaceKind, RateUnit};

use crate::app::state::{HoverTarget, Msg, State};
use crate::app::telemetry_graph::{GRAPH_HISTORY_CAP, TelemetryGraphKind};
use crate::theme::{
    BAR_HEIGHT, BLUE, CRUST, GREEN, MANTLE, MAUVE, NOTIF_CENTER_TOP_MARGIN, OVERLAY0, PEACH, RED,
    SUBTEXT0, SUBTEXT1, SURFACE0, SURFACE1, TEXT, font_ui, font_ui_semibold,
};

const POPUP_WIDTH: f32 = 346.0;
const HEADER_H: f32 = 34.0;
const TABS_H: f32 = 28.0;
const BODY_PAD: f32 = 10.0;
const CHART_W: f32 = POPUP_WIDTH - BODY_PAD * 2.0;
const CHART_H: f32 = 114.0;
const STAT_CARD_H: f32 = 38.0;
const STAT_GAP: f32 = 6.0;
const CORE_STRIP_H: f32 = 30.0;

/// Single plotted line/area series passed to the reusable [`build_line_chart`] widget.
pub struct ChartSeries<'a> {
    pub label: &'static str,
    pub values: &'a [f32],
    pub color: Color,
    pub fill_alpha: f32,
}

/// Configuration for the reusable [`build_line_chart`] `Canvas` plot.
pub struct ChartSpec<'a> {
    pub cache_key: (u8, u64),
    pub width: f32,
    pub height: f32,
    pub y_max: f32,
    pub y_top_label: String,
    pub y_mid_label: String,
    pub y_bot_label: String,
    pub span_label: &'static str,
    pub series: &'a [ChartSeries<'a>],
}

/// Reusable vector line + translucent area chart widget with Y-axis labels, gridlines, and legend.
pub fn build_line_chart(spec: ChartSpec<'_>) -> Element<Msg> {
    let w = spec.width;
    let h = spec.height;
    let y_max = spec.y_max.max(1.0);

    // Owned copies moved into the Canvas paint closure (small fixed arrays, <= 60 floats each).
    let owned_series: Vec<(Vec<f32>, Color, f32)> = spec
        .series
        .iter()
        .map(|s| (s.values.to_vec(), s.color, s.fill_alpha))
        .collect();

    let plot_canvas = Canvas::new(Size::new(w.logical_px(), h.logical_px()), move |p| {
        let pad_l = 4.0_f32;
        let pad_r = 4.0_f32;
        let pad_t = 14.0_f32;
        let pad_b = 14.0_f32;
        let plot_w = (w - pad_l - pad_r).max(1.0);
        let plot_h = (h - pad_t - pad_b).max(1.0);

        p.fill_rect(
            GeoRect::new(0.logical_px(), 0.logical_px(), w.logical_px(), h.logical_px()),
            MANTLE,
        );

        // Horizontal subtle gridlines at 0%, 25%, 50%, 75%, 100%.
        for step in 0..=4 {
            let gy = pad_t + plot_h * (step as f32 / 4.0);
            let mut grid = PathBuilder::new();
            grid.move_to(pad_l.logical_px(), gy.logical_px())
                .line_to((w - pad_r).logical_px(), gy.logical_px());
            let col = if step == 0 || step == 4 {
                SURFACE1.with_opacity(0.65)
            } else {
                SURFACE0.with_opacity(0.9)
            };
            p.stroke_path(&grid, 0.8.logical_px(), col);
        }

        // Subtle vertical quarter gridlines.
        for step in 1..4 {
            let gx = pad_l + plot_w * (step as f32 / 4.0);
            let mut grid = PathBuilder::new();
            grid.move_to(gx.logical_px(), pad_t.logical_px())
                .line_to(gx.logical_px(), (pad_t + plot_h).logical_px());
            p.stroke_path(&grid, 0.7.logical_px(), SURFACE0.with_opacity(0.55));
        }

        let denom = (GRAPH_HISTORY_CAP.saturating_sub(1)).max(1) as f32;
        let baseline_y = pad_t + plot_h;

        for (vals, color, fill_alpha) in &owned_series {
            if vals.is_empty() {
                continue;
            }
            let n = vals.len();
            // Right-align samples so fresh telemetry enters smoothly from the right edge.
            let x_of = |i: usize| -> f32 {
                let slot = (GRAPH_HISTORY_CAP - n) + i;
                pad_l + (slot as f32 / denom) * plot_w
            };
            let y_of = |v: f32| -> f32 {
                let norm = (v / y_max).clamp(0.0, 1.0);
                baseline_y - norm * plot_h
            };

            if n >= 2 {
                if *fill_alpha > 0.0 {
                    let mut fill_pb = PathBuilder::new();
                    fill_pb.move_to(x_of(0).logical_px(), baseline_y.logical_px());
                    for (i, &v) in vals.iter().enumerate() {
                        fill_pb.line_to(x_of(i).logical_px(), y_of(v).logical_px());
                    }
                    fill_pb
                        .line_to(x_of(n - 1).logical_px(), baseline_y.logical_px())
                        .close();
                    p.fill_path(&fill_pb, color.with_opacity(*fill_alpha));
                }

                let mut line_pb = PathBuilder::new();
                line_pb.move_to(x_of(0).logical_px(), y_of(vals[0]).logical_px());
                for (i, &v) in vals.iter().enumerate().skip(1) {
                    line_pb.line_to(x_of(i).logical_px(), y_of(v).logical_px());
                }
                p.stroke_path(&line_pb, 1.6.logical_px(), *color);
            }

            // Sharp 4x4 square marker at the latest sample point.
            let lx = x_of(n - 1);
            let ly = y_of(vals[n - 1]);
            p.fill_rect(
                GeoRect::new(
                    (lx - 2.0).logical_px(),
                    (ly - 2.0).logical_px(),
                    4.logical_px(),
                    4.logical_px(),
                ),
                *color,
            );
        }

        // Crisp 1px outer frame.
        let mut border = PathBuilder::new();
        border
            .move_to(0.5.logical_px(), 0.5.logical_px())
            .line_to((w - 0.5).logical_px(), 0.5.logical_px())
            .line_to((w - 0.5).logical_px(), (h - 0.5).logical_px())
            .line_to(0.5.logical_px(), (h - 0.5).logical_px())
            .close();
        p.stroke_path(&border, 1.0.logical_px(), SURFACE1);
    })
    .cache_key(("telemetry-chart", spec.cache_key))
    .key("telemetry-chart-canvas");

    let mut legend = Row::new()
        .size(Size::auto())
        .gap(8.logical_px())
        .align(Align::Center)
        .position(Position::absolute((w - 6.0).logical_px(), 3.logical_px()))
        .anchor(ElementAnchor::TopRight);

    for s in spec.series {
        legend = legend.child(
            Row::new()
                .size(Size::auto())
                .gap(3.logical_px())
                .align(Align::Center)
                .child(
                    Rect::new()
                        .size(Size::new(6.logical_px(), 2.logical_px()))
                        .fill(s.color),
                )
                .child(
                    Text::new(s.label)
                        .font(font_ui_semibold())
                        .font_size(8.logical_px())
                        .color(SUBTEXT1),
                ),
        );
    }

    Group::new()
        .key("telemetry-chart-box")
        .size(Size::new(w.logical_px(), h.logical_px()))
        .child(plot_canvas)
        .child(
            Text::new(spec.y_top_label)
                .font(font_ui())
                .font_size(8.logical_px())
                .color(OVERLAY0)
                .position(Position::absolute(6.logical_px(), 2.logical_px())),
        )
        .child(
            Text::new(spec.y_mid_label)
                .font(font_ui())
                .font_size(8.logical_px())
                .color(OVERLAY0)
                .position(Position::absolute(6.logical_px(), (h * 0.5 - 5.0).logical_px())),
        )
        .child(
            Text::new(spec.y_bot_label)
                .font(font_ui())
                .font_size(8.logical_px())
                .color(OVERLAY0)
                .position(Position::absolute(6.logical_px(), (h - 12.0).logical_px())),
        )
        .child(
            Text::new(spec.span_label)
                .font(font_ui())
                .font_size(8.logical_px())
                .color(OVERLAY0)
                .position(Position::absolute((w - 6.0).logical_px(), (h - 12.0).logical_px()))
                .anchor(ElementAnchor::TopRight),
        )
        .child(legend)
        .into()
}

pub fn build_telemetry_popup_surface(state: &State) -> Surface<Msg> {
    let ease = state.telemetry_graph.popup_progress();
    let kind = state.telemetry_graph.active;
    let accent = kind_accent(kind);

    let show_core_strip = kind == TelemetryGraphKind::Cpu
        && state
            .cpu
            .as_ref()
            .is_some_and(|c| !c.per_core_percent.is_empty());

    let core_extra_h = if show_core_strip {
        CORE_STRIP_H + STAT_GAP
    } else {
        0.0
    };

    let body_h = BODY_PAD * 2.0
        + CHART_H
        + STAT_GAP
        + STAT_CARD_H * 2.0
        + STAT_GAP
        + core_extra_h;
    let full_h = HEADER_H + 1.0 + TABS_H + 1.0 + body_h;
    let animated_h = (full_h * ease).max(1.0);
    let animated_y = (BAR_HEIGHT - 8.0) + (NOTIF_CENTER_TOP_MARGIN - (BAR_HEIGHT - 8.0)) * ease;

    let fallback_left = (state.bar_width - POPUP_WIDTH - 140.0).max(8.0);
    let left_x = if state.telemetry_graph.popup_left_x > 0.0 {
        state.telemetry_graph.popup_left_x
    } else {
        fallback_left
    }
    .clamp(8.0, (state.bar_width - POPUP_WIDTH - 8.0).max(8.0));

    let (hero_parts, chart_el, stats) = build_kind_content(state, kind, accent);

    let mut body_col = Column::new()
        .size(Size::new(Length::Fill, Length::Auto))
        .padding(Edges::all(BODY_PAD.logical_px()))
        .gap(STAT_GAP.logical_px())
        .child(chart_el)
        .child(build_stat_row(&stats[0], &stats[1]))
        .child(build_stat_row(&stats[2], &stats[3]));

    if show_core_strip
        && let Some(cpu) = &state.cpu
    {
        body_col = body_col.child(build_cpu_core_strip(
            &cpu.per_core_percent,
            state.telemetry_graph.sample_seq,
        ));
    }

    let card_inner = Group::new()
        .size(Size::new(POPUP_WIDTH.logical_px(), full_h.logical_px()))
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(CRUST)
                .corner_radius(0.logical_px()),
        )
        .child(
            Column::new()
                .size(Size::new(Length::Fill, Length::Auto))
                .child(build_header(kind, accent, &hero_parts))
                .child(
                    Rect::new()
                        .size(Size::new(Length::Fill, 1.logical_px()))
                        .fill(SURFACE1),
                )
                .child(build_tab_strip(state, kind))
                .child(
                    Rect::new()
                        .size(Size::new(Length::Fill, 1.logical_px()))
                        .fill(SURFACE1),
                )
                .child(body_col),
        )
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(Color::TRANSPARENT)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), SURFACE1),
        );

    let animated_box = Clip::new(card_inner)
        .key("telemetry-popup-box")
        .position(Position::absolute(
            left_x.logical_px(),
            animated_y.logical_px(),
        ))
        .size(Size::new(POPUP_WIDTH.logical_px(), animated_h.logical_px()))
        .on_pointer_down(|_| Msg::Noop);

    let backdrop = Group::new()
        .key("telemetry-popup-backdrop")
        .size(Size::fill())
        .on_pointer_down(|_| Msg::CloseTelemetryPopup)
        .child(animated_box);

    Surface::floating("bar-telemetry-popup", Output::Primary)
        .layer(Layer::Overlay)
        .ignore_exclusive_zone()
        .anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT)
        .size(state.viewport_size())
        .input_mode(InputMode::Full)
        .keyboard_interactivity(KeyboardInteractivity::OnDemand)
        .background(Color::TRANSPARENT)
        .on_close(Msg::CloseTelemetryPopup)
        .on_key(|ev| (ev.key == Key::Escape).then_some(Msg::CloseTelemetryPopup))
        .child(backdrop)
}

const fn kind_accent(kind: TelemetryGraphKind) -> Color {
    match kind {
        TelemetryGraphKind::Cpu => MAUVE,
        TelemetryGraphKind::Memory => BLUE,
        TelemetryGraphKind::Gpu => PEACH,
        TelemetryGraphKind::Network => GREEN,
    }
}

fn build_header(
    kind: TelemetryGraphKind,
    accent: Color,
    hero_parts: &[(String, Color)],
) -> Element<Msg> {
    let mut right_row = Row::new()
        .size(Size::auto())
        .gap(8.logical_px())
        .align(Align::Center)
        .position(Position::relative_to_parent(
            Percent::new(100.0),
            Percent::new(50.0),
        ))
        .anchor(ElementAnchor::CenterRight)
        .padding(Edges::new(
            0.logical_px(),
            10.logical_px(),
            0.logical_px(),
            0.logical_px(),
        ));

    for (val, col) in hero_parts {
        right_row = right_row.child(
            Text::new(val.clone())
                .font(font_ui_semibold())
                .font_size(11.logical_px())
                .color(*col),
        );
    }

    Group::new()
        .size(Size::new(Length::Fill, HEADER_H.logical_px()))
        .child(
            Row::new()
                .size(Size::auto())
                .gap(6.logical_px())
                .align(Align::Center)
                .position(Position::relative_to_parent(
                    Percent::new(0.0),
                    Percent::new(50.0),
                ))
                .anchor(ElementAnchor::CenterLeft)
                .padding(Edges::new(
                    0.logical_px(),
                    0.logical_px(),
                    0.logical_px(),
                    10.logical_px(),
                ))
                .child(
                    Text::new(kind.icon())
                        .font(font_ui())
                        .font_size(11.logical_px())
                        .bounds_mode(TextBoundsMode::Optical)
                        .color(accent),
                )
                .child(
                    Text::new(kind.title())
                        .font(font_ui_semibold())
                        .font_size(11.logical_px())
                        .color(TEXT),
                ),
        )
        .child(right_row)
        .into()
}

fn build_tab_strip(state: &State, active: TelemetryGraphKind) -> Element<Msg> {
    let tab_w = (POPUP_WIDTH - 2.0) / 4.0;
    let mut row = Row::new()
        .size(Size::new(Length::Fill, TABS_H.logical_px()))
        .padding(Edges::symmetric(0.logical_px(), 1.logical_px()))
        .align(Align::Center);

    for kind in TelemetryGraphKind::ALL {
        let is_active = kind == active;
        let hovered = state.hovered == HoverTarget::TelemetryTab(kind);
        let accent = kind_accent(kind);
        let bg = if is_active {
            MANTLE
        } else if hovered {
            SURFACE0
        } else {
            CRUST
        };

        let mut tab = Group::new()
            .key(("telemetry-tab", kind.id_u8()))
            .size(Size::new(tab_w.logical_px(), TABS_H.logical_px()))
            .cursor(CursorShape::Pointer)
            .on_hover(Msg::Hover(HoverTarget::TelemetryTab(kind)))
            .on_leave(Msg::Leave(HoverTarget::TelemetryTab(kind)))
            .on_click(Msg::TelemetrySelectKind(kind))
            .child(Rect::new().size(Size::fill()).fill(bg))
            .child(
                Row::new()
                    .size(Size::auto())
                    .gap(5.logical_px())
                    .align(Align::Center)
                    .position(Position::relative_to_parent(
                        Percent::new(50.0),
                        Percent::new(50.0),
                    ))
                    .anchor(ElementAnchor::Center)
                    .child(
                        Text::new(kind.icon())
                            .font(font_ui())
                            .font_size(10.logical_px())
                            .bounds_mode(TextBoundsMode::Optical)
                            .color(if is_active || hovered { accent } else { SUBTEXT0 }),
                    )
                    .child(
                        Text::new(kind.tab_label())
                            .font(font_ui_semibold())
                            .font_size(9.5.logical_px())
                            .color(if is_active {
                                TEXT
                            } else if hovered {
                                SUBTEXT1
                            } else {
                                SUBTEXT0
                            }),
                    ),
            );

        if is_active {
            tab = tab.child(
                Rect::new()
                    .position(Position::absolute(0.logical_px(), (TABS_H - 2.0).logical_px()))
                    .size(Size::new(tab_w.logical_px(), 2.logical_px()))
                    .fill(accent),
            );
        }

        row = row.child(tab);
    }

    row.into()
}

struct StatItem {
    label: &'static str,
    value: String,
    accent: Color,
}

fn build_stat_row(left: &StatItem, right: &StatItem) -> Element<Msg> {
    let card_w = (CHART_W - STAT_GAP) * 0.5;
    Row::new()
        .size(Size::new(CHART_W.logical_px(), STAT_CARD_H.logical_px()))
        .gap(STAT_GAP.logical_px())
        .child(build_stat_card(card_w, left))
        .child(build_stat_card(card_w, right))
        .into()
}

fn build_stat_card(width: f32, item: &StatItem) -> Element<Msg> {
    Group::new()
        .size(Size::new(width.logical_px(), STAT_CARD_H.logical_px()))
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(MANTLE)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), SURFACE1),
        )
        .child(
            Column::new()
                .size(Size::fill())
                .padding(Edges::symmetric(5.logical_px(), 8.logical_px()))
                .gap(2.logical_px())
                .child(
                    Text::new(item.label)
                        .font(font_ui())
                        .font_size(8.5.logical_px())
                        .color(SUBTEXT0),
                )
                .child(
                    Text::new(item.value.clone())
                        .font(font_ui_semibold())
                        .font_size(10.logical_px())
                        .color(item.accent),
                ),
        )
        .into()
}

/// Compact per-core load mini-bar strip shown beneath CPU stats.
fn build_cpu_core_strip(cores: &[f32], seq: u64) -> Element<Msg> {
    let owned: Vec<f32> = cores.to_vec();
    let w = CHART_W;
    let h = CORE_STRIP_H;
    let canvas = Canvas::new(Size::new(w.logical_px(), h.logical_px()), move |p| {
        p.fill_rect(
            GeoRect::new(0.logical_px(), 0.logical_px(), w.logical_px(), h.logical_px()),
            MANTLE,
        );
        let pad_x = 6.0_f32;
        let pad_y = 4.0_f32;
        let avail_w = (w - pad_x * 2.0).max(1.0);
        let avail_h = (h - pad_y * 2.0).max(1.0);
        let n = owned.len().max(1);
        let gap = if n > 16 { 1.5_f32 } else { 3.0_f32 };
        let bar_w = ((avail_w - gap * (n.saturating_sub(1) as f32)) / n as f32).max(2.0);

        for (i, &pct) in owned.iter().enumerate() {
            let x = pad_x + i as f32 * (bar_w + gap);
            let norm = (pct / 100.0).clamp(0.0, 1.0);
            p.fill_rect(
                GeoRect::new(
                    x.logical_px(),
                    pad_y.logical_px(),
                    bar_w.logical_px(),
                    avail_h.logical_px(),
                ),
                SURFACE0,
            );
            let fill_h = (norm * avail_h).max(1.0);
            let col = if pct >= 85.0 {
                RED
            } else if pct >= 60.0 {
                PEACH
            } else {
                MAUVE
            };
            p.fill_rect(
                GeoRect::new(
                    x.logical_px(),
                    (pad_y + avail_h - fill_h).logical_px(),
                    bar_w.logical_px(),
                    fill_h.logical_px(),
                ),
                col,
            );
        }

        let mut border = PathBuilder::new();
        border
            .move_to(0.5.logical_px(), 0.5.logical_px())
            .line_to((w - 0.5).logical_px(), 0.5.logical_px())
            .line_to((w - 0.5).logical_px(), (h - 0.5).logical_px())
            .line_to(0.5.logical_px(), (h - 0.5).logical_px())
            .close();
        p.stroke_path(&border, 1.0.logical_px(), SURFACE1);
    })
    .cache_key(("cpu-cores", seq, cores.len()))
    .key("telemetry-cpu-cores");

    canvas.into()
}

fn build_kind_content(
    state: &State,
    kind: TelemetryGraphKind,
    accent: Color,
) -> (Vec<(String, Color)>, Element<Msg>, [StatItem; 4]) {
    let g = &state.telemetry_graph;
    let seq = g.sample_seq;
    let mut buf_a = [0.0_f32; GRAPH_HISTORY_CAP];
    let mut buf_b = [0.0_f32; GRAPH_HISTORY_CAP];

    match kind {
        TelemetryGraphKind::Cpu => {
            let n = g.cpu_usage.copy_chronological(&mut buf_a);
            let usage = state.cpu.as_ref().map_or(0.0, |c| c.total_percent);
            let avg_ghz = state
                .cpu
                .as_ref()
                .and_then(|c| c.avg_freq(FreqUnit::Ghz))
                .unwrap_or(0.0);
            let max_ghz = state
                .cpu
                .as_ref()
                .and_then(|c| c.max_freq(FreqUnit::Ghz))
                .unwrap_or(0.0);
            let load1 = state.cpu.as_ref().map_or(0.0, |c| c.load_avg.one);
            let load5 = state.cpu.as_ref().map_or(0.0, |c| c.load_avg.five);
            let temp_str = state
                .cpu_temp_celsius
                .map_or_else(|| String::from("--"), |t| format!("{t:.0}°C"));
            let power_str = state
                .cpu_power_watts
                .map_or_else(|| String::from("--"), |w| format!("{w:.1} W"));

            let chart = build_line_chart(ChartSpec {
                cache_key: (kind.id_u8(), seq),
                width: CHART_W,
                height: CHART_H,
                y_max: 100.0,
                y_top_label: String::from("100%"),
                y_mid_label: String::from("50%"),
                y_bot_label: String::from("0%"),
                span_label: "60s",
                series: &[ChartSeries {
                    label: "CPU",
                    values: &buf_a[..n],
                    color: accent,
                    fill_alpha: 0.20,
                }],
            });
            let stats = [
                StatItem {
                    label: "Frequency (Avg / Max)",
                    value: format!("{avg_ghz:.2} / {max_ghz:.2} GHz"),
                    accent: TEXT,
                },
                StatItem {
                    label: "Load Average (1m / 5m)",
                    value: format!("{load1:.2} / {load5:.2}"),
                    accent: TEXT,
                },
                StatItem {
                    label: "Package Temp",
                    value: temp_str,
                    accent: PEACH,
                },
                StatItem {
                    label: "Package Power",
                    value: power_str,
                    accent: BLUE,
                },
            ];
            (vec![(format!("{:.0}%", usage), accent)], chart, stats)
        }
        TelemetryGraphKind::Memory => {
            let n_ram = g.mem_usage.copy_chronological(&mut buf_a);
            let n_swap = g.swap_usage.copy_chronological(&mut buf_b);
            let (pct, used_g, total_g, avail_g, swap_u, swap_t) = match &state.memory {
                Some(m) => {
                    let tb = m.total.as_bytes() as f64;
                    let p = if tb > 0.0 {
                        (m.used.as_bytes() as f64 / tb) * 100.0
                    } else {
                        0.0
                    };
                    (
                        p,
                        m.used.as_gib(),
                        m.total.as_gib(),
                        m.available.as_gib(),
                        m.swap_used.as_gib(),
                        m.swap_total.as_gib(),
                    )
                }
                None => (0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            };
            let peak_pct = g.mem_usage.peak();

            let chart = build_line_chart(ChartSpec {
                cache_key: (kind.id_u8(), seq),
                width: CHART_W,
                height: CHART_H,
                y_max: 100.0,
                y_top_label: String::from("100%"),
                y_mid_label: String::from("50%"),
                y_bot_label: String::from("0%"),
                span_label: "60s",
                series: &[
                    ChartSeries {
                        label: "RAM",
                        values: &buf_a[..n_ram],
                        color: BLUE,
                        fill_alpha: 0.20,
                    },
                    ChartSeries {
                        label: "Swap",
                        values: &buf_b[..n_swap],
                        color: PEACH,
                        fill_alpha: 0.08,
                    },
                ],
            });
            let stats = [
                StatItem {
                    label: "RAM Used / Total",
                    value: format!("{used_g:.1} / {total_g:.1} GiB"),
                    accent: BLUE,
                },
                StatItem {
                    label: "Available",
                    value: format!("{avail_g:.1} GiB"),
                    accent: GREEN,
                },
                StatItem {
                    label: "Swap Used / Total",
                    value: format!("{swap_u:.1} / {swap_t:.1} GiB"),
                    accent: PEACH,
                },
                StatItem {
                    label: "60s Peak Usage",
                    value: format!("{peak_pct:.0}%"),
                    accent: TEXT,
                },
            ];
            (
                vec![(format!("{pct:.0}% · {used_g:.1}G"), accent)],
                chart,
                stats,
            )
        }
        TelemetryGraphKind::Gpu => {
            let n_util = g.gpu_usage.copy_chronological(&mut buf_a);
            let n_vram = g.gpu_vram.copy_chronological(&mut buf_b);
            let dev = state.gpu.as_ref().and_then(|s| s.gpus.first());
            let util = dev.and_then(|d| d.utilization_percent);
            let sclk = dev.and_then(|d| d.core_clock_mhz).unwrap_or(0);
            let mclk = dev.and_then(|d| d.memory_clock_mhz);
            let temp_str = match (
                dev.and_then(|d| d.temperature_celsius),
                dev.and_then(|d| d.hotspot_celsius),
            ) {
                (Some(t), Some(h)) => format!("{t:.0}°C · {h:.0}°C hot"),
                (Some(t), None) => format!("{t:.0}°C"),
                _ => String::from("--"),
            };
            let power_str = dev
                .and_then(|d| d.power_watts)
                .map_or_else(|| String::from("--"), |w| format!("{w:.1} W"));
            let vram_str = match (dev.and_then(|d| d.vram_used), dev.and_then(|d| d.vram_total)) {
                (Some(u), Some(t)) => format!("{:.1} / {:.1} GiB", u.as_gib(), t.as_gib()),
                _ => String::from("--"),
            };
            let clock_str = match mclk {
                Some(m) => format!("{sclk} / {m} MHz"),
                None => format!("{sclk} MHz"),
            };

            let has_vram = dev.is_some_and(|d| d.vram_total.is_some());
            let mut series_vec = vec![ChartSeries {
                label: "Core",
                values: &buf_a[..n_util],
                color: PEACH,
                fill_alpha: 0.22,
            }];
            if has_vram {
                series_vec.push(ChartSeries {
                    label: "VRAM",
                    values: &buf_b[..n_vram],
                    color: MAUVE,
                    fill_alpha: 0.10,
                });
            }

            let chart = build_line_chart(ChartSpec {
                cache_key: (kind.id_u8(), seq),
                width: CHART_W,
                height: CHART_H,
                y_max: 100.0,
                y_top_label: String::from("100%"),
                y_mid_label: String::from("50%"),
                y_bot_label: String::from("0%"),
                span_label: "2m",
                series: &series_vec,
            });
            let hero = match util {
                Some(u) => format!("{u:.0}% · {sclk}MHz"),
                None => format!("{sclk}MHz"),
            };
            let stats = [
                StatItem {
                    label: "Core / Mem Clock",
                    value: clock_str,
                    accent: TEXT,
                },
                StatItem {
                    label: "VRAM Used / Total",
                    value: vram_str,
                    accent: MAUVE,
                },
                StatItem {
                    label: "Temperature",
                    value: temp_str,
                    accent: PEACH,
                },
                StatItem {
                    label: "Power Draw",
                    value: power_str,
                    accent: BLUE,
                },
            ];
            (vec![(hero, accent)], chart, stats)
        }
        TelemetryGraphKind::Network => {
            let n_rx = g.net_rx_bps.copy_chronological(&mut buf_a);
            let n_tx = g.net_tx_bps.copy_chronological(&mut buf_b);
            let peak_bps = g
                .net_rx_bps
                .peak()
                .max(g.net_tx_bps.peak())
                .max(1024.0);
            // Round ceiling cleanly upward for stable Y-axis labels.
            let y_max = nice_rate_ceiling(peak_bps);

            let iface = state.network.as_ref().and_then(|n| n.active_interface());
            let rx_now = iface.map_or(0.0, |i| i.rx_bytes_per_sec);
            let tx_now = iface.map_or(0.0, |i| i.tx_bytes_per_sec);
            let rx_str = DataRate::from_bytes_per_sec(rx_now).format(RateUnit::AutoBytes);
            let tx_str = DataRate::from_bytes_per_sec(tx_now).format(RateUnit::AutoBytes);

            let (iface_str, detail_label, detail_val) = if let Some(i) = iface {
                let nm_dev = state
                    .nm
                    .as_ref()
                    .and_then(|nm| nm.devices.iter().find(|d| d.interface == i.name));
                let ip = nm_dev
                    .and_then(|d| d.ip4_address.as_deref())
                    .or(i.ipv4.as_deref())
                    .unwrap_or("0.0.0.0");
                let info = format!("{} · {ip}", i.name);
                match i.kind {
                    NetInterfaceKind::Wifi => {
                        let ssid = nm_dev
                            .and_then(|d| d.active_ssid.as_deref())
                            .unwrap_or("Wi-Fi");
                        let sig = nm_dev.and_then(|d| d.strength).unwrap_or(0);
                        (info, "Wi-Fi Signal", format!("{ssid} ({sig}%)"))
                    }
                    _ => {
                        let gw = i.gateway.as_deref().unwrap_or("--");
                        (info, "Gateway", gw.to_owned())
                    }
                }
            } else {
                (
                    String::from("Disconnected"),
                    "Gateway",
                    String::from("--"),
                )
            };

            let chart = build_line_chart(ChartSpec {
                cache_key: (kind.id_u8(), seq),
                width: CHART_W,
                height: CHART_H,
                y_max,
                y_top_label: DataRate::from_bytes_per_sec(f64::from(y_max))
                    .format(RateUnit::AutoBytes),
                y_mid_label: DataRate::from_bytes_per_sec(f64::from(y_max * 0.5))
                    .format(RateUnit::AutoBytes),
                y_bot_label: String::from("0B/s"),
                span_label: "60s",
                series: &[
                    ChartSeries {
                        label: "RX ↓",
                        values: &buf_a[..n_rx],
                        color: GREEN,
                        fill_alpha: 0.20,
                    },
                    ChartSeries {
                        label: "TX ↑",
                        values: &buf_b[..n_tx],
                        color: BLUE,
                        fill_alpha: 0.12,
                    },
                ],
            });
            let stats = [
                StatItem {
                    label: "Download (RX)",
                    value: rx_str.clone(),
                    accent: GREEN,
                },
                StatItem {
                    label: "Upload (TX)",
                    value: tx_str.clone(),
                    accent: BLUE,
                },
                StatItem {
                    label: "Interface · IP",
                    value: iface_str,
                    accent: TEXT,
                },
                StatItem {
                    label: detail_label,
                    value: detail_val,
                    accent: SUBTEXT1,
                },
            ];
            (
                vec![
                    (format!("↓ {rx_str}"), GREEN),
                    (format!("↑ {tx_str}"), BLUE),
                ],
                chart,
                stats,
            )
        }
    }
}

/// Rounds peak bytes/sec up to a clean binary step (`1 KiB/s`, `10 KiB/s`, `100 KiB/s`, `1 MiB/s`, ...).
#[must_use]
fn nice_rate_ceiling(peak_bps: f32) -> f32 {
    const STEPS: [f32; 14] = [
        1024.0,
        4.0 * 1024.0,
        16.0 * 1024.0,
        64.0 * 1024.0,
        256.0 * 1024.0,
        1024.0 * 1024.0,
        4.0 * 1024.0 * 1024.0,
        16.0 * 1024.0 * 1024.0,
        64.0 * 1024.0 * 1024.0,
        128.0 * 1024.0 * 1024.0,
        256.0 * 1024.0 * 1024.0,
        512.0 * 1024.0 * 1024.0,
        1024.0 * 1024.0 * 1024.0,
        4.0 * 1024.0 * 1024.0 * 1024.0,
    ];
    for step in STEPS {
        if peak_bps <= step {
            return step;
        }
    }
    peak_bps * 1.2
}
