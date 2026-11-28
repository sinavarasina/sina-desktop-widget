//! Right-bar real-time audio visualizer (`\ [visualizer] \ \ [Privacy] ...`) styled to mirror
//! the transparent left-bar incoming notification ticker.
//!
//! Supports 5 studio-grade modes (Spectrum Bars, Mirror Bars, Oscilloscope, Stereo VU, Wave
//! Spectrum) rendered on a single cached `Canvas` using WayWi v0.3 `LinearGradient` fills.
//! Unmounts completely (0 px, 0 wakeups, 0 capture threads) when disabled or silent.

use waywi::geometry::Bounds as GeoRect;
use waywi::prelude::*;

use crate::app::state::{HoverTarget, Msg, State};
use crate::app::visualizer::{SCOPE_POINTS, VIZ_BANDS, VisualizerMode};
use crate::theme::{
    BAR_HEIGHT, BLUE, GREEN, MAUVE, OVERLAY0, PEACH, RED, SURFACE0, SURFACE1, TEXT, font_ui_semibold,
};
use crate::widgets::separators::{SepShape, separator};

/// Builds the transparent right-bar visualizer directly to the left of Privacy, or `None` when
/// disabled, hidden by responsive layout, or fully decayed in silence.
pub fn build_visualizer(state: &State) -> Option<Element<Msg>> {
    let layout = state.layout();
    if !state.config.widgets.show_visualizer
        || !layout.show_visualizer
        || !state.visualizer.is_visible()
    {
        return None;
    }

    let hovered = state.hovered == HoverTarget::VisualizerBar;
    let width = layout.visualizer_width;
    let height = BAR_HEIGHT;
    let mode = state.visualizer.mode;
    let presence = state.visualizer.presence.clamp(0.0, 1.0);
    let seq = state.visualizer.frame_seq;

    let bands = state.visualizer.bands;
    let band_peaks = state.visualizer.band_peaks;
    let scope = state.visualizer.scope;
    let vu_l = state.visualizer.vu_left;
    let vu_r = state.visualizer.vu_right;
    let vu_pl = state.visualizer.vu_peak_left;
    let vu_pr = state.visualizer.vu_peak_right;

    let canvas = Canvas::new(Size::new(width.logical_px(), height.logical_px()), move |p| {
        match mode {
            VisualizerMode::Bars => draw_spectrum_bars(p, width, height, &bands, &band_peaks),
            VisualizerMode::Mirror => draw_mirror_bars(p, width, height, &bands),
            VisualizerMode::Oscilloscope => draw_oscilloscope(p, width, height, &scope),
            VisualizerMode::StereoVu => draw_stereo_vu(p, width, height, vu_l, vu_r, vu_pl, vu_pr),
            VisualizerMode::Wave => draw_wave_spectrum(p, width, height, &bands),
        }
    })
    .cache_key(("bar-viz", mode.id_u8(), seq, width.to_bits()))
    .key("bar-visualizer-canvas");

    let mut body = Group::new()
        .key("bar-visualizer-body")
        .size((width.logical_px(), height.logical_px()))
        .opacity(presence)
        .cursor(CursorShape::Pointer)
        .on_hover(Msg::Hover(HoverTarget::VisualizerBar))
        .on_leave(Msg::Leave(HoverTarget::VisualizerBar))
        .on_pointer_down(Msg::VisualizerPointerDown)
        .child(canvas);

    if mode == VisualizerMode::StereoVu {
        body = body
            .child(
                Text::new("L")
                    .font(font_ui_semibold())
                    .font_size(7.logical_px())
                    .color(OVERLAY0)
                    .position(Position::absolute(2.logical_px(), 3.logical_px())),
            )
            .child(
                Text::new("R")
                    .font(font_ui_semibold())
                    .font_size(7.logical_px())
                    .color(OVERLAY0)
                    .position(Position::absolute(2.logical_px(), 13.logical_px())),
            );
    }

    let left_slash = separator(
        "sep-viz-l",
        SepShape::SlashLThin,
        if hovered { MAUVE } else { SURFACE0 },
        Color::TRANSPARENT,
    );

    Some(
        Row::new()
            .key("mod-visualizer")
            .size((Length::Auto, height.logical_px()))
            .align(Align::Center)
            .child(left_slash)
            .child(body)
            .into(),
    )
}

fn draw_spectrum_bars(
    p: &mut Painter<'_>,
    w: f32,
    h: f32,
    bands: &[f32; VIZ_BANDS],
    peaks: &[f32; VIZ_BANDS],
) {
    let pad_x = 4.0_f32;
    let pad_t = 3.0_f32;
    let pad_b = 3.0_f32;
    let plot_w = (w - pad_x * 2.0).max(8.0);
    let plot_h = (h - pad_t - pad_b).max(4.0);
    let base_y = pad_t + plot_h;

    // Subtle baseline hairline.
    p.fill_rect(
        GeoRect::new(
            pad_x.logical_px(),
            (base_y - 0.5).logical_px(),
            plot_w.logical_px(),
            1.logical_px(),
        ),
        SURFACE0.with_opacity(0.75),
    );

    let gap = 2.0_f32;
    let bar_w = ((plot_w - gap * (VIZ_BANDS - 1) as f32) / VIZ_BANDS as f32).max(2.0);
    let grad = LinearGradient::new(
        Point::new(0.logical_px(), 0.logical_px()),
        Point::new(0.logical_px(), 1.logical_px()),
        [
            GradientStop::new(0.0, PEACH),
            GradientStop::new(0.45, MAUVE),
            GradientStop::new(1.0, BLUE),
        ],
    );

    for i in 0..VIZ_BANDS {
        let x = pad_x + i as f32 * (bar_w + gap);
        let bh = (bands[i].clamp(0.0, 1.0) * plot_h).max(1.2);
        p.fill_rect(
            GeoRect::new(
                x.logical_px(),
                (base_y - bh).logical_px(),
                bar_w.logical_px(),
                bh.logical_px(),
            ),
            grad.clone(),
        );

        let pk = peaks[i].clamp(0.0, 1.0);
        if pk > 0.06 {
            let py = (base_y - pk * plot_h - 1.5).clamp(pad_t, base_y - 1.5);
            p.fill_rect(
                GeoRect::new(
                    x.logical_px(),
                    py.logical_px(),
                    bar_w.logical_px(),
                    1.3.logical_px(),
                ),
                TEXT.with_opacity(0.85),
            );
        }
    }
}

fn draw_mirror_bars(p: &mut Painter<'_>, w: f32, h: f32, bands: &[f32; VIZ_BANDS]) {
    let pad_x = 4.0_f32;
    let pad_y = 2.5_f32;
    let plot_w = (w - pad_x * 2.0).max(8.0);
    let half_h = (h * 0.5 - pad_y).max(2.0);
    let mid_y = h * 0.5;

    // Subtle center spine.
    p.fill_rect(
        GeoRect::new(
            pad_x.logical_px(),
            (mid_y - 0.5).logical_px(),
            plot_w.logical_px(),
            1.logical_px(),
        ),
        SURFACE1.with_opacity(0.6),
    );

    let gap = 2.0_f32;
    let bar_w = ((plot_w - gap * (VIZ_BANDS - 1) as f32) / VIZ_BANDS as f32).max(2.0);
    let grad = LinearGradient::new(
        Point::new(0.logical_px(), 0.logical_px()),
        Point::new(0.logical_px(), 1.logical_px()),
        [
            GradientStop::new(0.0, PEACH),
            GradientStop::new(0.5, MAUVE),
            GradientStop::new(1.0, BLUE),
        ],
    );

    for (i, &val) in bands.iter().enumerate() {
        let x = pad_x + i as f32 * (bar_w + gap);
        let ext = (val.clamp(0.0, 1.0) * half_h).max(1.0);
        p.fill_rect(
            GeoRect::new(
                x.logical_px(),
                (mid_y - ext).logical_px(),
                bar_w.logical_px(),
                (ext * 2.0).logical_px(),
            ),
            grad.clone(),
        );
    }
}

fn draw_oscilloscope(p: &mut Painter<'_>, w: f32, h: f32, scope: &[f32; SCOPE_POINTS]) {
    let pad_x = 4.0_f32;
    let pad_y = 3.0_f32;
    let plot_w = (w - pad_x * 2.0).max(8.0);
    let mid_y = h * 0.5;
    let amp_h = (h * 0.5 - pad_y).max(2.0);

    // Subtle zero-crossing axis.
    let mut axis = PathBuilder::new();
    axis.move_to(pad_x.logical_px(), mid_y.logical_px())
        .line_to((w - pad_x).logical_px(), mid_y.logical_px());
    p.stroke_path(&axis, 0.8.logical_px(), SURFACE0.with_opacity(0.85));

    let denom = (SCOPE_POINTS - 1) as f32;
    let x_of = |i: usize| pad_x + (i as f32 / denom) * plot_w;
    let y_of = |v: f32| mid_y - v.clamp(-1.0, 1.0) * amp_h;

    // Translucent ribbon fill back to center line.
    let mut fill_pb = PathBuilder::new();
    fill_pb.move_to(x_of(0).logical_px(), mid_y.logical_px());
    for (i, &v) in scope.iter().enumerate() {
        fill_pb.line_to(x_of(i).logical_px(), y_of(v).logical_px());
    }
    fill_pb
        .line_to(x_of(SCOPE_POINTS - 1).logical_px(), mid_y.logical_px())
        .close();
    p.fill_path(&fill_pb, MAUVE.with_opacity(0.18));

    let mut line_pb = PathBuilder::new();
    line_pb.move_to(x_of(0).logical_px(), y_of(scope[0]).logical_px());
    for (i, &v) in scope.iter().enumerate().skip(1) {
        line_pb.line_to(x_of(i).logical_px(), y_of(v).logical_px());
    }
    p.stroke_path(&line_pb, 1.5.logical_px(), MAUVE);
}

fn draw_stereo_vu(
    p: &mut Painter<'_>,
    w: f32,
    h: f32,
    vu_l: f32,
    vu_r: f32,
    peak_l: f32,
    peak_r: f32,
) {
    let left_x = 11.0_f32;
    let right_pad = 4.0_f32;
    let track_w = (w - left_x - right_pad).max(16.0);
    let seg_count = 16usize;
    let gap = 1.5_f32;
    let seg_w = ((track_w - gap * (seg_count - 1) as f32) / seg_count as f32).max(2.0);
    let bar_h = 5.0_f32;
    let y_l = (h * 0.5 - bar_h - 1.5).max(2.0);
    let y_r = (h * 0.5 + 1.5).min(h - bar_h - 2.0);

    let draw_channel = |p: &mut Painter<'_>, y: f32, level: f32, peak: f32| {
        let lit = (level.clamp(0.0, 1.0) * seg_count as f32).round() as usize;
        let peak_idx = if peak > 0.04 {
            Some(((peak.clamp(0.0, 1.0) * seg_count as f32).ceil() as usize).saturating_sub(1).min(seg_count - 1))
        } else {
            None
        };
        for i in 0..seg_count {
            let x = left_x + i as f32 * (seg_w + gap);
            let frac = (i + 1) as f32 / seg_count as f32;
            let active_col = if frac > 0.85 {
                RED
            } else if frac > 0.62 {
                PEACH
            } else {
                GREEN
            };
            let col = if i < lit {
                active_col
            } else if peak_idx == Some(i) {
                active_col.with_opacity(0.95)
            } else {
                SURFACE0.with_opacity(0.7)
            };
            p.fill_rect(
                GeoRect::new(x.logical_px(), y.logical_px(), seg_w.logical_px(), bar_h.logical_px()),
                col,
            );
        }
    };

    draw_channel(p, y_l, vu_l, peak_l);
    draw_channel(p, y_r, vu_r, peak_r);
}

fn draw_wave_spectrum(p: &mut Painter<'_>, w: f32, h: f32, bands: &[f32; VIZ_BANDS]) {
    let pad_x = 4.0_f32;
    let pad_t = 3.0_f32;
    let pad_b = 3.0_f32;
    let plot_w = (w - pad_x * 2.0).max(8.0);
    let plot_h = (h - pad_t - pad_b).max(4.0);
    let base_y = pad_t + plot_h;
    let denom = (VIZ_BANDS - 1) as f32;

    let x_of = |i: usize| pad_x + (i as f32 / denom) * plot_w;
    let y_of = |v: f32| base_y - v.clamp(0.0, 1.0) * plot_h;

    let mut fill_pb = PathBuilder::new();
    fill_pb.move_to(x_of(0).logical_px(), base_y.logical_px());
    for (i, &v) in bands.iter().enumerate() {
        fill_pb.line_to(x_of(i).logical_px(), y_of(v).logical_px());
    }
    fill_pb
        .line_to(x_of(VIZ_BANDS - 1).logical_px(), base_y.logical_px())
        .close();
    let grad = LinearGradient::new(
        Point::new(0.logical_px(), 0.logical_px()),
        Point::new(0.logical_px(), 1.logical_px()),
        [
            GradientStop::new(0.0, MAUVE.with_opacity(0.45)),
            GradientStop::new(1.0, BLUE.with_opacity(0.06)),
        ],
    );
    p.fill_path(&fill_pb, grad);

    let mut line_pb = PathBuilder::new();
    line_pb.move_to(x_of(0).logical_px(), y_of(bands[0]).logical_px());
    for (i, &v) in bands.iter().enumerate().skip(1) {
        line_pb.line_to(x_of(i).logical_px(), y_of(v).logical_px());
    }
    p.stroke_path(&line_pb, 1.5.logical_px(), MAUVE);
}
