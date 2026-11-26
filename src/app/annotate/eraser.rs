//! Fast AABB-culled vector segment-splitting eraser.
//!
//! Instead of requiring `BlendMode::Clear` pixel erasure (or forcing whole-stroke deletion),
//! this module splits any stroke intersecting the eraser capsule `(ax, ay) -> (bx, by)` into
//! clean surviving sub-strokes with interpolated boundary cuts, keeping full Undo/Redo support
//! and minimal vector memory.

use std::sync::Arc;

use super::{AnnotationBoard, Stroke, StrokeBounds, StrokePoint};

#[derive(Debug, Clone, Copy)]
struct EraserCapsule {
    ax: f32,
    ay: f32,
    bx: f32,
    by: f32,
    radius: f32,
}

impl EraserCapsule {
    #[must_use]
    fn dist_sq_to(&self, x: f32, y: f32) -> f32 {
        point_to_segment_dist_sq(x, y, self.ax, self.ay, self.bx, self.by)
    }
}

/// Applies a capsule eraser from `(ax, ay)` to `(bx, by)` with `eraser_radius` across `board`.
/// Returns `true` if any stroke was split or removed.
pub fn erase_segment_on_board(
    board: &mut AnnotationBoard,
    ax: f32,
    ay: f32,
    bx: f32,
    by: f32,
    eraser_radius: f32,
) -> bool {
    if board.strokes.is_empty() {
        return false;
    }

    let cap = EraserCapsule {
        ax,
        ay,
        bx,
        by,
        radius: eraser_radius,
    };
    let mut any_changed = false;
    let mut next_strokes: Vec<Arc<Stroke>> = Vec::with_capacity(board.strokes.len());

    for stroke in &board.strokes {
        if !stroke.bounds.intersects_capsule(ax, ay, bx, by, eraser_radius) {
            next_strokes.push(Arc::clone(stroke));
            continue;
        }

        if let Some(pieces) = split_stroke_outside_capsule(stroke, cap) {
            any_changed = true;
            for mut piece in pieces {
                piece.points.shrink_to_fit();
                next_strokes.push(Arc::new(piece));
            }
        } else {
            next_strokes.push(Arc::clone(stroke));
        }
    }

    if any_changed {
        next_strokes.shrink_to_fit();
        board.strokes = next_strokes;
    }
    any_changed
}

/// Returns `Some(surviving_pieces)` if `stroke` was touched by the eraser capsule, or `None` if untouched.
fn split_stroke_outside_capsule(stroke: &Stroke, cap: EraserCapsule) -> Option<Vec<Stroke>> {
    let pts = &stroke.points;
    if pts.is_empty() {
        return Some(Vec::new());
    }

    // Single-point dot stroke.
    if pts.len() == 1 {
        let p = pts[0];
        let hit_r = cap.radius + stroke.radius_at(p.pressure) * 0.7;
        if cap.dist_sq_to(p.x, p.y) <= hit_r * hit_r {
            return Some(Vec::new());
        }
        return None;
    }

    let mut touched = false;
    let mut pieces: Vec<Vec<StrokePoint>> = Vec::new();
    let mut current: Vec<StrokePoint> = Vec::new();

    for i in 0..pts.len() {
        let p = pts[i];
        let hit_r = cap.radius + stroke.radius_at(p.pressure) * 0.65;
        let p_inside = cap.dist_sq_to(p.x, p.y) <= hit_r * hit_r;

        if i == 0 {
            if p_inside {
                touched = true;
            } else {
                current.push(p);
            }
            continue;
        }

        let prev = pts[i - 1];
        let prev_r = cap.radius + stroke.radius_at(prev.pressure) * 0.65;
        let prev_inside = cap.dist_sq_to(prev.x, prev.y) <= prev_r * prev_r;

        match (prev_inside, p_inside) {
            (false, false) => {
                // Both endpoints outside, but check if the stroke segment passes through the eraser!
                let mid_r = (hit_r + prev_r) * 0.5;
                if let Some((t_enter, t_exit)) = segment_capsule_clip(prev, p, cap, mid_r) {
                    touched = true;
                    if t_enter > 0.02 {
                        current.push(lerp_point(prev, p, t_enter));
                    }
                    if current.len() >= 2 {
                        pieces.push(std::mem::take(&mut current));
                    } else {
                        current.clear();
                    }
                    if t_exit < 0.98 {
                        current.push(lerp_point(prev, p, t_exit));
                    }
                    current.push(p);
                } else {
                    current.push(p);
                }
            }
            (false, true) => {
                touched = true;
                // Entering eraser: clip at entry boundary and close current piece.
                let t = binary_search_boundary(prev, p, cap, hit_r, false);
                if t > 0.02 {
                    current.push(lerp_point(prev, p, t));
                }
                if current.len() >= 2 {
                    pieces.push(std::mem::take(&mut current));
                } else {
                    current.clear();
                }
            }
            (true, false) => {
                touched = true;
                // Exiting eraser: start a fresh piece at exit boundary.
                let t = binary_search_boundary(prev, p, cap, hit_r, true);
                if t < 0.98 {
                    current.push(lerp_point(prev, p, t));
                }
                current.push(p);
            }
            (true, true) => {
                touched = true;
            }
        }
    }

    if !touched {
        return None;
    }

    if current.len() >= 2 {
        pieces.push(current);
    }

    let max_half = stroke.base_width * 0.5 + 1.0;
    let out = pieces
        .into_iter()
        .map(|points| {
            let bounds = StrokeBounds::from_points(&points, max_half);
            Stroke {
                color: stroke.color,
                base_width: stroke.base_width,
                pressure_sensitive: stroke.pressure_sensitive,
                points,
                bounds,
            }
        })
        .collect();
    Some(out)
}

#[must_use]
fn lerp_point(a: StrokePoint, b: StrokePoint, t: f32) -> StrokePoint {
    StrokePoint {
        x: a.x + (b.x - a.x) * t,
        y: a.y + (b.y - a.y) * t,
        pressure: a.pressure + (b.pressure - a.pressure) * t,
    }
}

/// Squared Euclidean distance from point `(px, py)` to line segment `(ax, ay)-(bx, by)`.
#[must_use]
pub fn point_to_segment_dist_sq(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let abx = bx - ax;
    let aby = by - ay;
    let ab_len_sq = abx * abx + aby * aby;
    if ab_len_sq <= 1e-6 {
        let dx = px - ax;
        let dy = py - ay;
        return dx * dx + dy * dy;
    }
    let t = (((px - ax) * abx + (py - ay) * aby) / ab_len_sq).clamp(0.0, 1.0);
    let cx = ax + abx * t;
    let cy = ay + aby * t;
    let dx = px - cx;
    let dy = py - cy;
    dx * dx + dy * dy
}

/// Checks whether a stroke segment `p0->p1` (with both endpoints outside) passes through the
/// eraser capsule, returning `(t_enter, t_exit)` in `0.0..=1.0`.
#[must_use]
fn segment_capsule_clip(
    p0: StrokePoint,
    p1: StrokePoint,
    cap: EraserCapsule,
    r: f32,
) -> Option<(f32, f32)> {
    // Find closest parameter on p0->p1 to capsule endpoints & midpoint (handles long fast strokes).
    let r_sq = r * r;
    let mut best_t = None;
    let mut best_d_sq = f32::INFINITY;
    for &(qx, qy) in &[
        (cap.ax, cap.ay),
        (cap.bx, cap.by),
        ((cap.ax + cap.bx) * 0.5, (cap.ay + cap.by) * 0.5),
    ] {
        let t = project_t(qx, qy, p0.x, p0.y, p1.x, p1.y);
        let pt = lerp_point(p0, p1, t);
        let d_sq = cap.dist_sq_to(pt.x, pt.y);
        if d_sq < best_d_sq {
            best_d_sq = d_sq;
            best_t = Some(t);
        }
    }
    let t_mid = best_t?;
    if best_d_sq > r_sq {
        return None;
    }

    let mid_pt = lerp_point(p0, p1, t_mid);
    let t_enter = t_mid * binary_search_boundary(p0, mid_pt, cap, r, false);
    let t_exit = t_mid + (1.0 - t_mid) * binary_search_boundary(mid_pt, p1, cap, r, true);
    Some((t_enter, t_exit))
}

#[must_use]
fn project_t(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let abx = bx - ax;
    let aby = by - ay;
    let len_sq = abx * abx + aby * aby;
    if len_sq <= 1e-6 {
        return 0.5;
    }
    (((px - ax) * abx + (py - ay) * aby) / len_sq).clamp(0.0, 1.0)
}

/// 6-iteration binary search (`< 1.6%` error) finding where `a->b` crosses the eraser boundary.
#[must_use]
fn binary_search_boundary(
    a: StrokePoint,
    b: StrokePoint,
    cap: EraserCapsule,
    r: f32,
    a_inside: bool,
) -> f32 {
    let r_sq = r * r;
    let mut lo = 0.0_f32;
    let mut hi = 1.0_f32;
    for _ in 0..6 {
        let mid = (lo + hi) * 0.5;
        let pt = lerp_point(a, b, mid);
        let inside = cap.dist_sq_to(pt.x, pt.y) <= r_sq;
        if inside == a_inside {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo + hi) * 0.5
}
