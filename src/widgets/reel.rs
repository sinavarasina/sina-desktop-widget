//! Clipped pause-and-slide rotating text slot primitive backed by `ReelState`.

use waywi::prelude::*;

use crate::app::reel::ReelState;
use crate::app::state::Msg;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReelAlign {
    Center,
    Left,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReelDirection {
    SlideUp,
    SlideLeft,
}

/// Renders a single clipped line that holds on the active item and smoothly slides to the next
/// item (upward or right-to-left) whenever `reel.is_animating()` is true.
#[allow(clippy::too_many_arguments)]
pub fn build_reel_slot(
    key: &'static str,
    width: f32,
    height: f32,
    items: &[String],
    reel: &ReelState,
    font: Font,
    font_size: f32,
    color: Color,
    max_chars: usize,
    align: ReelAlign,
    direction: ReelDirection,
) -> Element<Msg> {
    build_reel_slot_accented(
        key,
        width,
        height,
        items,
        reel,
        font,
        font_size,
        color,
        None,
        max_chars,
        align,
        direction,
    )
}

/// Like [`build_reel_slot`], optionally coloring the first character of each item with a rotating
/// palette indexed by item position (e.g. `G`entoo, `H`ypr, `v`arasina).
#[allow(clippy::too_many_arguments)]
pub fn build_reel_slot_accented(
    key: &'static str,
    width: f32,
    height: f32,
    items: &[String],
    reel: &ReelState,
    font: Font,
    font_size: f32,
    color: Color,
    first_char_palette: Option<&[Color]>,
    max_chars: usize,
    align: ReelAlign,
    direction: ReelDirection,
) -> Element<Msg> {
    if items.is_empty() {
        return Group::new()
            .key(key)
            .size(Size::new(width.logical_px(), height.logical_px()))
            .into();
    }

    let (x_pct, anchor) = match align {
        ReelAlign::Center => (Percent::new(50.0), ElementAnchor::Center),
        ReelAlign::Left => (Percent::new(0.0), ElementAnchor::CenterLeft),
    };

    let idx = reel.index % items.len();
    let current = truncate_compact(&items[idx], max_chars);
    let cur_accent = first_char_palette
        .filter(|pal| !pal.is_empty())
        .map(|pal| pal[idx % pal.len()]);

    let inner: Element<Msg> = if !reel.animating || items.len() <= 1 {
        Group::new()
            .size(Size::new(width.logical_px(), height.logical_px()))
            .child(build_reel_line(
                &current,
                font,
                font_size,
                color,
                cur_accent,
                1.0,
                Position::relative_to_parent(x_pct, Percent::new(50.0)),
                anchor,
            ))
            .into()
    } else {
        let next_idx = (idx + 1) % items.len();
        let next = truncate_compact(&items[next_idx], max_chars);
        let next_accent = first_char_palette
            .filter(|pal| !pal.is_empty())
            .map(|pal| pal[next_idx % pal.len()]);
        let p = reel.progress.clamp(0.0, 1.0);
        let ease = 1.0 - (1.0 - p).powi(3);
        let (cur_dx, cur_dy, next_dx, next_dy) = match direction {
            ReelDirection::SlideUp => (0.0, -ease * height, 0.0, (1.0 - ease) * height),
            ReelDirection::SlideLeft => (-ease * width, 0.0, (1.0 - ease) * width, 0.0),
        };
        let cur_alpha = (1.0 - ease * 0.55).clamp(0.0, 1.0);
        let next_alpha = (0.45 + ease * 0.55).clamp(0.0, 1.0);

        Group::new()
            .size(Size::new(width.logical_px(), height.logical_px()))
            .child(build_reel_line(
                &current,
                font.clone(),
                font_size,
                color,
                cur_accent,
                cur_alpha,
                Position::RelativeToParent {
                    x: x_pct,
                    y: Percent::new(50.0),
                    offset_x: cur_dx.logical_px(),
                    offset_y: cur_dy.logical_px(),
                },
                anchor,
            ))
            .child(build_reel_line(
                &next,
                font,
                font_size,
                color,
                next_accent,
                next_alpha,
                Position::RelativeToParent {
                    x: x_pct,
                    y: Percent::new(50.0),
                    offset_x: next_dx.logical_px(),
                    offset_y: next_dy.logical_px(),
                },
                anchor,
            ))
            .into()
    };

    Clip::new(inner)
        .key(key)
        .size(Size::new(width.logical_px(), height.logical_px()))
        .into()
}

#[allow(clippy::too_many_arguments)]
fn build_reel_line(
    text: &str,
    font: Font,
    font_size: f32,
    color: Color,
    first_char_accent: Option<Color>,
    alpha: f32,
    position: Position,
    anchor: ElementAnchor,
) -> Element<Msg> {
    if let Some(accent) = first_char_accent {
        let mut chars = text.chars();
        if let Some(first) = chars.next() {
            let rest = chars.as_str();
            let mut row = Row::new()
                .size(Size::auto())
                .align(Align::Center)
                .position(position)
                .anchor(anchor)
                .child(
                    Text::new(first.to_string())
                        .font(font.clone())
                        .font_size(font_size.logical_px())
                        .line_height(1.0)
                        .color(accent.with_opacity(alpha)),
                );
            if !rest.is_empty() {
                row = row.child(
                    Text::new(rest.to_owned())
                        .font(font)
                        .font_size(font_size.logical_px())
                        .line_height(1.0)
                        .color(color.with_opacity(alpha)),
                );
            }
            return row.into();
        }
    }

    Text::new(text.to_owned())
        .font(font)
        .font_size(font_size.logical_px())
        .line_height(1.0)
        .color(color.with_opacity(alpha))
        .position(position)
        .anchor(anchor)
        .into()
}

fn truncate_compact(text: &str, max_chars: usize) -> String {
    let count = text.chars().count();
    if count <= max_chars || max_chars < 2 {
        return text.to_owned();
    }
    let mut out: String = text.chars().take(max_chars - 1).collect();
    out.push('\u{2026}');
    out
}
