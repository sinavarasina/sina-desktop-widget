//! Fullscreen native Color Picker overlay (`Layer::Overlay`) with an `11x11` pixel loupe,
//! live `#rrggbb` / `rgb(r, g, b)` readout, and Wayland clipboard copy.

use std::sync::Arc;

use waywi::prelude::*;

use crate::app::state::{Msg, State};
use crate::theme::{
    CRUST, MANTLE, MAUVE, OVERLAY0, PEACH, SUBTEXT0, SURFACE0, SURFACE1, TEXT, font_ui,
    font_ui_semibold,
};

const LOUPE_GRID: i32 = 11;
const LOUPE_HALF: i32 = LOUPE_GRID / 2;
const LOUPE_CELL_PX: f32 = 10.0;
const LOUPE_CANVAS_PX: f32 = (LOUPE_GRID as f32) * LOUPE_CELL_PX;
const CARD_W: f32 = 152.0;
const CARD_H: f32 = LOUPE_CANVAS_PX + 66.0;
const CURSOR_OFFSET: f32 = 20.0;

pub fn build_color_picker_surface(state: &State) -> Option<Surface<Msg>> {
    let snapshot = state.color_picker.snapshot?;
    let frame = state.color_picker.frame.clone()?;
    let w = state.color_picker.viewport_w.max(320.0);
    let h = state.color_picker.viewport_h.max(240.0);
    let (cx, cy) = state.color_picker.cursor_pos;
    let sampled = state.color_picker.sampled_rgba();
    let sample_col = Color::rgba(sampled[0], sampled[1], sampled[2], 255);
    let formatted = state.color_picker.formatted_current();
    let fmt_label = state.color_picker.format.label();

    let (px, py) = state.color_picker.logical_to_pixel(cx, cy).unwrap_or((0, 0));

    // Auto-flip loupe card so it never clips off the right or bottom edge.
    let card_x = if cx + CURSOR_OFFSET + CARD_W <= w - 8.0 {
        cx + CURSOR_OFFSET
    } else {
        (cx - CURSOR_OFFSET - CARD_W).max(8.0)
    };
    let card_y = if cy + CURSOR_OFFSET + CARD_H <= h - 8.0 {
        cy + CURSOR_OFFSET
    } else {
        (cy - CURSOR_OFFSET - CARD_H).max(8.0)
    };

    let loupe = build_loupe_canvas(frame, px, py);

    let swatch = Rect::new()
        .size(Size::new(18.logical_px(), 18.logical_px()))
        .fill(sample_col)
        .corner_radius(0.logical_px())
        .border(1.logical_px(), SURFACE1);

    let readout = Column::new()
        .size(Size::auto())
        .gap(1.logical_px())
        .align(Align::Start)
        .child(
            Text::new(formatted)
                .font(font_ui_semibold())
                .font_size(10.logical_px())
                .color(TEXT),
        )
        .child(
            Text::new(format!("{fmt_label} · {px},{py}"))
                .font(font_ui())
                .font_size(8.5.logical_px())
                .color(SUBTEXT0),
        );

    let info_row = Row::new()
        .size(Size::new((CARD_W - 16.0).logical_px(), 22.logical_px()))
        .gap(7.logical_px())
        .align(Align::Center)
        .child(swatch)
        .child(readout);

    let hint = Text::new("LMB copy · RMB fmt · Esc")
        .font(font_ui())
        .font_size(8.logical_px())
        .color(OVERLAY0);

    let loupe_card = Group::new()
        .key("colorpicker-loupe-card")
        .position(Position::absolute(card_x.logical_px(), card_y.logical_px()))
        .size(Size::new(CARD_W.logical_px(), CARD_H.logical_px()))
        .input_enabled(false)
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(CRUST.with_opacity(0.95))
                .corner_radius(0.logical_px())
                .border(1.logical_px(), PEACH),
        )
        .child(
            Column::new()
                .size(Size::fill())
                .padding(Edges::all(8.logical_px()))
                .gap(6.logical_px())
                .align(Align::Center)
                .child(loupe)
                .child(info_row)
                .child(hint),
        );

    let bg_img = Image::new(snapshot)
        .key("colorpicker-bg")
        .size(Size::new(w.logical_px(), h.logical_px()))
        .fit(ImageFit::Fill)
        .filter(ImageFilter::Nearest)
        .input_enabled(false);

    let stage = Group::new()
        .key("colorpicker-stage")
        .size(Size::fill())
        .cursor(CursorShape::Crosshair)
        .on_pointer_move(Msg::ColorPickerPointerMove)
        .on_pointer_down(Msg::ColorPickerPointerDown)
        .on_scroll(Msg::ColorPickerScroll)
        .child(bg_img)
        .child(loupe_card);

    let surface = Surface::floating("screen-colorpicker", Output::Primary)
        .layer(Layer::Overlay)
        .ignore_exclusive_zone()
        .anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT)
        .size(state.viewport_size())
        .input_mode(InputMode::Full)
        .keyboard_interactivity(KeyboardInteractivity::Exclusive)
        .background(MANTLE)
        .on_close(Msg::CloseColorPicker)
        .on_key(|ev| Some(Msg::ColorPickerKey(ev)))
        .child(stage);

    Some(surface)
}

fn build_loupe_canvas(frame: Arc<ScreencopyFrame>, cx: u32, cy: u32) -> Element<Msg> {
    let canvas = Canvas::new(
        Size::new(LOUPE_CANVAS_PX.logical_px(), LOUPE_CANVAS_PX.logical_px()),
        move |p| {
            p.clear(CRUST);
            let fw = frame.width as i64;
            let fh = frame.height as i64;
            for dy in -LOUPE_HALF..=LOUPE_HALF {
                for dx in -LOUPE_HALF..=LOUPE_HALF {
                    let sx = (cx as i64 + dx as i64).clamp(0, (fw - 1).max(0)) as u32;
                    let sy = (cy as i64 + dy as i64).clamp(0, (fh - 1).max(0)) as u32;
                    let [r, g, b, _] = frame.pixel_rgba(sx, sy).unwrap_or([30, 30, 46, 255]);
                    let gx = ((dx + LOUPE_HALF) as f32) * LOUPE_CELL_PX;
                    let gy = ((dy + LOUPE_HALF) as f32) * LOUPE_CELL_PX;
                    p.fill_rect(
                        Bounds::new(
                            gx.logical_px(),
                            gy.logical_px(),
                            LOUPE_CELL_PX.logical_px(),
                            LOUPE_CELL_PX.logical_px(),
                        ),
                        Color::rgba(r, g, b, 255),
                    );
                }
            }
            // Subtle pixel grid hairlines.
            let grid_col = SURFACE0.with_opacity(0.35);
            for i in 1..LOUPE_GRID {
                let pos = (i as f32) * LOUPE_CELL_PX;
                p.fill_rect(
                    Bounds::new(pos.logical_px(), 0.logical_px(), 0.5.logical_px(), LOUPE_CANVAS_PX.logical_px()),
                    grid_col,
                );
                p.fill_rect(
                    Bounds::new(0.logical_px(), pos.logical_px(), LOUPE_CANVAS_PX.logical_px(), 0.5.logical_px()),
                    grid_col,
                );
            }
            // Crisp center pixel reticle (dark outer + mauve inner).
            let mid = (LOUPE_HALF as f32) * LOUPE_CELL_PX;
            p.stroke_rect(
                Bounds::new(
                    (mid - 1.0).logical_px(),
                    (mid - 1.0).logical_px(),
                    (LOUPE_CELL_PX + 2.0).logical_px(),
                    (LOUPE_CELL_PX + 2.0).logical_px(),
                ),
                1.0.logical_px(),
                CRUST,
            );
            p.stroke_rect(
                Bounds::new(
                    mid.logical_px(),
                    mid.logical_px(),
                    LOUPE_CELL_PX.logical_px(),
                    LOUPE_CELL_PX.logical_px(),
                ),
                1.2.logical_px(),
                MAUVE,
            );
        },
    )
    .key("colorpicker-loupe-grid")
    .cache_key(("cp-loupe", cx, cy))
    .input_enabled(false);

    Group::new()
        .size(Size::new(
            (LOUPE_CANVAS_PX + 2.0).logical_px(),
            (LOUPE_CANVAS_PX + 2.0).logical_px(),
        ))
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(CRUST)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), SURFACE1),
        )
        .child(
            Group::new()
                .position(Position::absolute(1.logical_px(), 1.logical_px()))
                .size(Size::new(
                    LOUPE_CANVAS_PX.logical_px(),
                    LOUPE_CANVAS_PX.logical_px(),
                ))
                .child(canvas),
        )
        .into()
}
