//! Right-bar mirror status pill on the left of MPRIS (`\ \ [Privacy | Clipboard | Active] >`).
//!
//! Geometrically mirrors the left-bar Notifications pill (`\ [Notif] / /`) with reversed
//! slant & thin accent border, combining:
//! - Android-style vertical capability-gated Privacy micro-rectangles (Camera / Mic / ScreenShare)
//! - Native Wayland Clipboard history trigger (+ live hex swatch from Color Picker)
//! - Compact active indicators for Caffeine (`idle-inhibit`), Live Zoom, and Pinned Annotations

use waywi::prelude::*;

use crate::app::state::{HoverTarget, Msg, State};
use crate::theme::{
    BAR_HEIGHT, BLUE, GREEN, MANTLE, MAUVE, PEACH, RED, SUBTEXT1, SURFACE0, SURFACE1, TEXT,
    font_ui, font_ui_semibold,
};
use crate::widgets::separators::{SepShape, separator};

/// Builds the standalone Privacy (+ active tool badge) pill on the far left of the right bar
/// cluster (`\ \ [Privacy] <` when MPRIS is visible, or `\ \ [Privacy] >` when MPRIS is hidden so
/// its right edge interlocks seamlessly with Clipboard's inward `>` notch).
pub fn build_privacy_pill(state: &State, adjacent_mpris: bool) -> Element<Msg> {
    let pill_hovered = matches!(
        state.hovered,
        HoverTarget::PrivacyStatus | HoverTarget::ActiveToolBadge
    );
    let pill_bg = if pill_hovered { SURFACE0 } else { MANTLE };

    let mut inner = Row::new()
        .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
        .padding(Edges::symmetric(0.logical_px(), 5.logical_px()))
        .gap(5.logical_px())
        .align(Align::Center)
        .child(build_privacy_bars(state));

    if let Some(tool_badge) = build_active_tool_badge(state) {
        inner = inner.child(tool_badge);
    }

    let body = Group::new()
        .key("privacy-pill-body")
        .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
        .child(Rect::new().size(Size::fill()).fill(pill_bg))
        .child(inner);

    let right_cap = if adjacent_mpris {
        separator("sep-privacy-r", SepShape::CurveR, pill_bg, Color::TRANSPARENT)
    } else {
        separator("sep-privacy-r", SepShape::ArrowR, pill_bg, Color::TRANSPARENT)
    };

    Row::new()
        .key("mod-privacy-pill")
        .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
        .align(Align::Center)
        .child(separator(
            "sep-privacy-l-thin",
            SepShape::SlashLThin,
            SURFACE0,
            Color::TRANSPARENT,
        ))
        .child(separator(
            "sep-privacy-l",
            SepShape::SlashL,
            pill_bg,
            Color::TRANSPARENT,
        ))
        .child(body)
        .child(right_cap)
        .into()
}

/// Builds the standalone Clipboard (+ Caffeine indicator) pill placed immediately to the left of
/// the System Tray: inward diamond left notch (`CurveL`) + slash right edge (`SlashRInv`) with a
/// clean diagonal gap against Tray's `SlashL`.
pub fn build_clipboard_pill(state: &State) -> Element<Msg> {
    let pill_hovered = matches!(
        state.hovered,
        HoverTarget::ClipboardBar | HoverTarget::CaffeineBadge
    );
    let pill_bg = if pill_hovered || state.clipboard.is_popup_open() {
        SURFACE0
    } else {
        MANTLE
    };

    let mut inner = Row::new()
        .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
        .padding(Edges::symmetric(0.logical_px(), 6.logical_px()))
        .gap(5.logical_px())
        .align(Align::Center)
        .child(build_clipboard_segment(state));

    if state.caffeine_active {
        inner = inner.child(build_caffeine_badge(state));
    }

    let body = Group::new()
        .key("clipboard-pill-body")
        .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
        .child(Rect::new().size(Size::fill()).fill(pill_bg))
        .child(inner);

    Row::new()
        .key("mod-clipboard-pill")
        .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
        .align(Align::Center)
        .child(separator(
            "sep-clipboard-l",
            SepShape::CurveL,
            pill_bg,
            Color::TRANSPARENT,
        ))
        .child(body)
        .child(separator(
            "sep-clipboard-r",
            SepShape::SlashRInv,
            pill_bg,
            Color::TRANSPARENT,
        ))
        .into()
}

/// Vertical stack of sharp capability-gated privacy indicator rectangles (`5x3px`, `0px` radius).
fn build_privacy_bars(state: &State) -> Element<Msg> {
    let p = &state.privacy;
    let has_mic_hw = state
        .audio
        .as_ref()
        .is_none_or(|a| !a.sources.is_empty());

    let mut dots: Vec<Color> = Vec::with_capacity(4);
    if state.has_camera_hw || p.camera_in_use {
        dots.push(if p.camera_in_use { GREEN } else { SURFACE1 });
    }
    if has_mic_hw || p.microphone_in_use || p.audio_capture_in_use {
        let col = if p.audio_capture_in_use {
            RED
        } else if p.microphone_in_use {
            PEACH
        } else {
            SURFACE1
        };
        dots.push(col);
    }
    // Screen share (Wayland screencast portal) is always relevant on Wayland.
    dots.push(if p.screen_share_in_use { BLUE } else { SURFACE1 });
    // Location (GeoClue2) only occupies a slot when actively in use on desktop hosts.
    if p.location_in_use {
        dots.push(MAUVE);
    }

    let mut col = Column::new()
        .size(Size::auto())
        .gap(2.logical_px())
        .align(Align::Center);
    for color in dots {
        col = col.child(
            Rect::new()
                .size(Size::new(5.logical_px(), 3.logical_px()))
                .fill(color)
                .corner_radius(0.logical_px()),
        );
    }

    Group::new()
        .key("status-privacy-bars")
        .size(Size::new(9.logical_px(), BAR_HEIGHT.logical_px()))
        .on_hover(Msg::Hover(HoverTarget::PrivacyStatus))
        .on_leave(Msg::Leave(HoverTarget::PrivacyStatus))
        .child(
            col.position(Position::relative_to_parent(
                Percent::new(50.0),
                Percent::new(50.0),
            ))
            .anchor(ElementAnchor::Center),
        )
        .into()
}

fn build_clipboard_segment(state: &State) -> Element<Msg> {
    let hovered = state.hovered == HoverTarget::ClipboardBar || state.clipboard.is_popup_open();
    let count = state.clipboard.entries.len();
    let icon_col = if hovered {
        MAUVE
    } else if count > 0 {
        TEXT
    } else {
        SUBTEXT1
    };

    // Compact icon + superscript count (`pangkat`) so the count stays clean and unobtrusive.
    let badge_w = if count == 0 {
        13.0
    } else if count < 10 {
        19.0
    } else {
        24.0
    };
    let mut icon_badge = Group::new()
        .size(Size::new(badge_w.logical_px(), BAR_HEIGHT.logical_px()))
        .child(
            Text::new("\u{f0ea}")
                .font(font_ui())
                .font_size(11.logical_px())
                .bounds_mode(TextBoundsMode::Optical)
                .color(icon_col)
                .position(Position::absolute(0.logical_px(), (BAR_HEIGHT * 0.5).logical_px()))
                .anchor(ElementAnchor::CenterLeft),
        );

    if count > 0 {
        icon_badge = icon_badge.child(
            Text::new(count.to_string())
                .font(font_ui_semibold())
                .font_size(7.5.logical_px())
                .line_height(1.0)
                .color(if hovered { TEXT } else { MAUVE })
                .position(Position::absolute(12.logical_px(), 3.logical_px())),
        );
    }

    let mut row = Row::new()
        .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
        .gap(4.logical_px())
        .align(Align::Center)
        .child(icon_badge);

    if let Some(swatch_col) = state.clipboard.latest_hex_color() {
        row = row.child(
            Rect::new()
                .size(Size::new(8.logical_px(), 8.logical_px()))
                .fill(swatch_col)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), SURFACE1),
        );
    }

    Group::new()
        .key("status-clipboard-btn")
        .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
        .cursor(CursorShape::Pointer)
        .on_hover(Msg::Hover(HoverTarget::ClipboardBar))
        .on_leave(Msg::Leave(HoverTarget::ClipboardBar))
        .on_pointer_down(Msg::ClipboardBarPointerDown)
        .child(row)
        .into()
}

fn build_caffeine_badge(state: &State) -> Element<Msg> {
    let hovered = state.hovered == HoverTarget::CaffeineBadge;
    Group::new()
        .key("status-caffeine-badge")
        .size(Size::new(14.logical_px(), BAR_HEIGHT.logical_px()))
        .cursor(CursorShape::Pointer)
        .on_hover(Msg::Hover(HoverTarget::CaffeineBadge))
        .on_leave(Msg::Leave(HoverTarget::CaffeineBadge))
        .on_click(Msg::ToggleCaffeine)
        .child(
            Text::new("\u{f0f4}")
                .font(font_ui())
                .font_size(10.5.logical_px())
                .bounds_mode(TextBoundsMode::Optical)
                .color(if hovered { TEXT } else { PEACH })
                .position(Position::relative_to_parent(
                    Percent::new(50.0),
                    Percent::new(50.0),
                ))
                .anchor(ElementAnchor::Center),
        )
        .into()
}

fn build_active_tool_badge(state: &State) -> Option<Element<Msg>> {
    let hovered = state.hovered == HoverTarget::ActiveToolBadge;
    if state.zoom.is_live() {
        let label = format!("{:.1}x", state.zoom.zoom);
        return Some(
            Group::new()
                .key("status-live-zoom-badge")
                .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
                .cursor(CursorShape::Pointer)
                .on_hover(Msg::Hover(HoverTarget::ActiveToolBadge))
                .on_leave(Msg::Leave(HoverTarget::ActiveToolBadge))
                .on_click(Msg::ActiveToolBadgeClick)
                .child(
                    Row::new()
                        .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
                        .gap(3.logical_px())
                        .align(Align::Center)
                        .child(
                            Rect::new()
                                .size(Size::new(4.logical_px(), 4.logical_px()))
                                .fill(GREEN)
                                .corner_radius(0.logical_px()),
                        )
                        .child(
                            Text::new(label)
                                .font(font_ui_semibold())
                                .font_size(9.5.logical_px())
                                .color(if hovered { TEXT } else { GREEN }),
                        ),
                )
                .into(),
        );
    }

    let ws = state.active_workspace_key();
    if state.annotate.pinned_visible && state.annotate.has_strokes(&ws) {
        return Some(
            Group::new()
                .key("status-pinned-annotate-badge")
                .size(Size::new(14.logical_px(), BAR_HEIGHT.logical_px()))
                .cursor(CursorShape::Pointer)
                .on_hover(Msg::Hover(HoverTarget::ActiveToolBadge))
                .on_leave(Msg::Leave(HoverTarget::ActiveToolBadge))
                .on_click(Msg::ActiveToolBadgeClick)
                .child(
                    Text::new("\u{f040}")
                        .font(font_ui())
                        .font_size(10.5.logical_px())
                        .bounds_mode(TextBoundsMode::Optical)
                        .color(if hovered { TEXT } else { MAUVE })
                        .position(Position::relative_to_parent(
                            Percent::new(50.0),
                            Percent::new(50.0),
                        ))
                        .anchor(ElementAnchor::Center),
                )
                .into(),
        );
    }

    None
}

/// Formats a detailed multi-line hover tooltip for the privacy indicator column.
#[must_use]
pub fn privacy_tooltip(state: &State) -> String {
    let p = &state.privacy;
    if p.active.is_empty() {
        return String::from("Privacy · All sensors idle");
    }
    let mut lines = Vec::with_capacity(p.active.len() + 1);
    lines.push(String::from("Active Hardware Access"));
    for entry in &p.active {
        let kind = match entry.kind {
            PrivacyAccessKind::Camera => "Cam",
            PrivacyAccessKind::Microphone => "Mic",
            PrivacyAccessKind::AudioCapture => "Audio",
            PrivacyAccessKind::ScreenShare => "Screen",
            PrivacyAccessKind::Location => "Loc",
        };
        let app = if entry.app_name.is_empty() {
            entry.node_name.as_deref().unwrap_or("unknown")
        } else {
            entry.app_name.as_str()
        };
        lines.push(format!("{kind}: {app}"));
    }
    lines.join("\n")
}
