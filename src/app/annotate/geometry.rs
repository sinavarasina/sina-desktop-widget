//! Smooth pressure-sensitive filled ribbon polygon tessellation for `waywi::Painter`.
//!
//! Renders each freehand stroke as a single closed vector polygon with rounded end-caps,
//! providing smooth calligraphic pressure variation and uniform alpha (zero double-blended
//! joint circles on translucent highlighters) across both software and GPU renderers.

use std::f32::consts::PI;

use waywi::prelude::*;

use super::Stroke;

const CAP_SEGMENTS: usize = 6;

/// Draws a single vector stroke onto `painter` using a closed variable-width ribbon polygon.
pub fn draw_stroke(painter: &mut Painter<'_>, stroke: &Stroke) {
    let pts = &stroke.points;
    if pts.is_empty() {
        return;
    }

    if pts.len() == 1 {
        let pt = pts[0];
        let r = stroke.radius_at(pt.pressure);
        painter.fill_circle(
            pt.x.logical_px(),
            pt.y.logical_px(),
            r.logical_px(),
            stroke.color,
        );
        return;
    }

    let n = pts.len();
    let mut left_edge: Vec<(f32, f32)> = Vec::with_capacity(n);
    let mut right_edge: Vec<(f32, f32)> = Vec::with_capacity(n);
    let mut prev_dir = (1.0_f32, 0.0_f32);

    for i in 0..n {
        let cur = pts[i];
        let (dx, dy) = if i == 0 {
            (pts[1].x - cur.x, pts[1].y - cur.y)
        } else if i + 1 == n {
            (cur.x - pts[i - 1].x, cur.y - pts[i - 1].y)
        } else {
            // Central difference gives smooth tangent transitions across corners.
            let ax = cur.x - pts[i - 1].x;
            let ay = cur.y - pts[i - 1].y;
            let al = (ax * ax + ay * ay).sqrt();
            let bx = pts[i + 1].x - cur.x;
            let by = pts[i + 1].y - cur.y;
            let bl = (bx * bx + by * by).sqrt();
            if al > 1e-4 && bl > 1e-4 {
                let ux = ax / al;
                let uy = ay / al;
                let vx = bx / bl;
                let vy = by / bl;
                let sx = ux + vx;
                let sy = uy + vy;
                // Avoid degenerate zero vector on exact 180-degree hairpins.
                if sx * sx + sy * sy < 1e-3 {
                    (vx, vy)
                } else {
                    (sx, sy)
                }
            } else if bl > 1e-4 {
                (bx, by)
            } else {
                (ax, ay)
            }
        };

        let len = (dx * dx + dy * dy).sqrt();
        let (tx, ty) = if len > 1e-4 {
            let d = (dx / len, dy / len);
            prev_dir = d;
            d
        } else {
            prev_dir
        };

        // Left-hand unit normal `(-ty, tx)`.
        let nx = -ty;
        let ny = tx;
        let r = stroke.radius_at(cur.pressure);

        left_edge.push((cur.x + nx * r, cur.y + ny * r));
        right_edge.push((cur.x - nx * r, cur.y - ny * r));
    }

    let mut pb = PathBuilder::new();
    pb.move_to(left_edge[0].0.logical_px(), left_edge[0].1.logical_px());

    for &(lx, ly) in left_edge.iter().skip(1) {
        pb.line_to(lx.logical_px(), ly.logical_px());
    }

    // Rounded end-cap around the final point from left edge to right edge.
    let end_pt = pts[n - 1];
    let end_r = stroke.radius_at(end_pt.pressure);
    let end_start_angle = (left_edge[n - 1].1 - end_pt.y).atan2(left_edge[n - 1].0 - end_pt.x);
    for step in 1..CAP_SEGMENTS {
        let frac = (step as f32) / (CAP_SEGMENTS as f32);
        let angle = end_start_angle - PI * frac;
        let cx = end_pt.x + angle.cos() * end_r;
        let cy = end_pt.y + angle.sin() * end_r;
        pb.line_to(cx.logical_px(), cy.logical_px());
    }

    for &(rx, ry) in right_edge.iter().rev() {
        pb.line_to(rx.logical_px(), ry.logical_px());
    }

    // Rounded start-cap around the initial point from right edge back to left edge.
    let start_pt = pts[0];
    let start_r = stroke.radius_at(start_pt.pressure);
    let start_angle = (right_edge[0].1 - start_pt.y).atan2(right_edge[0].0 - start_pt.x);
    for step in 1..CAP_SEGMENTS {
        let frac = (step as f32) / (CAP_SEGMENTS as f32);
        let angle = start_angle - PI * frac;
        let cx = start_pt.x + angle.cos() * start_r;
        let cy = start_pt.y + angle.sin() * start_r;
        pb.line_to(cx.logical_px(), cy.logical_px());
    }

    pb.close();
    painter.fill_path(&pb, stroke.color);
}

/// Draws a crisp thin preview ring indicating the active vector eraser radius.
pub fn draw_eraser_cursor(painter: &mut Painter<'_>, x: f32, y: f32, radius: f32, color: Color) {
    const RING_STEPS: usize = 24;
    let mut pb = PathBuilder::new();
    for i in 0..=RING_STEPS {
        let angle = (i as f32 / RING_STEPS as f32) * 2.0 * PI;
        let px = x + angle.cos() * radius;
        let py = y + angle.sin() * radius;
        if i == 0 {
            pb.move_to(px.logical_px(), py.logical_px());
        } else {
            pb.line_to(px.logical_px(), py.logical_px());
        }
    }
    pb.close();
    painter.stroke_path(&pb, 1.25.logical_px(), color);
}
