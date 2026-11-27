//! Minimalist per-workspace screen annotation model with pressure-sensitive vector
//! strokes, segment-splitting eraser, bounded Undo/Redo (`Ctrl+Z` / `Ctrl+Y`), and
//! aggressive heap reclamation on clear.

pub mod eraser;
pub mod geometry;

use std::collections::HashMap;
use std::sync::Arc;

use waywi::prelude::*;

use crate::app::anim::RevealAnim;
use crate::theme::{BLUE, GREEN, MAUVE, PEACH, RED, TEXT};

/// Maximum Undo entries retained per workspace to bound long-session RAM usage.
pub const MAX_UNDO_STEPS: usize = 48;
/// Minimum Euclidean distance (logical px) between consecutive stroke samples.
const MIN_SAMPLE_DIST_SQ: f32 = 2.25; // 1.5px^2
/// Minimum pressure delta that forces recording a sample even at small movement.
const MIN_PRESSURE_DELTA: f32 = 0.06;
/// Exponential smoothing factor for stylus pressure (`0.0..=1.0`; higher = snappier).
const PRESSURE_SMOOTH_ALPHA: f32 = 0.45;
/// Highlighter opacity (uniform across the single closed ribbon polygon).
pub const HIGHLIGHTER_ALPHA: f32 = 0.34;
/// Highlighter width multiplier relative to the base pen size.
pub const HIGHLIGHTER_WIDTH_MULT: f32 = 3.2;
/// Eraser radius multiplier relative to the base pen size.
pub const ERASER_RADIUS_MULT: f32 = 2.6;

/// Curated Catppuccin Mocha ink palette (keys `1`..=`6`).
pub const ANNOTATE_PALETTE: [(Color, &str); 6] = [
    (MAUVE, "Mauve"),
    (RED, "Red"),
    (PEACH, "Peach"),
    (GREEN, "Green"),
    (BLUE, "Blue"),
    (TEXT, "Chalk"),
];

/// Quick brush size presets (logical px).
pub const SIZE_PRESETS: [f32; 3] = [3.0, 6.0, 11.0];
pub const MIN_BRUSH_SIZE: f32 = 2.0;
pub const MAX_BRUSH_SIZE: f32 = 24.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AnnotateTool {
    #[default]
    Pen,
    Highlighter,
    Eraser,
}

/// Identifies which Hyprland workspace owns an annotation board.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WorkspaceKey {
    Numbered(i32),
    Named(String),
    Special(String),
}

impl WorkspaceKey {
    /// Short badge text displayed on the floating annotation toolbar (`WS 1`, `SP:magic`, etc.).
    #[must_use]
    pub fn short_label(&self) -> String {
        match self {
            Self::Numbered(id) => format!("WS {id}"),
            Self::Named(name) => {
                let clean = name.strip_prefix("name:").unwrap_or(name).trim();
                let short: String = clean.chars().take(8).collect();
                format!("WS {short}")
            }
            Self::Special(name) => {
                let clean = name.strip_prefix("special:").unwrap_or(name).trim();
                if clean.is_empty() {
                    String::from("SP")
                } else {
                    let short: String = clean.chars().take(7).collect();
                    format!("SP:{short}")
                }
            }
        }
    }
}

/// Compact 12-byte stroke control point in surface-local logical coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrokePoint {
    pub x: f32,
    pub y: f32,
    /// Normalized tip pressure in `0.05..=1.0` (`1.0` for non-pressure mouse/touch).
    pub pressure: f32,
}

/// Axis-aligned bounding box inflated by maximum stroke half-width for O(1) eraser culling.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrokeBounds {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

impl StrokeBounds {
    #[must_use]
    pub fn from_points(points: &[StrokePoint], max_half_width: f32) -> Self {
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for pt in points {
            min_x = min_x.min(pt.x);
            min_y = min_y.min(pt.y);
            max_x = max_x.max(pt.x);
            max_y = max_y.max(pt.y);
        }
        if !min_x.is_finite() {
            return Self {
                min_x: 0.0,
                min_y: 0.0,
                max_x: 0.0,
                max_y: 0.0,
            };
        }
        Self {
            min_x: min_x - max_half_width,
            min_y: min_y - max_half_width,
            max_x: max_x + max_half_width,
            max_y: max_y + max_half_width,
        }
    }

    pub fn expand_point(&mut self, x: f32, y: f32, pad: f32) {
        self.min_x = self.min_x.min(x - pad);
        self.min_y = self.min_y.min(y - pad);
        self.max_x = self.max_x.max(x + pad);
        self.max_y = self.max_y.max(y + pad);
    }

    #[must_use]
    pub fn intersects_capsule(&self, ax: f32, ay: f32, bx: f32, by: f32, radius: f32) -> bool {
        let seg_min_x = ax.min(bx) - radius;
        let seg_max_x = ax.max(bx) + radius;
        let seg_min_y = ay.min(by) - radius;
        let seg_max_y = ay.max(by) + radius;
        !(self.max_x < seg_min_x
            || self.min_x > seg_max_x
            || self.max_y < seg_min_y
            || self.min_y > seg_max_y)
    }
}

/// Immutable (once committed) vector stroke shared cheaply with `Canvas` draw closures via `Arc`.
#[derive(Debug, Clone, PartialEq)]
pub struct Stroke {
    pub color: Color,
    pub base_width: f32,
    pub pressure_sensitive: bool,
    pub points: Vec<StrokePoint>,
    pub bounds: StrokeBounds,
}

impl Stroke {
    #[must_use]
    pub fn new(color: Color, base_width: f32, pressure_sensitive: bool, start: StrokePoint) -> Self {
        let pad = base_width * 0.5 + 1.0;
        let bounds = StrokeBounds {
            min_x: start.x - pad,
            min_y: start.y - pad,
            max_x: start.x + pad,
            max_y: start.y + pad,
        };
        Self {
            color,
            base_width,
            pressure_sensitive,
            points: vec![start],
            bounds,
        }
    }

    /// Computes effective half-width at `pressure` (`0.05..=1.0`).
    #[must_use]
    pub fn radius_at(&self, pressure: f32) -> f32 {
        let half = (self.base_width * 0.5).max(0.5);
        if self.pressure_sensitive {
            let p = pressure.clamp(0.05, 1.0);
            half * (0.18 + 0.82 * p.powf(0.85))
        } else {
            half
        }
    }

    /// Appends a sample if it moves enough or shifts pressure meaningfully.
    pub fn push_sample(&mut self, x: f32, y: f32, raw_pressure: f32) -> bool {
        let target_p = if self.pressure_sensitive {
            raw_pressure.clamp(0.05, 1.0)
        } else {
            1.0
        };
        let (last_x, last_y, smoothed_p) = if let Some(last) = self.points.last() {
            let sp = if self.pressure_sensitive {
                last.pressure + (target_p - last.pressure) * PRESSURE_SMOOTH_ALPHA
            } else {
                1.0
            };
            (last.x, last.y, sp)
        } else {
            let pt = StrokePoint {
                x,
                y,
                pressure: target_p,
            };
            self.points.push(pt);
            self.bounds.expand_point(x, y, self.base_width * 0.5 + 1.0);
            return true;
        };

        let dx = x - last_x;
        let dy = y - last_y;
        let dist_sq = dx * dx + dy * dy;
        let dp = (smoothed_p - self.points.last().map_or(1.0, |p| p.pressure)).abs();
        if dist_sq < MIN_SAMPLE_DIST_SQ && dp < MIN_PRESSURE_DELTA {
            return false;
        }
        self.points.push(StrokePoint {
            x,
            y,
            pressure: smoothed_p,
        });
        self.bounds.expand_point(x, y, self.base_width * 0.5 + 1.0);
        true
    }
}

/// Compact reversible delta recorded on each committed draw or eraser drag.
#[derive(Debug, Clone)]
enum UndoEntry {
    /// Appended one stroke at the end of `strokes`.
    AddStroke(Arc<Stroke>),
    /// Eraser drag replaced `before` stroke list with `after` (stored as cheap `Arc` slices).
    EraseBatch {
        before: Vec<Arc<Stroke>>,
        after: Vec<Arc<Stroke>>,
    },
}

/// Per-workspace annotation board with bounded undo/redo history and `Canvas` cache revision.
#[derive(Debug, Default, Clone)]
pub struct AnnotationBoard {
    pub strokes: Vec<Arc<Stroke>>,
    undo_stack: Vec<UndoEntry>,
    redo_stack: Vec<UndoEntry>,
    /// Monotonic revision bumped whenever `strokes` changes; keys the committed `Canvas` cache.
    pub revision: u64,
}

impl AnnotationBoard {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.strokes.is_empty()
    }

    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    fn push_undo(&mut self, entry: UndoEntry) {
        self.redo_stack.clear();
        self.redo_stack.shrink_to_fit();
        if self.undo_stack.len() >= MAX_UNDO_STEPS {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(entry);
    }

    pub fn commit_stroke(&mut self, mut stroke: Stroke) {
        stroke.points.shrink_to_fit();
        let arc = Arc::new(stroke);
        self.strokes.push(Arc::clone(&arc));
        self.push_undo(UndoEntry::AddStroke(arc));
        self.revision = self.revision.wrapping_add(1);
    }

    pub fn record_erase_batch(&mut self, before: Vec<Arc<Stroke>>) {
        let after = self.strokes.clone();
        self.push_undo(UndoEntry::EraseBatch { before, after });
    }

    pub fn undo(&mut self) -> bool {
        let Some(entry) = self.undo_stack.pop() else {
            return false;
        };
        match entry {
            UndoEntry::AddStroke(arc) => {
                self.strokes.pop();
                self.redo_stack.push(UndoEntry::AddStroke(arc));
            }
            UndoEntry::EraseBatch { before, after } => {
                self.strokes = before.clone();
                self.redo_stack.push(UndoEntry::EraseBatch { before, after });
            }
        }
        self.revision = self.revision.wrapping_add(1);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(entry) = self.redo_stack.pop() else {
            return false;
        };
        match entry {
            UndoEntry::AddStroke(arc) => {
                self.strokes.push(Arc::clone(&arc));
                self.undo_stack.push(UndoEntry::AddStroke(arc));
            }
            UndoEntry::EraseBatch { before, after } => {
                self.strokes = after.clone();
                self.undo_stack.push(UndoEntry::EraseBatch { before, after });
            }
        }
        self.revision = self.revision.wrapping_add(1);
        true
    }

    /// Drops every stroke and history buffer and frees heap capacity immediately.
    pub fn clear_and_free(&mut self) {
        self.strokes.clear();
        self.strokes.shrink_to_fit();
        self.undo_stack.clear();
        self.undo_stack.shrink_to_fit();
        self.redo_stack.clear();
        self.redo_stack.shrink_to_fit();
        self.revision = self.revision.wrapping_add(1);
    }
}

/// Active input source currently owning an in-progress draw or erase drag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveInputSource {
    Pointer,
    Tablet(TabletToolId),
    Touch(i32),
}

/// Parameters for initiating a draw or erase gesture from mouse, pen tablet, or touch.
#[derive(Debug, Clone, Copy)]
pub struct StrokeInputStart {
    pub source: ActiveInputSource,
    pub x: f32,
    pub y: f32,
    pub pressure: f32,
    pub pressure_sensitive: bool,
    pub force_eraser: bool,
}

/// In-progress eraser drag session (grouped into a single Undo entry on lift).
#[derive(Debug, Clone)]
pub struct ActiveEraserDrag {
    pub source: ActiveInputSource,
    pub last_x: f32,
    pub last_y: f32,
    pub snapshot_before: Vec<Arc<Stroke>>,
    pub modified: bool,
}

/// Application state for the minimalist per-workspace screen annotation overlay.
pub struct AnnotateState {
    pub anim: RevealAnim,
    /// When `true`, exiting draw mode keeps the active workspace's drawings visible in
    /// `InputMode::PassThrough` mode so clicks pass straight through to windows below.
    pub pinned_visible: bool,
    pub tool: AnnotateTool,
    pub color_idx: usize,
    pub brush_size: f32,
    /// Per-workspace boards; empty boards are evicted so unused workspaces hold 0 heap bytes.
    pub boards: HashMap<WorkspaceKey, AnnotationBoard>,
    /// Active freehand stroke currently being drawn (rendered on the fast top canvas layer).
    pub active_stroke: Option<(ActiveInputSource, Stroke)>,
    /// Active vector eraser drag session.
    pub active_eraser: Option<ActiveEraserDrag>,
    /// Temporary eraser override via stylus barrel button (`BTN_STYLUS`).
    pub stylus_barrel_eraser: bool,
    /// Last known cursor/stylus position on the canvas (for crisp eraser radius preview).
    pub cursor_pos: Option<(f32, f32)>,
    /// Monotonic counter bumped on active stroke / cursor changes for lightweight redraw.
    pub live_seq: u64,
    /// Monotonic generation bumped on `clear_workspace` / `clear_all` so `Canvas` cache keys
    /// invalidate even when a workspace board is removed from the map.
    pub clear_epoch: u64,
}

impl Default for AnnotateState {
    fn default() -> Self {
        Self {
            anim: RevealAnim::popup(),
            pinned_visible: false,
            tool: AnnotateTool::Pen,
            color_idx: 0,
            brush_size: 4.0,
            boards: HashMap::new(),
            active_stroke: None,
            active_eraser: None,
            stylus_barrel_eraser: false,
            cursor_pos: None,
            live_seq: 0,
            clear_epoch: 0,
        }
    }
}

impl AnnotateState {
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.anim.is_open()
    }

    #[must_use]
    pub fn is_animating(&self) -> bool {
        self.anim.is_animating()
    }

    #[must_use]
    pub fn progress(&self) -> f32 {
        self.anim.progress()
    }

    /// Returns true if the given workspace has at least one committed stroke.
    #[must_use]
    pub fn has_strokes(&self, ws: &WorkspaceKey) -> bool {
        self.boards.get(ws).is_some_and(|b| !b.is_empty())
    }

    /// Whether the overlay surface should be mounted (either interactive mode, animating out,
    /// or pinned pass-through with at least one stroke on the current workspace).
    #[must_use]
    pub fn should_mount_surface(&self, ws: &WorkspaceKey) -> bool {
        self.anim.is_visible() || (self.pinned_visible && self.has_strokes(ws))
    }

    pub fn open(&mut self) {
        self.anim.set_open(true);
    }

    pub fn close(&mut self, ws: &WorkspaceKey) {
        self.finish_active_interaction(ws);
        self.cursor_pos = None;
        self.stylus_barrel_eraser = false;
        self.anim.set_open(false);
    }

    pub fn advance_animation(&mut self, now: std::time::Instant) {
        self.anim.advance(now);
    }

    #[must_use]
    pub fn active_color(&self) -> Color {
        ANNOTATE_PALETTE[self.color_idx % ANNOTATE_PALETTE.len()].0
    }

    #[must_use]
    pub fn effective_stroke_color(&self) -> Color {
        let base = self.active_color();
        match self.tool {
            AnnotateTool::Highlighter => base.with_opacity(HIGHLIGHTER_ALPHA),
            AnnotateTool::Pen | AnnotateTool::Eraser => base,
        }
    }

    #[must_use]
    pub fn effective_stroke_width(&self) -> f32 {
        match self.tool {
            AnnotateTool::Highlighter => (self.brush_size * HIGHLIGHTER_WIDTH_MULT).clamp(8.0, 44.0),
            AnnotateTool::Pen | AnnotateTool::Eraser => self.brush_size,
        }
    }

    #[must_use]
    pub fn eraser_radius(&self) -> f32 {
        (self.brush_size * ERASER_RADIUS_MULT).clamp(8.0, 48.0)
    }

    pub fn step_brush_size(&mut self, delta: f32) {
        self.brush_size = (self.brush_size + delta).clamp(MIN_BRUSH_SIZE, MAX_BRUSH_SIZE);
        self.bump_live();
    }

    pub fn set_brush_size(&mut self, size: f32) {
        self.brush_size = size.clamp(MIN_BRUSH_SIZE, MAX_BRUSH_SIZE);
        self.bump_live();
    }

    #[must_use]
    pub fn board(&self, ws: &WorkspaceKey) -> Option<&AnnotationBoard> {
        self.boards.get(ws)
    }

    #[must_use]
    pub fn total_stroke_count(&self) -> usize {
        self.boards.values().map(|b| b.strokes.len()).sum()
    }

    #[must_use]
    pub fn has_any_board_data(&self) -> bool {
        self.boards
            .values()
            .any(|b| !b.is_empty() || b.can_undo() || b.can_redo())
            || self.active_stroke.is_some()
    }

    fn bump_live(&mut self) {
        self.live_seq = self.live_seq.wrapping_add(1);
    }

    /// Starts a drawing or erasing gesture from mouse, pen tablet, or touch.
    pub fn begin_input(&mut self, ws: &WorkspaceKey, input: StrokeInputStart) {
        // Commit any lingering interaction from another source cleanly.
        self.finish_active_interaction(ws);
        self.cursor_pos = Some((input.x, input.y));

        let use_eraser = input.force_eraser
            || self.tool == AnnotateTool::Eraser
            || self.stylus_barrel_eraser;
        if use_eraser {
            let snapshot_before = self
                .boards
                .get(ws)
                .map(|b| b.strokes.clone())
                .unwrap_or_default();
            let mut drag = ActiveEraserDrag {
                source: input.source,
                last_x: input.x,
                last_y: input.y,
                snapshot_before,
                modified: false,
            };
            if self.apply_eraser_segment(ws, input.x, input.y, input.x, input.y) {
                drag.modified = true;
            }
            self.active_eraser = Some(drag);
            self.bump_live();
            return;
        }

        let start_p = if input.pressure_sensitive {
            input.pressure.clamp(0.05, 1.0)
        } else {
            1.0
        };
        let stroke = Stroke::new(
            self.effective_stroke_color(),
            self.effective_stroke_width(),
            input.pressure_sensitive,
            StrokePoint {
                x: input.x,
                y: input.y,
                pressure: start_p,
            },
        );
        self.active_stroke = Some((input.source, stroke));
        self.bump_live();
    }

    /// Updates an active draw or erase drag, or tracks hover position for the eraser cursor ring.
    pub fn move_input(
        &mut self,
        ws: &WorkspaceKey,
        source: ActiveInputSource,
        x: f32,
        y: f32,
        pressure: f32,
    ) {
        let should_track_cursor = self.tool == AnnotateTool::Eraser
            || self.stylus_barrel_eraser
            || self.active_eraser.is_some();
        if should_track_cursor {
            self.cursor_pos = Some((x, y));
            self.bump_live();
        } else if self.active_stroke.is_some() {
            self.cursor_pos = Some((x, y));
        }

        if let Some(ref mut drag) = self.active_eraser {
            if drag.source != source {
                return;
            }
            let (ax, ay) = (drag.last_x, drag.last_y);
            drag.last_x = x;
            drag.last_y = y;
            if self.apply_eraser_segment(ws, ax, ay, x, y)
                && let Some(ref mut d) = self.active_eraser
            {
                d.modified = true;
            }
            self.bump_live();
            return;
        }

        if let Some((active_src, ref mut stroke)) = self.active_stroke {
            if active_src != source {
                return;
            }
            if stroke.push_sample(x, y, pressure) {
                self.bump_live();
            }
        }
    }

    /// Finishes the active stroke or eraser drag for `source` (or any source if `None`).
    pub fn end_input(&mut self, ws: &WorkspaceKey, source: Option<ActiveInputSource>) {
        if let Some(drag) = self.active_eraser.take() {
            if source.is_some_and(|s| s != drag.source) {
                self.active_eraser = Some(drag);
                return;
            }
            if drag.modified {
                let board = self.boards.entry(ws.clone()).or_default();
                board.record_erase_batch(drag.snapshot_before);
                self.prune_if_completely_empty(ws);
            }
            self.bump_live();
            return;
        }

        if let Some((active_src, stroke)) = self.active_stroke.take() {
            if source.is_some_and(|s| s != active_src) {
                self.active_stroke = Some((active_src, stroke));
                return;
            }
            let board = self.boards.entry(ws.clone()).or_default();
            board.commit_stroke(stroke);
            self.bump_live();
        }
    }

    pub fn finish_active_interaction(&mut self, ws: &WorkspaceKey) {
        self.end_input(ws, None);
    }

    fn apply_eraser_segment(
        &mut self,
        ws: &WorkspaceKey,
        ax: f32,
        ay: f32,
        bx: f32,
        by: f32,
    ) -> bool {
        let radius = self.eraser_radius();
        let Some(board) = self.boards.get_mut(ws) else {
            return false;
        };
        if eraser::erase_segment_on_board(board, ax, ay, bx, by, radius) {
            board.revision = board.revision.wrapping_add(1);
            true
        } else {
            false
        }
    }

    pub fn undo(&mut self, ws: &WorkspaceKey) -> bool {
        self.finish_active_interaction(ws);
        let Some(board) = self.boards.get_mut(ws) else {
            return false;
        };
        let changed = board.undo();
        if changed {
            self.prune_if_completely_empty(ws);
            self.bump_live();
        }
        changed
    }

    pub fn redo(&mut self, ws: &WorkspaceKey) -> bool {
        self.finish_active_interaction(ws);
        let Some(board) = self.boards.get_mut(ws) else {
            return false;
        };
        let changed = board.redo();
        if changed {
            self.bump_live();
        }
        changed
    }

    /// Clears the current workspace board and frees all its allocated memory immediately.
    pub fn clear_workspace(&mut self, ws: &WorkspaceKey) -> bool {
        let had_active = self.active_stroke.take().is_some();
        self.active_eraser = None;
        let had_board = if let Some(mut board) = self.boards.remove(ws) {
            let had = !board.is_empty() || board.can_undo() || board.can_redo();
            board.clear_and_free();
            drop(board);
            self.boards.shrink_to_fit();
            had
        } else {
            false
        };
        if had_active || had_board {
            self.clear_epoch = self.clear_epoch.wrapping_add(1);
            self.bump_live();
            true
        } else {
            false
        }
    }

    /// Clears EVERY workspace board and frees all heap allocations immediately.
    pub fn clear_all(&mut self) -> bool {
        let had = self.has_any_board_data();
        self.active_stroke = None;
        self.active_eraser = None;
        for board in self.boards.values_mut() {
            board.clear_and_free();
        }
        self.boards.clear();
        self.boards.shrink_to_fit();
        if had {
            self.clear_epoch = self.clear_epoch.wrapping_add(1);
            self.bump_live();
            true
        } else {
            false
        }
    }

    fn prune_if_completely_empty(&mut self, ws: &WorkspaceKey) {
        let should_remove = self
            .boards
            .get(ws)
            .is_some_and(|b| b.is_empty() && !b.can_undo() && !b.can_redo());
        if should_remove {
            self.boards.remove(ws);
            self.boards.shrink_to_fit();
        }
    }
}
