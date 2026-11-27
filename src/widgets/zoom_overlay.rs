//! Fullscreen animated Screen Magnifier overlay (`Layer::Overlay`) supporting:
//! - **Snapshot mode**: WayWi `screencopy` frozen frame with zero-copy GPU/CPU transforms,
//!   deadzone follow, scroll zoom, drag pan, and bottom HUD.
//! - **Live mode**: Click-through (`InputMode::Selective`) compact bottom HUD pill while
//!   Hyprland's native compositor zoom (`cursor:zoom_factor`) runs live at 60–144Hz.

use waywi::prelude::*;

use crate::app::state::{HoverTarget, Msg, State};
use crate::app::zoom::ZoomMode;
use crate::theme::{
    CRUST, GREEN, MANTLE, MAUVE, OVERLAY0, PEACH, RED, SUBTEXT1, SURFACE0, SURFACE1, TEXT,
    font_ui, font_ui_semibold,
};

const HUD_HEIGHT: f32 = 32.0;
const HUD_BOTTOM_MARGIN: f32 = 16.0;
const HUD_BTN_SIZE: f32 = 22.0;

pub fn build_zoom_surface(state: &State) -> Option<Surface<Msg>> {
    let w = state.zoom.viewport_w.max(320.0);
    let h = state.zoom.viewport_h.max(240.0);
    let active = state.zoom.is_active();
    let is_live = state.zoom.mode == ZoomMode::Live;

    if is_live {
        // Live Hyprland compositor zoom: mount ONLY the click-through bottom HUD pill so all
        // desktop windows remain 100% live and interactive underneath.
        let mut root = Group::new().key("zoom-live-root").size(Size::fill());
        if state.zoom.hud_progress > 0.02 {
            root = root.child(build_zoom_hud(state, w, h, true));
        }

        let surface = Surface::floating("screen-zoom", Output::Primary)
            .layer(Layer::Overlay)
            .ignore_exclusive_zone()
            .anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT)
            .size(Size::fill())
            .input_mode(InputMode::Selective)
            .keyboard_interactivity(KeyboardInteractivity::None)
            .background(Color::TRANSPARENT)
            .on_close(Msg::CloseZoom)
            .child(root);
        return Some(surface);
    }

    let snapshot = state.zoom.snapshot?;
    let zoomed_image = Image::new(snapshot)
        .key("zoom-snapshot-image")
        .size(Size::new(w.logical_px(), h.logical_px()))
        .fit(ImageFit::Fill)
        .filter(state.zoom.image_filter())
        .transform(state.zoom.image_transform())
        .input_enabled(false);

    let clipped_view = Clip::new(zoomed_image)
        .key("zoom-snapshot-clip")
        .size(Size::fill())
        .input_enabled(false);

    let mut stage = Group::new()
        .key("zoom-stage")
        .size(Size::fill())
        .child(clipped_view);

    if active {
        let cursor = if state.zoom.drag_last.is_some() {
            CursorShape::Grabbing
        } else {
            CursorShape::Grab
        };
        stage = stage
            .cursor(cursor)
            .on_pointer_down(Msg::ZoomPointerDown)
            .on_pointer_move(Msg::ZoomPointerMove)
            .on_pointer_up(|_| Msg::ZoomPointerUp)
            .on_scroll(Msg::ZoomScroll);
    }

    let mut root = Group::new()
        .key("zoom-root")
        .size(Size::fill())
        .child(stage);

    if state.zoom.hud_progress > 0.02 {
        root = root.child(build_zoom_hud(state, w, h, false));
    }

    let mut surface = Surface::floating("screen-zoom", Output::Primary)
        .layer(Layer::Overlay)
        .ignore_exclusive_zone()
        .anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT)
        .size(Size::fill())
        .input_mode(InputMode::Full)
        .background(CRUST)
        .on_close(Msg::CloseZoom);

    if active {
        surface = surface
            .keyboard_interactivity(KeyboardInteractivity::Exclusive)
            .on_key(|ev| Some(Msg::ZoomKey(ev)));
    } else {
        surface = surface.keyboard_interactivity(KeyboardInteractivity::None);
    }

    Some(surface.child(root))
}

fn build_zoom_hud(state: &State, viewport_w: f32, viewport_h: f32, is_live: bool) -> Element<Msg> {
    let ease = state.zoom.hud_progress.clamp(0.0, 1.0);
    let slide = (1.0 - ease) * 14.0;
    let y = (viewport_h - HUD_HEIGHT - HUD_BOTTOM_MARGIN + slide).max(8.0);

    let zoom_label = format!("{:.1}x", state.zoom.zoom);
    let filter_active = state.zoom.crisp_pixels;

    let mut row = Row::new()
        .size(Size::new(Length::Auto, HUD_HEIGHT.logical_px()))
        .padding(Edges::symmetric(0.logical_px(), 8.logical_px()))
        .gap(5.logical_px())
        .align(Align::Center);

    if is_live {
        row = row
            .child(
                Text::new("\u{f111}")
                    .font(font_ui())
                    .font_size(7.logical_px())
                    .bounds_mode(TextBoundsMode::Optical)
                    .color(GREEN),
            )
            .child(
                Text::new("LIVE")
                    .font(font_ui_semibold())
                    .font_size(9.logical_px())
                    .color(GREEN),
            );
    }

    row = row
        .child(
            Text::new(zoom_label)
                .font(font_ui_semibold())
                .font_size(10.5.logical_px())
                .color(MAUVE),
        )
        .child(hud_divider())
        .child(hud_icon_btn(
            "zoom-hud-minus",
            "\u{f010}",
            false,
            state.hovered == HoverTarget::ZoomHudMinus,
            MAUVE,
            HoverTarget::ZoomHudMinus,
            Msg::ZoomStep(-1),
        ))
        .child(hud_icon_btn(
            "zoom-hud-plus",
            "\u{f00e}",
            false,
            state.hovered == HoverTarget::ZoomHudPlus,
            MAUVE,
            HoverTarget::ZoomHudPlus,
            Msg::ZoomStep(1),
        ))
        .child(hud_divider())
        .child(hud_icon_btn(
            "zoom-hud-live",
            if is_live { "\u{f2dc}" } else { "\u{f03d}" },
            is_live,
            state.hovered == HoverTarget::ZoomHudLive,
            GREEN,
            HoverTarget::ZoomHudLive,
            Msg::ZoomToggleLive,
        ))
        .child(hud_icon_btn(
            "zoom-hud-filter",
            "\u{f00a}",
            filter_active,
            state.hovered == HoverTarget::ZoomHudFilter,
            PEACH,
            HoverTarget::ZoomHudFilter,
            Msg::ZoomToggleFilter,
        ))
        .child(hud_icon_btn(
            "zoom-hud-reset",
            "\u{f05b}",
            false,
            state.hovered == HoverTarget::ZoomHudReset,
            MAUVE,
            HoverTarget::ZoomHudReset,
            Msg::ZoomReset,
        ));

    if !is_live {
        row = row.child(hud_icon_btn(
            "zoom-hud-recapture",
            "\u{f021}",
            false,
            state.hovered == HoverTarget::ZoomHudRecapture,
            MAUVE,
            HoverTarget::ZoomHudRecapture,
            Msg::ZoomRecapture,
        ));
    }

    row = row
        .child(hud_divider())
        .child(
            Text::new(if filter_active { "px" } else { "aa" })
                .font(font_ui())
                .font_size(9.logical_px())
                .color(if filter_active { PEACH } else { OVERLAY0 }),
        )
        .child(hud_icon_btn(
            "zoom-hud-close",
            "\u{f00d}",
            false,
            state.hovered == HoverTarget::ZoomHudClose,
            RED,
            HoverTarget::ZoomHudClose,
            Msg::CloseZoom,
        ));

    Group::new()
        .key("zoom-hud-bar")
        .position(Position::absolute(
            (viewport_w * 0.5).logical_px(),
            y.logical_px(),
        ))
        .anchor(ElementAnchor::TopCenter)
        .size(Size::new(Length::Auto, HUD_HEIGHT.logical_px()))
        .opacity(ease)
        .on_hover(Msg::ZoomHudHover(true))
        .on_leave(Msg::ZoomHudHover(false))
        .on_pointer_down(|_| Msg::Noop)
        .on_scroll(Msg::ZoomScroll)
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(CRUST.with_opacity(0.92))
                .corner_radius(0.logical_px())
                .border(1.logical_px(), SURFACE1),
        )
        .child(row)
        .into()
}

fn hud_divider() -> Element<Msg> {
    Rect::new()
        .size(Size::new(1.logical_px(), 14.logical_px()))
        .fill(SURFACE1)
        .into()
}

fn hud_icon_btn(
    key: &'static str,
    icon: &'static str,
    active: bool,
    hovered: bool,
    accent: Color,
    hover_target: HoverTarget,
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
    let fg = if active || hovered { accent } else { SUBTEXT1 };

    Group::new()
        .key(key)
        .size(Size::new(HUD_BTN_SIZE.logical_px(), HUD_BTN_SIZE.logical_px()))
        .cursor(CursorShape::Pointer)
        .on_hover(Msg::Hover(hover_target))
        .on_leave(Msg::Leave(hover_target))
        .on_click(on_click)
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
                .font_size(10.5.logical_px())
                .bounds_mode(TextBoundsMode::Optical)
                .color(if active || hovered { fg } else { TEXT })
                .position(Position::relative_to_parent(
                    Percent::new(50.0),
                    Percent::new(50.0),
                ))
                .anchor(ElementAnchor::Center),
        )
        .into()
}
