//! Fullscreen native Screenshot overlay (`Layer::Overlay`) supporting Area drag selection,
//! Hyprland Window snapping, and Full-screen capture with Copy (`image/png`) & Save toggles.

use waywi::prelude::*;

use crate::app::screenshot::ScreenshotMode;
use crate::app::state::{HoverTarget, Msg, State};
use crate::theme::{
    BLUE, CRUST, GREEN, MANTLE, MAUVE, POPUP_TOP_MARGIN, RED, SUBTEXT1, SURFACE0, SURFACE1, TEXT,
    font_ui, font_ui_semibold,
};

const TOOLBAR_H: f32 = 34.0;
const BTN_H: f32 = 24.0;

pub fn build_screenshot_surface(state: &State) -> Option<Surface<Msg>> {
    let snapshot = state.screenshot.snapshot?;
    let w = state.screenshot.viewport_w.max(320.0);
    let h = state.screenshot.viewport_h.max(240.0);
    let selection = state.screenshot.active_selection_rect();
    let mode = state.screenshot.mode;

    let bg_img = Image::new(snapshot)
        .key("screenshot-bg")
        .size(Size::new(w.logical_px(), h.logical_px()))
        .fit(ImageFit::Fill)
        .filter(ImageFilter::Nearest)
        .input_enabled(false);

    // Dim everything outside the active selection rect so the target area/window pops cleanly.
    let scrim_canvas = Canvas::new(Size::new(w.logical_px(), h.logical_px()), move |p| {
        let dim = CRUST.with_opacity(0.46);
        match selection {
            Some((rx, ry, rw, rh)) => {
                // Top band
                if ry > 0.0 {
                    p.fill_rect(
                        Bounds::new(0.logical_px(), 0.logical_px(), w.logical_px(), ry.logical_px()),
                        dim,
                    );
                }
                // Bottom band
                let bottom_y = ry + rh;
                if bottom_y < h {
                    p.fill_rect(
                        Bounds::new(
                            0.logical_px(),
                            bottom_y.logical_px(),
                            w.logical_px(),
                            (h - bottom_y).logical_px(),
                        ),
                        dim,
                    );
                }
                // Left band
                if rx > 0.0 {
                    p.fill_rect(
                        Bounds::new(0.logical_px(), ry.logical_px(), rx.logical_px(), rh.logical_px()),
                        dim,
                    );
                }
                // Right band
                let right_x = rx + rw;
                if right_x < w {
                    p.fill_rect(
                        Bounds::new(
                            right_x.logical_px(),
                            ry.logical_px(),
                            (w - right_x).logical_px(),
                            rh.logical_px(),
                        ),
                        dim,
                    );
                }
                // Crisp selection outline
                let accent = match mode {
                    ScreenshotMode::Area => MAUVE,
                    ScreenshotMode::Window => BLUE,
                    ScreenshotMode::Full => GREEN,
                };
                p.stroke_rect(
                    Bounds::new(rx.logical_px(), ry.logical_px(), rw.logical_px(), rh.logical_px()),
                    1.5.logical_px(),
                    accent,
                );
            }
            None => {
                p.fill_rect(
                    Bounds::new(0.logical_px(), 0.logical_px(), w.logical_px(), h.logical_px()),
                    dim,
                );
            }
        }
    })
    .key("screenshot-scrim")
    .input_enabled(false);

    let mut stage = Group::new()
        .key("screenshot-stage")
        .size(Size::fill())
        .cursor(if mode == ScreenshotMode::Window {
            CursorShape::Pointer
        } else {
            CursorShape::Crosshair
        })
        .on_pointer_down(Msg::ScreenshotPointerDown)
        .on_pointer_move(Msg::ScreenshotPointerMove)
        .on_pointer_up(Msg::ScreenshotPointerUp)
        .child(bg_img)
        .child(scrim_canvas);

    // Live dimension / window badge anchored near the selection rectangle.
    if let Some((rx, ry, rw, rh)) = selection {
        let badge_text = if mode == ScreenshotMode::Window
            && let Some(win) = state.screenshot.hovered_window()
        {
            let name = if !win.class.is_empty() {
                win.class.as_str()
            } else {
                win.title.as_str()
            };
            format!("{name} · {}×{}", rw.round() as u32, rh.round() as u32)
        } else {
            format!("{}×{}", rw.round() as u32, rh.round() as u32)
        };
        let badge_y = if ry >= 26.0 {
            ry - 22.0
        } else {
            (ry + 6.0).min(h - 24.0)
        };
        let badge_x = rx.clamp(6.0, (w - 160.0).max(6.0));
        let badge = Group::new()
            .key("screenshot-sel-badge")
            .position(Position::absolute(badge_x.logical_px(), badge_y.logical_px()))
            .size(Size::auto())
            .input_enabled(false)
            .child(
                Rect::new()
                    .size(Size::fill())
                    .fill(CRUST.with_opacity(0.92))
                    .corner_radius(0.logical_px())
                    .border(1.logical_px(), SURFACE1),
            )
            .child(
                Row::new()
                    .padding(Edges::symmetric(2.logical_px(), 6.logical_px()))
                    .align(Align::Center)
                    .child(
                        Text::new(badge_text)
                            .font(font_ui_semibold())
                            .font_size(9.5.logical_px())
                            .color(MAUVE),
                    ),
            );
        stage = stage.child(badge);
    }

    let root = Group::new()
        .key("screenshot-root")
        .size(Size::fill())
        .child(stage)
        .child(build_screenshot_toolbar(state, w));

    let surface = Surface::floating("screen-screenshot", Output::Primary)
        .layer(Layer::Overlay)
        .ignore_exclusive_zone()
        .anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT)
        .size(Size::fill())
        .input_mode(InputMode::Full)
        .keyboard_interactivity(KeyboardInteractivity::Exclusive)
        .background(CRUST)
        .on_close(Msg::CloseScreenshot)
        .on_key(|ev| Some(Msg::ScreenshotKey(ev)))
        .child(root);

    Some(surface)
}

fn build_screenshot_toolbar(state: &State, viewport_w: f32) -> Element<Msg> {
    let mode = state.screenshot.mode;
    let row = Row::new()
        .size(Size::new(Length::Auto, TOOLBAR_H.logical_px()))
        .padding(Edges::symmetric(0.logical_px(), 8.logical_px()))
        .gap(5.logical_px())
        .align(Align::Center)
        .child(mode_pill(
            ScreenshotMode::Area,
            mode == ScreenshotMode::Area,
            state.hovered == HoverTarget::ScreenshotModeBtn(ScreenshotMode::Area),
        ))
        .child(mode_pill(
            ScreenshotMode::Window,
            mode == ScreenshotMode::Window,
            state.hovered == HoverTarget::ScreenshotModeBtn(ScreenshotMode::Window),
        ))
        .child(mode_pill(
            ScreenshotMode::Full,
            mode == ScreenshotMode::Full,
            state.hovered == HoverTarget::ScreenshotModeBtn(ScreenshotMode::Full),
        ))
        .child(divider())
        .child(toggle_pill(
            "ss-copy",
            "\u{f0c5}",
            "Copy",
            state.screenshot.copy_to_clipboard,
            state.hovered == HoverTarget::ScreenshotCopyToggle,
            GREEN,
            HoverTarget::ScreenshotCopyToggle,
            Msg::ScreenshotToggleCopy,
        ))
        .child(toggle_pill(
            "ss-save",
            "\u{f0c7}",
            "Save",
            state.screenshot.save_to_file,
            state.hovered == HoverTarget::ScreenshotSaveToggle,
            BLUE,
            HoverTarget::ScreenshotSaveToggle,
            Msg::ScreenshotToggleSave,
        ))
        .child(divider())
        .child(close_btn(state.hovered == HoverTarget::ScreenshotClose));

    Group::new()
        .key("screenshot-toolbar")
        .position(Position::absolute(
            (viewport_w * 0.5).logical_px(),
            POPUP_TOP_MARGIN.logical_px(),
        ))
        .anchor(ElementAnchor::TopCenter)
        .size(Size::new(Length::Auto, TOOLBAR_H.logical_px()))
        .on_pointer_down(|_| Msg::Noop)
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(CRUST.with_opacity(0.95))
                .corner_radius(0.logical_px())
                .border(1.logical_px(), SURFACE1),
        )
        .child(row)
        .into()
}

fn mode_pill(mode: ScreenshotMode, active: bool, hovered: bool) -> Element<Msg> {
    let target = HoverTarget::ScreenshotModeBtn(mode);
    let bg = if active {
        SURFACE1
    } else if hovered {
        SURFACE0
    } else {
        MANTLE
    };
    let border = if active || hovered { MAUVE } else { SURFACE1 };
    let fg = if active || hovered { MAUVE } else { TEXT };

    Group::new()
        .key(("ss-mode", mode as u8))
        .size(Size::new(Length::Auto, BTN_H.logical_px()))
        .cursor(CursorShape::Pointer)
        .on_hover(Msg::Hover(target))
        .on_leave(Msg::Leave(target))
        .on_click(Msg::ScreenshotSetMode(mode))
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(bg)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), border),
        )
        .child(
            Row::new()
                .size(Size::new(Length::Auto, BTN_H.logical_px()))
                .padding(Edges::symmetric(0.logical_px(), 7.logical_px()))
                .gap(5.logical_px())
                .align(Align::Center)
                .child(
                    Text::new(mode.icon())
                        .font(font_ui())
                        .font_size(10.5.logical_px())
                        .bounds_mode(TextBoundsMode::Optical)
                        .color(fg),
                )
                .child(
                    Text::new(mode.label())
                        .font(font_ui_semibold())
                        .font_size(9.5.logical_px())
                        .color(fg),
                ),
        )
        .into()
}

#[allow(clippy::too_many_arguments)]
fn toggle_pill(
    key: &'static str,
    icon: &'static str,
    label: &'static str,
    active: bool,
    hovered: bool,
    accent: Color,
    target: HoverTarget,
    on_click: Msg,
) -> Element<Msg> {
    let bg = if active {
        SURFACE1
    } else if hovered {
        SURFACE0
    } else {
        MANTLE
    };
    let border = if active || hovered { accent } else { SURFACE1 };
    let fg = if active {
        accent
    } else if hovered {
        TEXT
    } else {
        SUBTEXT1
    };

    Group::new()
        .key(key)
        .size(Size::new(Length::Auto, BTN_H.logical_px()))
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
        )
        .child(
            Row::new()
                .size(Size::new(Length::Auto, BTN_H.logical_px()))
                .padding(Edges::symmetric(0.logical_px(), 7.logical_px()))
                .gap(4.logical_px())
                .align(Align::Center)
                .child(
                    Text::new(icon)
                        .font(font_ui())
                        .font_size(10.logical_px())
                        .bounds_mode(TextBoundsMode::Optical)
                        .color(fg),
                )
                .child(
                    Text::new(label)
                        .font(font_ui_semibold())
                        .font_size(9.5.logical_px())
                        .color(fg),
                ),
        )
        .into()
}

fn close_btn(hovered: bool) -> Element<Msg> {
    Group::new()
        .key("ss-close")
        .size(Size::new(BTN_H.logical_px(), BTN_H.logical_px()))
        .cursor(CursorShape::Pointer)
        .on_hover(Msg::Hover(HoverTarget::ScreenshotClose))
        .on_leave(Msg::Leave(HoverTarget::ScreenshotClose))
        .on_click(Msg::CloseScreenshot)
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(if hovered { SURFACE0 } else { MANTLE })
                .corner_radius(0.logical_px())
                .border(1.logical_px(), if hovered { RED } else { SURFACE1 }),
        )
        .child(
            Text::new("\u{f00d}")
                .font(font_ui())
                .font_size(10.5.logical_px())
                .bounds_mode(TextBoundsMode::Optical)
                .color(if hovered { RED } else { SUBTEXT1 })
                .position(Position::relative_to_parent(
                    Percent::new(50.0),
                    Percent::new(50.0),
                ))
                .anchor(ElementAnchor::Center),
        )
        .into()
}

fn divider() -> Element<Msg> {
    Rect::new()
        .size(Size::new(1.logical_px(), 14.logical_px()))
        .fill(SURFACE1)
        .into()
}

// ============================================================================
// Android-style floating post-capture preview card
// ============================================================================

const PREVIEW_W: f32 = 196.0;
const PREVIEW_THUMB_W: f32 = 180.0;
const PREVIEW_THUMB_H: f32 = 102.0;
const PREVIEW_ACTION_H: f32 = 24.0;
const PREVIEW_FULL_H: f32 = 152.0;

/// Builds the transient bottom-left Android-style screenshot preview toast with Open, Copy,
/// Folder, Delete, and Dismiss actions (non-focus-stealing, click-through outside the card).
pub fn build_screenshot_preview_surface(state: &State) -> Option<Surface<Msg>> {
    let preview = state.screenshot.preview.as_ref()?;
    let ease = preview.anim.progress();
    if ease <= 0.01 {
        return None;
    }

    let slide_y = (1.0 - ease) * 18.0;
    let has_file = preview.saved_path.is_some();

    let thumb_box = Group::new()
        .key("ss-preview-thumb")
        .size(Size::new(
            PREVIEW_THUMB_W.logical_px(),
            PREVIEW_THUMB_H.logical_px(),
        ))
        .cursor(CursorShape::Pointer)
        .on_click(Msg::ScreenshotPreviewOpen)
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(MANTLE)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), SURFACE0),
        )
        .child(
            Clip::new(
                Image::new(preview.thumb)
                    .size(Size::fill())
                    .fit(ImageFit::Contain),
            )
            .size(Size::fill()),
        )
        .child(
            Group::new()
                .position(Position::absolute(4.logical_px(), 4.logical_px()))
                .size(Size::auto())
                .input_enabled(false)
                .child(
                    Rect::new()
                        .size(Size::fill())
                        .fill(CRUST.with_opacity(0.88))
                        .corner_radius(0.logical_px())
                        .border(1.logical_px(), SURFACE1),
                )
                .child(
                    Row::new()
                        .padding(Edges::symmetric(1.logical_px(), 5.logical_px()))
                        .gap(4.logical_px())
                        .align(Align::Center)
                        .child(
                            Text::new(format!("{}×{}", preview.width_px, preview.height_px))
                                .font(font_ui_semibold())
                                .font_size(8.5.logical_px())
                                .color(MAUVE),
                        )
                        .child(
                            Text::new(if preview.copied && has_file {
                                "saved · copied"
                            } else if has_file {
                                "saved"
                            } else {
                                "copied"
                            })
                            .font(font_ui())
                            .font_size(8.logical_px())
                            .color(GREEN),
                        ),
                ),
        );

    let mut actions = Row::new()
        .size(Size::new(
            PREVIEW_THUMB_W.logical_px(),
            PREVIEW_ACTION_H.logical_px(),
        ))
        .gap(4.logical_px())
        .align(Align::Center)
        .child(preview_action_btn(
            "ss-prev-open",
            "\u{f08e}",
            Some("Open"),
            state.hovered == HoverTarget::ScreenshotPreviewOpen,
            MAUVE,
            HoverTarget::ScreenshotPreviewOpen,
            Msg::ScreenshotPreviewOpen,
        ))
        .child(preview_action_btn(
            "ss-prev-copy",
            "\u{f0c5}",
            None,
            state.hovered == HoverTarget::ScreenshotPreviewCopy,
            GREEN,
            HoverTarget::ScreenshotPreviewCopy,
            Msg::ScreenshotPreviewCopy,
        ));

    if has_file {
        actions = actions
            .child(preview_action_btn(
                "ss-prev-folder",
                "\u{f07b}",
                None,
                state.hovered == HoverTarget::ScreenshotPreviewFolder,
                BLUE,
                HoverTarget::ScreenshotPreviewFolder,
                Msg::ScreenshotPreviewFolder,
            ))
            .child(preview_action_btn(
                "ss-prev-del",
                "\u{f1f8}",
                None,
                state.hovered == HoverTarget::ScreenshotPreviewDelete,
                RED,
                HoverTarget::ScreenshotPreviewDelete,
                Msg::ScreenshotPreviewDelete,
            ));
    }

    actions = actions.child(preview_action_btn(
        "ss-prev-close",
        "\u{f00d}",
        None,
        state.hovered == HoverTarget::ScreenshotPreviewClose,
        RED,
        HoverTarget::ScreenshotPreviewClose,
        Msg::ScreenshotPreviewDismiss,
    ));

    let card = Group::new()
        .key("ss-preview-card")
        .position(Position::absolute(0.logical_px(), slide_y.logical_px()))
        .size(Size::new(PREVIEW_W.logical_px(), PREVIEW_FULL_H.logical_px()))
        .opacity(ease)
        .on_hover(Msg::ScreenshotPreviewHover(true))
        .on_leave(Msg::ScreenshotPreviewHover(false))
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(CRUST.with_opacity(0.96))
                .corner_radius(0.logical_px())
                .border(1.logical_px(), SURFACE1),
        )
        .child(
            Column::new()
                .size(Size::fill())
                .padding(Edges::all(8.logical_px()))
                .gap(8.logical_px())
                .align(Align::Center)
                .child(thumb_box)
                .child(actions),
        );

    let surface = Surface::floating("screen-screenshot-preview", Output::Primary)
        .layer(Layer::Overlay)
        .ignore_exclusive_zone()
        .anchor(Anchor::BOTTOM | Anchor::LEFT)
        .margin(Edges::new(
            0.logical_px(),
            0.logical_px(),
            16.logical_px(),
            16.logical_px(),
        ))
        .size(Size::new(
            PREVIEW_W.logical_px(),
            (PREVIEW_FULL_H + 18.0).logical_px(),
        ))
        .input_mode(InputMode::Selective)
        .keyboard_interactivity(KeyboardInteractivity::None)
        .background(Color::TRANSPARENT)
        .on_close(Msg::ScreenshotPreviewDismiss)
        .child(card);

    Some(surface)
}

fn preview_action_btn(
    key: &'static str,
    icon: &'static str,
    label: Option<&'static str>,
    hovered: bool,
    accent: Color,
    target: HoverTarget,
    on_click: Msg,
) -> Element<Msg> {
    let fg = if hovered { accent } else { SUBTEXT1 };
    let border = if hovered { accent } else { SURFACE1 };
    let bg = if hovered { SURFACE0 } else { MANTLE };

    let mut btn = Group::new()
        .key(key)
        .cursor(CursorShape::Pointer)
        .on_hover(Msg::Hover(target))
        .on_leave(Msg::Leave(target))
        .on_click(on_click);

    if let Some(text) = label {
        btn = btn
            .size(Size::new(Length::Auto, PREVIEW_ACTION_H.logical_px()))
            .child(
                Rect::new()
                    .size(Size::fill())
                    .fill(bg)
                    .corner_radius(0.logical_px())
                    .border(1.logical_px(), border),
            )
            .child(
                Row::new()
                    .size(Size::new(Length::Auto, PREVIEW_ACTION_H.logical_px()))
                    .padding(Edges::symmetric(0.logical_px(), 7.logical_px()))
                    .gap(5.logical_px())
                    .align(Align::Center)
                    .child(
                        Text::new(icon)
                            .font(font_ui())
                            .font_size(10.logical_px())
                            .bounds_mode(TextBoundsMode::Optical)
                            .color(fg),
                    )
                    .child(
                        Text::new(text)
                            .font(font_ui_semibold())
                            .font_size(9.5.logical_px())
                            .color(if hovered { accent } else { TEXT }),
                    ),
            );
    } else {
        btn = btn
            .size(Size::new(
                PREVIEW_ACTION_H.logical_px(),
                PREVIEW_ACTION_H.logical_px(),
            ))
            .child(
                Rect::new()
                    .size(Size::fill())
                    .fill(bg)
                    .corner_radius(0.logical_px())
                    .border(1.logical_px(), border),
            )
            .child(
                Text::new(icon)
                    .font(font_ui())
                    .font_size(10.logical_px())
                    .bounds_mode(TextBoundsMode::Optical)
                    .color(fg)
                    .position(Position::relative_to_parent(
                        Percent::new(50.0),
                        Percent::new(50.0),
                    ))
                    .anchor(ElementAnchor::Center),
            );
    }
    btn.into()
}
