//! Minimalist full-screen per-workspace annotation overlay (`Layer::Overlay`).
//!
//! Uses a two-layer `Canvas` architecture for optimal CPU & GPU performance:
//! - Layer 1 (committed strokes): cached via `Canvas::cache_key` per `(workspace, revision, clear_epoch)`
//!   so existing drawings are never re-tessellated while drawing new strokes.
//! - Layer 2 (live stroke & eraser ring): only mounted while actively drawing or aiming the eraser.

use std::sync::Arc;

use waywi::prelude::*;

use crate::app::annotate::geometry::{draw_eraser_cursor, draw_stroke};
use crate::app::annotate::{ANNOTATE_PALETTE, AnnotateTool, SIZE_PRESETS, Stroke};
use crate::app::state::{HoverTarget, Msg, State};
use crate::theme::{
    BLUE, CRUST, MANTLE, MAUVE, OVERLAY0, PEACH, POPUP_TOP_MARGIN, RED, SUBTEXT1, SURFACE0,
    SURFACE1, TEXT, font_ui, font_ui_semibold,
};

const TOOLBAR_HEIGHT: f32 = 34.0;
const BTN_SIZE: f32 = 24.0;
const SWATCH_SIZE: f32 = 16.0;

pub fn build_annotate_surface(state: &State) -> Surface<Msg> {
    let ws = state.active_workspace_key();
    let active = state.annotate.is_active();
    let ease = state.annotate.progress();
    let w = state.bar_width.max(320.0);
    let h = state.screen_height.max(240.0);

    // Layer 1: Cached committed strokes for the active workspace.
    let (committed_strokes, revision): (Vec<Arc<Stroke>>, u64) = state
        .annotate
        .board(&ws)
        .map(|b| (b.strokes.clone(), b.revision))
        .unwrap_or_default();

    let committed_canvas = Canvas::new(Size::new(w.logical_px(), h.logical_px()), move |painter| {
        for stroke in &committed_strokes {
            draw_stroke(painter, stroke);
        }
    })
    .cache_key((
        "annotate-committed",
        ws.clone(),
        revision,
        state.annotate.clear_epoch,
    ))
    .key("annotate-committed-canvas")
    .input_enabled(false);

    // Layer 2: Uncached live active stroke & eraser cursor ring (mounted only when needed).
    let live_stroke = state.annotate.active_stroke.as_ref().map(|(_, s)| s.clone());
    let eraser_ring = if active
        && (state.annotate.tool == AnnotateTool::Eraser
            || state.annotate.stylus_barrel_eraser
            || state.annotate.active_eraser.is_some())
    {
        state.annotate.cursor_pos.map(|(x, y)| {
            let color = if state.annotate.active_eraser.is_some() {
                RED.with_opacity(0.9)
            } else {
                SUBTEXT1.with_opacity(0.75)
            };
            (x, y, state.annotate.eraser_radius(), color)
        })
    } else {
        None
    };

    let mut stage = Group::new()
        .key("annotate-stage")
        .size(Size::fill())
        .child(committed_canvas);

    if live_stroke.is_some() || eraser_ring.is_some() {
        let live_canvas = Canvas::new(Size::new(w.logical_px(), h.logical_px()), move |painter| {
            if let Some(ref stroke) = live_stroke {
                draw_stroke(painter, stroke);
            }
            if let Some((x, y, radius, color)) = eraser_ring {
                draw_eraser_cursor(painter, x, y, radius, color);
            }
        })
        .key("annotate-live-canvas")
        .input_enabled(false);
        stage = stage.child(live_canvas);
    }

    if active {
        stage = stage
            .cursor(CursorShape::Crosshair)
            .on_pointer_down(Msg::AnnotatePointerDown)
            .on_pointer_move(Msg::AnnotatePointerMove)
            .on_pointer_up(Msg::AnnotatePointerUp)
            .on_scroll(Msg::AnnotateScroll)
            .on_tablet(Msg::AnnotateTablet)
            .on_touch(Msg::AnnotateTouch);
    }

    let mut root = Group::new()
        .key("annotate-root")
        .size(Size::fill())
        .child(stage);

    if ease > 0.01 {
        root = root.child(build_floating_toolbar(state, &ws, ease));
    }

    let mut surface = Surface::floating("screen-annotate", Output::Primary)
        .layer(Layer::Overlay)
        .ignore_exclusive_zone()
        .anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT)
        .size(state.viewport_size())
        .background(Color::TRANSPARENT)
        .on_close(Msg::CloseAnnotate);

    if active {
        surface = surface
            .input_mode(InputMode::Full)
            .keyboard_interactivity(KeyboardInteractivity::Exclusive)
            .on_key(|ev| Some(Msg::AnnotateKey(ev)))
            .on_pointer_up(|ev| Some(Msg::AnnotatePointerUp(ev)));
    } else {
        surface = surface
            .input_mode(InputMode::PassThrough)
            .keyboard_interactivity(KeyboardInteractivity::None);
    }

    surface.child(root)
}

fn build_floating_toolbar(
    state: &State,
    ws: &crate::app::annotate::WorkspaceKey,
    ease: f32,
) -> Element<Msg> {
    let board = state.annotate.board(ws);
    let ws_strokes = board.map_or(0, |b| b.strokes.len());
    let can_undo = board.is_some_and(|b| b.can_undo());
    let can_redo = board.is_some_and(|b| b.can_redo());
    let has_ws_data = board.is_some_and(|b| !b.is_empty() || b.can_undo() || b.can_redo())
        || state.annotate.active_stroke.is_some();
    let has_any_data = state.annotate.has_any_board_data();

    let ws_label = if ws_strokes > 0 {
        format!("{} · {ws_strokes}", ws.short_label())
    } else {
        ws.short_label()
    };

    // 1. Workspace badge
    let ws_badge = Text::new(ws_label)
        .font(font_ui_semibold())
        .font_size(10.logical_px())
        .color(MAUVE);

    // 2. Tools: Pen, Highlighter, Eraser
    let tools_row = Row::new()
        .size(Size::auto())
        .gap(4.logical_px())
        .align(Align::Center)
        .child(tool_btn(
            state,
            "annotate-tool-pen",
            "\u{f040}",
            AnnotateTool::Pen,
            MAUVE,
        ))
        .child(tool_btn(
            state,
            "annotate-tool-hl",
            "\u{f1fc}",
            AnnotateTool::Highlighter,
            PEACH,
        ))
        .child(tool_btn(
            state,
            "annotate-tool-eraser",
            "\u{f12d}",
            AnnotateTool::Eraser,
            RED,
        ));

    // 3. Catppuccin Mocha color swatches
    let mut colors_row = Row::new()
        .size(Size::auto())
        .gap(4.logical_px())
        .align(Align::Center);
    for (idx, &(color, _)) in ANNOTATE_PALETTE.iter().enumerate() {
        colors_row = colors_row.child(color_swatch(state, idx, color));
    }

    // 4. Brush size presets + exact px readout
    let mut sizes_row = Row::new()
        .size(Size::auto())
        .gap(3.logical_px())
        .align(Align::Center);
    for (idx, &preset) in SIZE_PRESETS.iter().enumerate() {
        sizes_row = sizes_row.child(size_btn(state, idx, preset));
    }
    sizes_row = sizes_row.child(
        Text::new(format!("{}px", state.annotate.brush_size.round() as u16))
            .font(font_ui_semibold())
            .font_size(9.5.logical_px())
            .color(SUBTEXT1),
    );

    // 5. History, Clear, Pin, Exit
    let actions_row = Row::new()
        .size(Size::auto())
        .gap(4.logical_px())
        .align(Align::Center)
        .child(action_icon_btn(
            state,
            "annotate-undo",
            "\u{f0e2}",
            HoverTarget::AnnotateUndo,
            can_undo,
            false,
            MAUVE,
            Msg::AnnotateUndo,
        ))
        .child(action_icon_btn(
            state,
            "annotate-redo",
            "\u{f01e}",
            HoverTarget::AnnotateRedo,
            can_redo,
            false,
            MAUVE,
            Msg::AnnotateRedo,
        ))
        .child(divider())
        .child(action_icon_btn(
            state,
            "annotate-clear-ws",
            "\u{f1f8}",
            HoverTarget::AnnotateClearWs,
            has_ws_data,
            false,
            RED,
            Msg::AnnotateClearWorkspace,
        ))
        .child(clear_all_btn(state, has_any_data))
        .child(divider())
        .child(action_icon_btn(
            state,
            "annotate-pin",
            "\u{f08d}",
            HoverTarget::AnnotatePin,
            true,
            state.annotate.pinned_visible,
            BLUE,
            Msg::AnnotateTogglePin,
        ))
        .child(action_icon_btn(
            state,
            "annotate-close",
            "\u{f00d}",
            HoverTarget::AnnotateClose,
            true,
            false,
            RED,
            Msg::CloseAnnotate,
        ));

    let strip = Row::new()
        .size(Size::new(Length::Auto, TOOLBAR_HEIGHT.logical_px()))
        .padding(Edges::symmetric(0.logical_px(), 8.logical_px()))
        .gap(7.logical_px())
        .align(Align::Center)
        .child(ws_badge)
        .child(divider())
        .child(tools_row)
        .child(divider())
        .child(colors_row)
        .child(divider())
        .child(sizes_row)
        .child(divider())
        .child(actions_row);

    // Absorb pointer/tablet/touch/scroll on toolbar background so clicks never draw underneath.
    let bar_card = Group::new()
        .key("annotate-toolbar-card")
        .size(Size::auto())
        .on_pointer_down(|_| Msg::Noop)
        .on_tablet(|_| Msg::Noop)
        .on_touch(|_| Msg::Noop)
        .on_scroll(|_| Msg::Noop)
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(CRUST)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), SURFACE1),
        )
        .child(strip);

    let slide_y = POPUP_TOP_MARGIN + (1.0 - ease) * -12.0;
    let mut hud_col = Column::new()
        .key("annotate-hud")
        .size(Size::auto())
        .gap(4.logical_px())
        .align(Align::Center)
        .opacity(ease)
        .position(Position::RelativeToParent {
            x: Percent::new(50.0),
            y: Percent::new(0.0),
            offset_x: 0.logical_px(),
            offset_y: slide_y.logical_px(),
        })
        .anchor(ElementAnchor::TopCenter)
        .child(bar_card);

    if let Some(hint) = hover_hint_text(state) {
        let hint_chip = Group::new()
            .key("annotate-hint-chip")
            .size(Size::auto())
            .input_enabled(false)
            .child(
                Rect::new()
                    .size(Size::fill())
                    .fill(CRUST)
                    .corner_radius(0.logical_px())
                    .border(1.logical_px(), SURFACE0),
            )
            .child(
                Row::new()
                    .padding(Edges::symmetric(2.logical_px(), 7.logical_px()))
                    .align(Align::Center)
                    .child(
                        Text::new(hint)
                            .font(font_ui())
                            .font_size(9.5.logical_px())
                            .color(SUBTEXT1),
                    ),
            );
        hud_col = hud_col.child(hint_chip);
    }

    hud_col.into()
}

fn divider() -> Element<Msg> {
    Rect::new()
        .size(Size::new(1.logical_px(), 16.logical_px()))
        .fill(SURFACE0)
        .into()
}

fn bind_button_inputs(group: Group<Msg>, target: HoverTarget, msg: Msg) -> Group<Msg> {
    let tablet_msg = msg.clone();
    let touch_msg = msg.clone();
    group
        .cursor(CursorShape::Pointer)
        .on_hover(Msg::Hover(target))
        .on_leave(Msg::Leave(target))
        .on_click(msg)
        .on_tablet(move |ev| match ev {
            TabletEvent::Tool(t) if t.kind == TabletToolEventKind::Down => tablet_msg.clone(),
            _ => Msg::Noop,
        })
        .on_touch(move |ev| {
            if ev.phase == TouchPhase::Down {
                touch_msg.clone()
            } else {
                Msg::Noop
            }
        })
}

fn tool_btn(
    state: &State,
    key: &'static str,
    glyph: &'static str,
    tool: AnnotateTool,
    accent: Color,
) -> Element<Msg> {
    let target = HoverTarget::AnnotateTool(tool);
    let active = state.annotate.tool == tool;
    let hovered = state.hovered == target;
    let bg = if active {
        SURFACE1
    } else if hovered {
        SURFACE0
    } else {
        MANTLE
    };
    let border = if active || hovered { accent } else { SURFACE0 };
    let fg = if active || hovered { accent } else { SUBTEXT1 };

    let g = Group::new()
        .key(key)
        .size(Size::new(BTN_SIZE.logical_px(), BTN_SIZE.logical_px()))
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(bg)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), border),
        )
        .child(
            Text::new(glyph)
                .font(font_ui())
                .font_size(11.logical_px())
                .bounds_mode(TextBoundsMode::Optical)
                .color(fg)
                .position(Position::relative_to_parent(
                    Percent::new(50.0),
                    Percent::new(50.0),
                ))
                .anchor(ElementAnchor::Center),
        );

    bind_button_inputs(g, target, Msg::AnnotateSetTool(tool)).into()
}

fn color_swatch(state: &State, idx: usize, color: Color) -> Element<Msg> {
    let target = HoverTarget::AnnotateColor(idx);
    let active = state.annotate.color_idx == idx && state.annotate.tool != AnnotateTool::Eraser;
    let hovered = state.hovered == target;
    let frame_border = if active {
        TEXT
    } else if hovered {
        SUBTEXT1
    } else {
        SURFACE0
    };

    let g = Group::new()
        .key(("annotate-color", idx))
        .size(Size::new(
            (SWATCH_SIZE + 4.0).logical_px(),
            (SWATCH_SIZE + 4.0).logical_px(),
        ))
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(MANTLE)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), frame_border),
        )
        .child(
            Rect::new()
                .size(Size::new(
                    (SWATCH_SIZE - 2.0).logical_px(),
                    (SWATCH_SIZE - 2.0).logical_px(),
                ))
                .fill(color)
                .corner_radius(0.logical_px())
                .position(Position::relative_to_parent(
                    Percent::new(50.0),
                    Percent::new(50.0),
                ))
                .anchor(ElementAnchor::Center),
        );

    bind_button_inputs(g, target, Msg::AnnotateSetColor(idx)).into()
}

fn size_btn(state: &State, idx: usize, preset: f32) -> Element<Msg> {
    let target = HoverTarget::AnnotateSize(idx);
    let active = (state.annotate.brush_size - preset).abs() < 0.75;
    let hovered = state.hovered == target;
    let dot_px = match idx {
        0 => 4.0_f32,
        1 => 7.0_f32,
        _ => 10.0_f32,
    };
    let bg = if active {
        SURFACE1
    } else if hovered {
        SURFACE0
    } else {
        MANTLE
    };
    let border = if active || hovered { MAUVE } else { SURFACE0 };
    let dot_col = if active || hovered { MAUVE } else { SUBTEXT1 };

    let g = Group::new()
        .key(("annotate-size", idx))
        .size(Size::new(20.logical_px(), 20.logical_px()))
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(bg)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), border),
        )
        .child(
            Rect::new()
                .size(Size::new(dot_px.logical_px(), dot_px.logical_px()))
                .corner_radius((dot_px * 0.5).logical_px())
                .fill(dot_col)
                .position(Position::relative_to_parent(
                    Percent::new(50.0),
                    Percent::new(50.0),
                ))
                .anchor(ElementAnchor::Center),
        );

    bind_button_inputs(g, target, Msg::AnnotateSetSize(preset)).into()
}

#[allow(clippy::too_many_arguments)]
fn action_icon_btn(
    state: &State,
    key: &'static str,
    glyph: &'static str,
    target: HoverTarget,
    enabled: bool,
    active: bool,
    accent: Color,
    msg: Msg,
) -> Element<Msg> {
    let hovered = enabled && state.hovered == target;
    let bg = if active {
        SURFACE1
    } else if hovered {
        SURFACE0
    } else {
        MANTLE
    };
    let border = if active || hovered { accent } else { SURFACE0 };
    let fg = if !enabled {
        OVERLAY0
    } else if active || hovered {
        accent
    } else {
        SUBTEXT1
    };

    let g = Group::new()
        .key(key)
        .size(Size::new(BTN_SIZE.logical_px(), BTN_SIZE.logical_px()))
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(bg)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), border),
        )
        .child(
            Text::new(glyph)
                .font(font_ui())
                .font_size(11.logical_px())
                .bounds_mode(TextBoundsMode::Optical)
                .color(fg)
                .position(Position::relative_to_parent(
                    Percent::new(50.0),
                    Percent::new(50.0),
                ))
                .anchor(ElementAnchor::Center),
        );

    if enabled {
        bind_button_inputs(g, target, msg).into()
    } else {
        g.on_pointer_down(|_| Msg::Noop)
            .on_tablet(|_| Msg::Noop)
            .on_touch(|_| Msg::Noop)
            .into()
    }
}

fn clear_all_btn(state: &State, enabled: bool) -> Element<Msg> {
    let target = HoverTarget::AnnotateClearAll;
    let hovered = enabled && state.hovered == target;
    let bg = if hovered { SURFACE0 } else { MANTLE };
    let border = if hovered { RED } else { SURFACE0 };
    let fg = if !enabled {
        OVERLAY0
    } else if hovered {
        RED
    } else {
        SUBTEXT1
    };

    let g = Group::new()
        .key("annotate-clear-all")
        .size(Size::new(34.logical_px(), BTN_SIZE.logical_px()))
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(bg)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), border),
        )
        .child(
            Text::new("All")
                .font(font_ui_semibold())
                .font_size(9.5.logical_px())
                .color(fg)
                .position(Position::relative_to_parent(
                    Percent::new(50.0),
                    Percent::new(50.0),
                ))
                .anchor(ElementAnchor::Center),
        );

    if enabled {
        bind_button_inputs(g, target, Msg::AnnotateClearAll).into()
    } else {
        g.on_pointer_down(|_| Msg::Noop)
            .on_tablet(|_| Msg::Noop)
            .on_touch(|_| Msg::Noop)
            .into()
    }
}

#[must_use]
fn hover_hint_text(state: &State) -> Option<String> {
    match state.hovered {
        HoverTarget::AnnotateTool(AnnotateTool::Pen) => Some(String::from("Pen (P)")),
        HoverTarget::AnnotateTool(AnnotateTool::Highlighter) => {
            Some(String::from("Highlighter (H)"))
        }
        HoverTarget::AnnotateTool(AnnotateTool::Eraser) => {
            Some(String::from("Eraser (E · Right-Drag)"))
        }
        HoverTarget::AnnotateColor(idx) => {
            let name = ANNOTATE_PALETTE[idx % ANNOTATE_PALETTE.len()].1;
            Some(format!("{name} ({})", idx + 1))
        }
        HoverTarget::AnnotateSize(idx) => {
            let px = SIZE_PRESETS[idx % SIZE_PRESETS.len()].round() as u16;
            Some(format!("Size {px}px ([ / ] · Scroll)"))
        }
        HoverTarget::AnnotateUndo => Some(String::from("Undo (Ctrl+Z)")),
        HoverTarget::AnnotateRedo => Some(String::from("Redo (Ctrl+Y)")),
        HoverTarget::AnnotateClearWs => Some(String::from("Clear Workspace (C)")),
        HoverTarget::AnnotateClearAll => {
            let total = state.annotate.total_stroke_count();
            Some(format!("Clear All Workspaces · {total} strokes (Shift+C)"))
        }
        HoverTarget::AnnotatePin => Some(if state.annotate.pinned_visible {
            String::from("Pinned Visible (K)")
        } else {
            String::from("Keep Visible After Exit (K)")
        }),
        HoverTarget::AnnotateClose => Some(String::from("Exit & Keep Saved (Esc)")),
        _ => None,
    }
}
