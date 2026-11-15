//! Crisp vector Powerline, curve-notch, and slant separators matching `MesloLGS Nerd Font`.

use waywi::geometry::Rect as GeoRect;
use waywi::prelude::*;

use crate::theme::{BAR_HEIGHT, SEP_WIDTH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SepShape {
    /// Solid right-pointing Powerline triangle (`\u{e0b0}` ``).
    ArrowR,
    /// Solid left-pointing Powerline triangle (`\u{e0b2}` ``).
    ArrowL,
    /// Thin left-pointing chevron (`\u{e0b3}` ``).
    ArrowLThin,
    /// Solid block with left-facing triangular notch cutout (`\u{e0d7}` ``).
    CurveL,
    /// Solid block with right-facing triangular notch cutout (`\u{e0d6}` ``).
    CurveR,
    /// Solid upper-left diagonal triangle (`\u{e0bc}` ``).
    SlashR,
    /// Thin `/` diagonal stroke (`\u{e0bb}` ``).
    SlashRThin,
    /// Solid upper-right diagonal triangle (`\u{e0be}` ``).
    SlashL,
    /// Solid lower-right diagonal triangle (`\u{e0ba}` ``).
    SlashLInv,
}

/// Renders one separator matching the bar's `18px` MesloLGS separator geometry at full `24px` height.
pub fn separator<M: 'static>(
    key: &'static str,
    shape: SepShape,
    fg: Color,
    bg: Color,
) -> Element<M> {
    Canvas::new(
        Size::px(SEP_WIDTH.logical_px(), BAR_HEIGHT.logical_px()),
        (key, shape, fg, bg),
        move |p| draw_separator(p, shape, fg, bg),
    )
    .key(key)
    .into()
}

fn draw_separator(p: &mut Painter<'_>, shape: SepShape, fg: Color, bg: Color) {
    let w = SEP_WIDTH;
    let h = BAR_HEIGHT;
    let mid_y = h * 0.5;

    if bg.a > 0 {
        p.fill_rect(
            GeoRect::new(
                0.logical_px(),
                0.logical_px(),
                w.logical_px(),
                h.logical_px(),
            ),
            bg,
        );
    }

    match shape {
        SepShape::ArrowR => {
            fill_poly(p, fg, &[(-0.3, 0.0), (w, mid_y), (-0.3, h)]);
        }
        SepShape::ArrowL => {
            fill_poly(p, fg, &[(w + 0.3, 0.0), (0.0, mid_y), (w + 0.3, h)]);
        }
        SepShape::CurveL => {
            // Left notch opening toward ArrowR; right flat edge attaches to MANTLE workspaces pill.
            fill_poly(
                p,
                fg,
                &[
                    (0.0, 0.0),
                    (w + 0.3, 0.0),
                    (w + 0.3, h),
                    (0.0, h),
                    (w - 0.5, mid_y),
                ],
            );
        }
        SepShape::CurveR => {
            // Right notch opening toward ArrowLThin; left flat edge attaches to MANTLE drawer.
            fill_poly(
                p,
                fg,
                &[(-0.3, 0.0), (w, 0.0), (0.5, mid_y), (w, h), (-0.3, h)],
            );
        }
        SepShape::SlashR => {
            fill_poly(p, fg, &[(-0.3, 0.0), (w, 0.0), (-0.3, h)]);
        }
        SepShape::SlashL => {
            fill_poly(p, fg, &[(0.0, 0.0), (w + 0.3, 0.0), (w + 0.3, h)]);
        }
        SepShape::SlashLInv => {
            fill_poly(p, fg, &[(w + 0.3, 0.0), (w + 0.3, h), (0.0, h)]);
        }
        SepShape::SlashRThin => {
            let mut pb = PathBuilder::new();
            pb.move_to(0.6.logical_px(), (h - 0.4).logical_px())
                .line_to((w - 0.6).logical_px(), 0.4.logical_px());
            p.stroke_path(&pb, 1.1.logical_px(), fg);
        }
        SepShape::ArrowLThin => {
            let mut pb = PathBuilder::new();
            pb.move_to((w - 0.6).logical_px(), 0.6.logical_px())
                .line_to(1.2.logical_px(), mid_y.logical_px())
                .line_to((w - 0.6).logical_px(), (h - 0.6).logical_px());
            p.stroke_path(&pb, 1.15.logical_px(), fg);
        }
    }
}

fn fill_poly(p: &mut Painter<'_>, color: Color, pts: &[(f32, f32)]) {
    let Some((first, rest)) = pts.split_first() else {
        return;
    };
    let mut pb = PathBuilder::new();
    pb.move_to(first.0.logical_px(), first.1.logical_px());
    for &(x, y) in rest {
        pb.line_to(x.logical_px(), y.logical_px());
    }
    pb.close();
    p.fill_path(&pb, color);
}
