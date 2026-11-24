//! Unified Hyprland workspace strip with layout-coded active accent, fullscreen dot,
//! special scratchpad indicator, and rich hover tooltips.

use waywi::prelude::*;

use crate::app::state::{HoverTarget, Msg, State};
use crate::theme::{
    BAR_HEIGHT, BLUE, GREEN, MANTLE, MAUVE, OVERLAY0, PEACH, SUBTEXT0, SURFACE1, TEAL, TEXT,
    font_ui, font_ui_bold,
};
use crate::widgets::text::marquee_slice;

const ACTIVE_TRI_W: f32 = 7.0;
const ACTIVE_TRI_H: f32 = 3.5;
const FULLSCREEN_DOT_SIZE: f32 = 2.5;

/// Maps Hyprland v0.57+ workspace tiling layout to a harmonious Catppuccin Mocha accent:
/// - `Dwindle` / `Unknown` → Mauve
/// - `Master` → Blue
/// - `Scrolling` → Teal
/// - `Monocle` → Peach
/// - `Custom(_)` → Green
#[must_use]
pub const fn layout_accent_color(layout: &HyprlandLayout) -> Color {
    match layout {
        HyprlandLayout::Dwindle | HyprlandLayout::Unknown => MAUVE,
        HyprlandLayout::Master => BLUE,
        HyprlandLayout::Scrolling => TEAL,
        HyprlandLayout::Monocle => PEACH,
        HyprlandLayout::Custom(_) => GREEN,
    }
}

pub fn resolve_active_ws_id(state: &State) -> i32 {
    let active_ws = state
        .workspaces
        .iter()
        .find(|w| w.focused && !w.is_special() && w.id.get() > 0)
        .or_else(|| {
            state
                .workspaces
                .iter()
                .find(|w| !w.is_special() && w.id.get() == state.active_ws_id)
        });
    active_ws.map_or(state.active_ws_id.max(1), |w| w.id.get())
}

#[must_use]
pub fn named_workspaces(state: &State) -> Vec<&WorkspaceInfo> {
    let mut list: Vec<&WorkspaceInfo> = state
        .workspaces
        .iter()
        .filter(|w| w.is_named())
        .collect();
    list.sort_by(|a, b| a.name.cmp(&b.name));
    list
}

#[must_use]
pub fn special_workspaces(state: &State) -> Vec<&WorkspaceInfo> {
    let mut list: Vec<&WorkspaceInfo> = state
        .workspaces
        .iter()
        .filter(|w| {
            w.is_special()
                && (w.windows > 0 || state.active_special.as_deref() == Some(w.name.as_str()))
        })
        .collect();
    list.sort_by(|a, b| a.name.cmp(&b.name));
    list
}

#[must_use]
fn named_display_label(ws: &WorkspaceInfo) -> &str {
    ws.name
        .strip_prefix("name:")
        .unwrap_or(ws.name.as_str())
        .trim()
}

/// Formats a concise multi-line tooltip for a normal, renamed, named, or special workspace.
#[must_use]
pub fn format_workspace_tooltip(ws: Option<&WorkspaceInfo>, fallback_id: i32) -> String {
    let Some(ws) = ws else {
        return format!("Workspace {fallback_id}");
    };

    let title = if let Some(sp) = ws.special_name() {
        if sp.is_empty() {
            String::from("Special")
        } else {
            format!("Special: {sp}")
        }
    } else if ws.is_named() {
        let label = named_display_label(ws);
        if label.is_empty() {
            String::from("Workspace")
        } else {
            format!("Workspace {label}")
        }
    } else {
        let id = ws.id.get();
        let trimmed_name = ws.name.trim();
        if !trimmed_name.is_empty() && trimmed_name != id.to_string() {
            format!("Workspace {id} ({trimmed_name})")
        } else {
            format!("Workspace {id}")
        }
    };

    let header = if ws.layout.is_known() {
        format!("{title} · {}", ws.layout.as_str())
    } else {
        title
    };

    let mut flags = Vec::new();
    if ws.has_fullscreen {
        flags.push("fullscreen");
    }
    if ws.is_persistent {
        flags.push("persistent");
    }
    let flags_suffix = if flags.is_empty() {
        String::new()
    } else {
        format!(" ({})", flags.join(", "))
    };

    let mut lines = vec![header, format!("Windows: {}{flags_suffix}", ws.windows)];
    let last = ws.last_window_title.trim();
    if !last.is_empty() {
        lines.push(format!("Last: {}", marquee_slice(last, 42, 0)));
    }
    lines.join("\n")
}

/// Resolves the tooltip text for whichever workspace target is currently hovered.
#[must_use]
pub fn hovered_workspace_tooltip(state: &State) -> Option<String> {
    match state.hovered {
        HoverTarget::ActiveWs => {
            let active_id = resolve_active_ws_id(state);
            let ws = state
                .workspaces
                .iter()
                .find(|w| !w.is_special() && !w.is_named() && w.id.get() == active_id);
            Some(format_workspace_tooltip(ws, active_id))
        }
        HoverTarget::InactiveWs(id) => {
            let ws = state
                .workspaces
                .iter()
                .find(|w| !w.is_special() && !w.is_named() && w.id.get() == id);
            Some(format_workspace_tooltip(ws, id))
        }
        HoverTarget::NamedWs(idx) => {
            let named = named_workspaces(state);
            let ws = named.get(idx).copied()?;
            Some(format_workspace_tooltip(Some(ws), 0))
        }
        HoverTarget::SpecialWs(idx) => {
            let specials = special_workspaces(state);
            let ws = specials.get(idx).copied()?;
            Some(format_workspace_tooltip(Some(ws), 0))
        }
        _ => None,
    }
}

pub fn build_workspaces(state: &State) -> Element<Msg> {
    let named_focused = state.workspaces.iter().any(|w| w.is_named() && w.focused);
    let active_id = resolve_active_ws_id(state);
    let mut ids: Vec<i32> = state
        .workspaces
        .iter()
        .filter(|w| !w.is_special() && !w.is_named())
        .map(|w| w.id.get())
        .filter(|&id| id > 0)
        .collect();
    if !named_focused && active_id > 0 && !ids.contains(&active_id) {
        ids.push(active_id);
    }
    ids.sort_unstable();
    ids.dedup();
    if ids.is_empty() && named_workspaces(state).is_empty() {
        ids.push(1);
    }

    let mut strip = Row::new()
        .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
        .padding(Edges::symmetric(0.logical_px(), 2.logical_px()))
        .align(Align::Center);

    for id in ids {
        let ws = state
            .workspaces
            .iter()
            .find(|w| !w.is_special() && !w.is_named() && w.id.get() == id);
        let is_active = !named_focused && id == active_id;
        let accent = ws.map_or(MAUVE, |w| layout_accent_color(&w.layout));
        let has_fullscreen = ws.is_some_and(|w| w.has_fullscreen);

        let target = if is_active {
            HoverTarget::ActiveWs
        } else {
            HoverTarget::InactiveWs(id)
        };
        let is_hovered = state.hovered == target;
        let fg = if is_active {
            accent
        } else if is_hovered {
            TEXT
        } else {
            SUBTEXT0
        };
        let font = if is_active {
            font_ui_bold()
        } else {
            font_ui()
        };

        let label = Row::new()
            .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
            .padding(Edges::new(
                0.logical_px(),
                5.logical_px(),
                if is_active { 1.5 } else { 0.0 }.logical_px(),
                5.logical_px(),
            ))
            .align(Align::Center)
            .child(
                Text::new(format!("{id}"))
                    .font(font)
                    .font_size(12.logical_px())
                    .line_height(1.15)
                    .color(fg),
            );

        let mut item = Group::new()
            .key(("ws-item", id))
            .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
            .on_hover(Msg::Hover(target))
            .on_leave(Msg::Leave(target))
            .on_pointer_down(move |ev| match ev.button {
                Some(MouseButton::Left) => Msg::SwitchWorkspace(WorkspaceId::new(id)),
                Some(MouseButton::Middle) => Msg::SwitchWorkspacePrevious,
                _ => Msg::Noop,
            })
            .child(label);

        if has_fullscreen {
            item = item.child(fullscreen_dot(if is_active { accent } else { PEACH }));
        }
        if is_active {
            item = item.child(active_triangle(("ws-active-tri", id), accent));
        }

        strip = strip.child(item);
    }

    let named = named_workspaces(state);
    for (idx, ws) in named.iter().enumerate() {
        let is_active = ws.focused;
        let accent = layout_accent_color(&ws.layout);
        let target = HoverTarget::NamedWs(idx);
        let is_hovered = state.hovered == target;
        let fg = if is_active {
            accent
        } else if is_hovered {
            TEXT
        } else {
            SUBTEXT0
        };
        let font = if is_active {
            font_ui_bold()
        } else {
            font_ui()
        };
        let display = marquee_slice(named_display_label(ws), 8, 0);
        let spec = ws.target_spec();

        let mut item = Group::new()
            .key(("ws-named", idx))
            .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
            .on_hover(Msg::Hover(target))
            .on_leave(Msg::Leave(target))
            .on_pointer_down(move |ev| match ev.button {
                Some(MouseButton::Left) => Msg::SwitchWorkspaceNamed(spec.clone()),
                Some(MouseButton::Middle) => Msg::SwitchWorkspacePrevious,
                _ => Msg::Noop,
            })
            .child(
                Row::new()
                    .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
                    .padding(Edges::new(
                        0.logical_px(),
                        5.logical_px(),
                        if is_active { 1.5 } else { 0.0 }.logical_px(),
                        5.logical_px(),
                    ))
                    .align(Align::Center)
                    .child(
                        Text::new(display)
                            .font(font)
                            .font_size(11.logical_px())
                            .line_height(1.15)
                            .color(fg),
                    ),
            );

        if ws.has_fullscreen {
            item = item.child(fullscreen_dot(if is_active { accent } else { PEACH }));
        }
        if is_active {
            item = item.child(active_triangle(("ws-named-tri", idx as i32), accent));
        }
        strip = strip.child(item);
    }

    let specials = special_workspaces(state);
    if !specials.is_empty() {
        strip = strip.child(
            Rect::new()
                .size(Size::new(1.logical_px(), 10.logical_px()))
                .fill(SURFACE1),
        );
        for (idx, ws) in specials.iter().enumerate() {
            let is_shown = state.active_special.as_deref() == Some(ws.name.as_str());
            let accent = layout_accent_color(&ws.layout);
            let target = HoverTarget::SpecialWs(idx);
            let is_hovered = state.hovered == target;
            let fg = if is_shown {
                accent
            } else if is_hovered {
                TEXT
            } else {
                OVERLAY0
            };
            let special_name = ws.name.clone();

            let mut item = Group::new()
                .key(("ws-special", idx))
                .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
                .on_hover(Msg::Hover(target))
                .on_leave(Msg::Leave(target))
                .on_pointer_down(move |ev| match ev.button {
                    Some(MouseButton::Left) => Msg::ToggleSpecialWorkspace(special_name.clone()),
                    Some(MouseButton::Middle) => Msg::SwitchWorkspacePrevious,
                    _ => Msg::Noop,
                })
                .child(
                    Row::new()
                        .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
                        .padding(Edges::new(
                            0.logical_px(),
                            5.logical_px(),
                            if is_shown { 1.5 } else { 0.0 }.logical_px(),
                            5.logical_px(),
                        ))
                        .align(Align::Center)
                        .child(
                            Text::new("\u{f005}")
                                .font(font_ui())
                                .font_size(10.logical_px())
                                .bounds_mode(TextBoundsMode::Optical)
                                .color(fg),
                        ),
                );

            if ws.has_fullscreen {
                item = item.child(fullscreen_dot(if is_shown { accent } else { PEACH }));
            }
            if is_shown {
                item = item.child(active_triangle(("ws-special-tri", idx as i32), accent));
            }
            strip = strip.child(item);
        }
    }

    Group::new()
        .key("ws-strip")
        .size(Size::new(Length::Auto, BAR_HEIGHT.logical_px()))
        .on_scroll(Msg::WorkspaceScroll)
        .on_pointer_down(|ev| {
            if ev.button == Some(MouseButton::Middle) {
                Msg::SwitchWorkspacePrevious
            } else {
                Msg::Noop
            }
        })
        .on_pointer_move(|ev| {
            let left = (ev.position.x.get() - ev.local_position.x.get()).max(8.0);
            Msg::HoverAnchorX(left)
        })
        .child(Rect::new().size(Size::fill()).fill(MANTLE))
        .child(strip)
        .into()
}

fn fullscreen_dot(color: Color) -> Element<Msg> {
    Rect::new()
        .size(Size::new(
            FULLSCREEN_DOT_SIZE.logical_px(),
            FULLSCREEN_DOT_SIZE.logical_px(),
        ))
        .corner_radius((FULLSCREEN_DOT_SIZE * 0.5).logical_px())
        .fill(color)
        .position(Position::RelativeToParent {
            x: Percent::new(100.0),
            y: Percent::new(0.0),
            offset_x: (-2.5).logical_px(),
            offset_y: 3.5.logical_px(),
        })
        .anchor(ElementAnchor::TopRight)
        .into()
}

fn active_triangle(key: (&'static str, i32), color: Color) -> Element<Msg> {
    Canvas::new(
        Size::new(ACTIVE_TRI_W.logical_px(), ACTIVE_TRI_H.logical_px()),
        move |p| {
            let mut pb = PathBuilder::new();
            pb.move_to(0.0.logical_px(), ACTIVE_TRI_H.logical_px())
                .line_to((ACTIVE_TRI_W * 0.5).logical_px(), 0.0.logical_px())
                .line_to(ACTIVE_TRI_W.logical_px(), ACTIVE_TRI_H.logical_px())
                .close();
            p.fill_path(&pb, color);
        },
    )
    .cache_key((key, color))
    .key(key)
    .position(Position::relative_to_parent(
        Percent::new(50.0),
        Percent::new(100.0),
    ))
    .anchor(ElementAnchor::BottomCenter)
    .into()
}
