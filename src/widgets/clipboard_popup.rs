//! Minimalist floating Clipboard History + Caffeine toggle popup (`Layer::Overlay`).

use waywi::prelude::*;

use crate::app::clipboard::{ClipboardEntry, ClipboardEntryKind};
use crate::app::state::{HoverTarget, Msg, State};
use crate::theme::{
    BAR_HEIGHT, BLUE, CRUST, MANTLE, MAUVE, NOTIF_CENTER_TOP_MARGIN, OVERLAY0, PEACH, RED,
    SUBTEXT0, SUBTEXT1, SURFACE0, SURFACE1, TEXT, font_ui, font_ui_semibold,
};

const POPUP_WIDTH: f32 = 328.0;
const HEADER_HEIGHT: f32 = 34.0;
const CARD_HEIGHT: f32 = 42.0;
const CARD_GAP: f32 = 4.0;
const LIST_PAD: f32 = 6.0;
const MAX_VISIBLE_CARDS: usize = 7;

pub fn build_clipboard_popup_surface(state: &State) -> Surface<Msg> {
    let ease = state.clipboard.popup_progress();
    let total = state.clipboard.entries.len();
    let max_start = total.saturating_sub(MAX_VISIBLE_CARDS);
    let start = state.clipboard.scroll_index.min(max_start);
    let end = (start + MAX_VISIBLE_CARDS).min(total);
    let visible = &state.clipboard.entries[start..end];

    let body_h = if visible.is_empty() {
        56.0
    } else {
        let n = visible.len() as f32;
        LIST_PAD * 2.0 + n * CARD_HEIGHT + (n - 1.0).max(0.0) * CARD_GAP
    };
    let full_h = HEADER_HEIGHT + 1.0 + body_h;
    let animated_h = (full_h * ease).max(1.0);
    let animated_y = (BAR_HEIGHT - 8.0) + (NOTIF_CENTER_TOP_MARGIN - (BAR_HEIGHT - 8.0)) * ease;

    let fallback_left = (state.bar_width - POPUP_WIDTH - 360.0).max(8.0);
    let left_x = if state.clipboard.popup_left_x > 0.0 {
        state.clipboard.popup_left_x
    } else {
        fallback_left
    }
    .clamp(8.0, (state.bar_width - POPUP_WIDTH - 8.0).max(8.0));

    let mut list_col = Column::new()
        .size(Size::new(Length::Fill, Length::Auto))
        .gap(CARD_GAP.logical_px())
        .padding(Edges::all(LIST_PAD.logical_px()));

    if visible.is_empty() {
        list_col = list_col.child(
            Row::new()
                .size(Size::new(Length::Fill, 44.logical_px()))
                .padding(Edges::symmetric(0.logical_px(), 10.logical_px()))
                .align(Align::Center)
                .child(
                    Text::new("Clipboard is empty")
                        .font(font_ui())
                        .font_size(11.logical_px())
                        .color(OVERLAY0),
                ),
        );
    } else {
        for entry in visible {
            list_col = list_col.child(build_entry_card(state, entry));
        }
    }

    let list_body = Group::new()
        .size(Size::new(Length::Fill, body_h.logical_px()))
        .on_scroll(Msg::ClipboardPopupScroll)
        .child(list_col);

    let card_inner = Group::new()
        .size(Size::new(POPUP_WIDTH.logical_px(), full_h.logical_px()))
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(CRUST)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), SURFACE1),
        )
        .child(
            Column::new()
                .size(Size::new(Length::Fill, Length::Auto))
                .child(build_header(state))
                .child(
                    Rect::new()
                        .size(Size::new(Length::Fill, 1.logical_px()))
                        .fill(SURFACE1),
                )
                .child(list_body),
        );

    let animated_box = Clip::new(card_inner)
        .key("clipboard-popup-box")
        .position(Position::absolute(
            left_x.logical_px(),
            animated_y.logical_px(),
        ))
        .size(Size::new(POPUP_WIDTH.logical_px(), animated_h.logical_px()))
        .on_pointer_down(|_| Msg::Noop);

    let backdrop = Group::new()
        .key("clipboard-popup-backdrop")
        .size(Size::fill())
        .on_pointer_down(|_| Msg::CloseClipboardPopup)
        .child(animated_box);

    Surface::floating("bar-clipboard-popup", Output::Primary)
        .layer(Layer::Overlay)
        .ignore_exclusive_zone()
        .anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT)
        .size(Size::fill())
        .input_mode(InputMode::Full)
        .keyboard_interactivity(KeyboardInteractivity::OnDemand)
        .background(Color::TRANSPARENT)
        .on_close(Msg::CloseClipboardPopup)
        .on_key(|ev| (ev.key == Key::Escape).then_some(Msg::CloseClipboardPopup))
        .child(backdrop)
}

fn build_header(state: &State) -> Element<Msg> {
    let count = state.clipboard.entries.len();
    let caff_on = state.caffeine_active;
    let caff_hov = state.hovered == HoverTarget::ClipboardCaffeineToggle;
    let clear_hov = state.hovered == HoverTarget::ClipboardClear;

    let caff_btn = Group::new()
        .key("clipboard-caffeine-btn")
        .size(Size::new(Length::Auto, 22.logical_px()))
        .cursor(CursorShape::Pointer)
        .on_hover(Msg::Hover(HoverTarget::ClipboardCaffeineToggle))
        .on_leave(Msg::Leave(HoverTarget::ClipboardCaffeineToggle))
        .on_click(Msg::ToggleCaffeine)
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(if caff_on {
                    SURFACE1
                } else if caff_hov {
                    SURFACE0
                } else {
                    MANTLE
                })
                .corner_radius(0.logical_px())
                .border(1.logical_px(), if caff_on || caff_hov { PEACH } else { SURFACE1 }),
        )
        .child(
            Row::new()
                .size(Size::new(Length::Auto, 22.logical_px()))
                .padding(Edges::symmetric(0.logical_px(), 7.logical_px()))
                .gap(4.logical_px())
                .align(Align::Center)
                .child(
                    Text::new("\u{f0f4}")
                        .font(font_ui())
                        .font_size(10.logical_px())
                        .bounds_mode(TextBoundsMode::Optical)
                        .color(if caff_on || caff_hov { PEACH } else { SUBTEXT1 }),
                )
                .child(
                    Text::new("Awake")
                        .font(font_ui_semibold())
                        .font_size(9.5.logical_px())
                        .color(if caff_on || caff_hov { PEACH } else { SUBTEXT1 }),
                ),
        );

    let clear_btn = Group::new()
        .key("clipboard-clear-btn")
        .size(Size::new(Length::Auto, 22.logical_px()))
        .cursor(CursorShape::Pointer)
        .on_hover(Msg::Hover(HoverTarget::ClipboardClear))
        .on_leave(Msg::Leave(HoverTarget::ClipboardClear))
        .on_click(Msg::ClipboardClearUnpinned)
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(if clear_hov { SURFACE0 } else { MANTLE })
                .corner_radius(0.logical_px())
                .border(1.logical_px(), if clear_hov { RED } else { SURFACE1 }),
        )
        .child(
            Row::new()
                .size(Size::new(Length::Auto, 22.logical_px()))
                .padding(Edges::symmetric(0.logical_px(), 7.logical_px()))
                .align(Align::Center)
                .child(
                    Text::new("Clear")
                        .font(font_ui_semibold())
                        .font_size(9.5.logical_px())
                        .color(if clear_hov { RED } else { SUBTEXT1 }),
                ),
        );

    Group::new()
        .size(Size::new(Length::Fill, HEADER_HEIGHT.logical_px()))
        .child(
            Row::new()
                .size(Size::auto())
                .gap(6.logical_px())
                .align(Align::Center)
                .position(Position::relative_to_parent(
                    Percent::new(0.0),
                    Percent::new(50.0),
                ))
                .anchor(ElementAnchor::CenterLeft)
                .padding(Edges::new(0.logical_px(), 0.logical_px(), 0.logical_px(), 10.logical_px()))
                .child(
                    Text::new("\u{f0ea}")
                        .font(font_ui())
                        .font_size(11.logical_px())
                        .bounds_mode(TextBoundsMode::Optical)
                        .color(MAUVE),
                )
                .child(
                    Text::new(format!("Clipboard ({count})"))
                        .font(font_ui_semibold())
                        .font_size(11.logical_px())
                        .color(MAUVE),
                ),
        )
        .child(
            Row::new()
                .size(Size::auto())
                .gap(6.logical_px())
                .align(Align::Center)
                .position(Position::relative_to_parent(
                    Percent::new(100.0),
                    Percent::new(50.0),
                ))
                .anchor(ElementAnchor::CenterRight)
                .padding(Edges::new(0.logical_px(), 10.logical_px(), 0.logical_px(), 0.logical_px()))
                .child(caff_btn)
                .child(clear_btn),
        )
        .into()
}

fn build_entry_card(state: &State, entry: &ClipboardEntry) -> Element<Msg> {
    let id = entry.id;
    let can_open = entry.can_open();
    let hovered = state.hovered == HoverTarget::ClipboardItem(id)
        || state.hovered == HoverTarget::ClipboardOpen(id)
        || state.hovered == HoverTarget::ClipboardPin(id)
        || state.hovered == HoverTarget::ClipboardDelete(id);
    let open_hov = state.hovered == HoverTarget::ClipboardOpen(id);
    let pin_hov = state.hovered == HoverTarget::ClipboardPin(id);
    let del_hov = state.hovered == HoverTarget::ClipboardDelete(id);

    let bg = if hovered { SURFACE0 } else { MANTLE };
    let border = if entry.pinned {
        PEACH
    } else if hovered {
        MAUVE
    } else {
        SURFACE1
    };

    let lead: Element<Msg> = match &entry.kind {
        ClipboardEntryKind::Image {
            thumb: Some(asset), ..
        } => Group::new()
            .size(Size::new(36.logical_px(), 28.logical_px()))
            .child(
                Rect::new()
                    .size(Size::fill())
                    .fill(CRUST)
                    .border(1.logical_px(), SURFACE1),
            )
            .child(
                Image::new(*asset)
                    .position(Position::absolute(1.logical_px(), 1.logical_px()))
                    .size(Size::new(34.logical_px(), 26.logical_px()))
                    .fit(ImageFit::Contain),
            )
            .into(),
        ClipboardEntryKind::Image { thumb: None, .. } => Group::new()
            .size(Size::new(36.logical_px(), 28.logical_px()))
            .child(Rect::new().size(Size::fill()).fill(CRUST).border(1.logical_px(), SURFACE1))
            .child(
                Text::new("\u{f03e}")
                    .font(font_ui())
                    .font_size(12.logical_px())
                    .bounds_mode(TextBoundsMode::Optical)
                    .color(SUBTEXT0)
                    .position(Position::relative_to_parent(Percent::new(50.0), Percent::new(50.0)))
                    .anchor(ElementAnchor::Center),
            )
            .into(),
        ClipboardEntryKind::Text {
            hex_color: Some(col),
            ..
        } => Rect::new()
            .size(Size::new(20.logical_px(), 20.logical_px()))
            .fill(*col)
            .corner_radius(0.logical_px())
            .border(1.logical_px(), SURFACE1)
            .into(),
        ClipboardEntryKind::Text {
            hex_color: None, ..
        } => Text::new("\u{f15c}")
            .font(font_ui())
            .font_size(11.logical_px())
            .bounds_mode(TextBoundsMode::Optical)
            .color(if hovered { MAUVE } else { OVERLAY0 })
            .into(),
    };

    let preview = entry.preview_line();
    let max_chars = if can_open { 26 } else { 30 };
    let truncated: String = if preview.chars().count() > max_chars {
        let mut s: String = preview.chars().take(max_chars - 1).collect();
        s.push('…');
        s
    } else {
        preview
    };

    let mut right_actions = Row::new()
        .size(Size::auto())
        .gap(4.logical_px())
        .align(Align::Center)
        .position(Position::relative_to_parent(
            Percent::new(100.0),
            Percent::new(50.0),
        ))
        .anchor(ElementAnchor::CenterRight)
        .padding(Edges::new(
            0.logical_px(),
            6.logical_px(),
            0.logical_px(),
            0.logical_px(),
        ));

    if can_open {
        let open_btn = Group::new()
            .key(("clip-open", id))
            .size(Size::new(22.logical_px(), 22.logical_px()))
            .cursor(CursorShape::Pointer)
            .on_hover(Msg::Hover(HoverTarget::ClipboardOpen(id)))
            .on_leave(Msg::Leave(HoverTarget::ClipboardOpen(id)))
            .on_click(Msg::ClipboardOpenEntry(id))
            .child(
                Rect::new()
                    .size(Size::fill())
                    .fill(if open_hov { SURFACE1 } else { CRUST })
                    .corner_radius(0.logical_px())
                    .border(1.logical_px(), if open_hov { BLUE } else { SURFACE1 }),
            )
            .child(
                Text::new("\u{f08e}")
                    .font(font_ui())
                    .font_size(9.5.logical_px())
                    .bounds_mode(TextBoundsMode::Optical)
                    .color(if open_hov { BLUE } else { SUBTEXT1 })
                    .position(Position::relative_to_parent(
                        Percent::new(50.0),
                        Percent::new(50.0),
                    ))
                    .anchor(ElementAnchor::Center),
            );
        right_actions = right_actions.child(open_btn);
    }

    let pin_btn = Group::new()
        .key(("clip-pin", id))
        .size(Size::new(22.logical_px(), 22.logical_px()))
        .cursor(CursorShape::Pointer)
        .on_hover(Msg::Hover(HoverTarget::ClipboardPin(id)))
        .on_leave(Msg::Leave(HoverTarget::ClipboardPin(id)))
        .on_click(Msg::ClipboardPinEntry(id))
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(if pin_hov { SURFACE1 } else { CRUST })
                .corner_radius(0.logical_px())
                .border(1.logical_px(), if entry.pinned || pin_hov { PEACH } else { SURFACE1 }),
        )
        .child(
            Text::new("\u{f08d}")
                .font(font_ui())
                .font_size(9.5.logical_px())
                .bounds_mode(TextBoundsMode::Optical)
                .color(if entry.pinned || pin_hov { PEACH } else { OVERLAY0 })
                .position(Position::relative_to_parent(Percent::new(50.0), Percent::new(50.0)))
                .anchor(ElementAnchor::Center),
        );

    let del_btn = Group::new()
        .key(("clip-del", id))
        .size(Size::new(22.logical_px(), 22.logical_px()))
        .cursor(CursorShape::Pointer)
        .on_hover(Msg::Hover(HoverTarget::ClipboardDelete(id)))
        .on_leave(Msg::Leave(HoverTarget::ClipboardDelete(id)))
        .on_click(Msg::ClipboardDeleteEntry(id))
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(if del_hov { SURFACE1 } else { CRUST })
                .corner_radius(0.logical_px())
                .border(1.logical_px(), if del_hov { RED } else { SURFACE1 }),
        )
        .child(
            Text::new("\u{f1f8}")
                .font(font_ui())
                .font_size(9.5.logical_px())
                .bounds_mode(TextBoundsMode::Optical)
                .color(if del_hov { RED } else { OVERLAY0 })
                .position(Position::relative_to_parent(Percent::new(50.0), Percent::new(50.0)))
                .anchor(ElementAnchor::Center),
        );

    right_actions = right_actions.child(pin_btn).child(del_btn);

    let text_w = if can_open { 174.0 } else { 208.0 };
    let text_box = Group::new()
        .size(Size::new(text_w.logical_px(), 16.logical_px()))
        .child(
            Text::new(truncated)
                .font(font_ui())
                .font_size(10.5.logical_px())
                .color(if hovered { TEXT } else { SUBTEXT1 })
                .position(Position::relative_to_parent(
                    Percent::new(0.0),
                    Percent::new(50.0),
                ))
                .anchor(ElementAnchor::CenterLeft),
        );

    Group::new()
        .key(("clip-card", id))
        .size(Size::new(Length::Fill, CARD_HEIGHT.logical_px()))
        .cursor(CursorShape::Pointer)
        .on_hover(Msg::Hover(HoverTarget::ClipboardItem(id)))
        .on_leave(Msg::Leave(HoverTarget::ClipboardItem(id)))
        .on_click(Msg::ClipboardCopyEntry(id))
        .on_scroll(Msg::ClipboardPopupScroll)
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(bg)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), border),
        )
        .child(
            Row::new()
                .size(Size::auto())
                .gap(8.logical_px())
                .align(Align::Center)
                .position(Position::relative_to_parent(Percent::new(0.0), Percent::new(50.0)))
                .anchor(ElementAnchor::CenterLeft)
                .padding(Edges::new(0.logical_px(), 0.logical_px(), 0.logical_px(), 8.logical_px()))
                .child(lead)
                .child(text_box),
        )
        .child(right_actions)
        .into()
}
