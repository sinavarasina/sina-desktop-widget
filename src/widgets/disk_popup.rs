//! UDisks2 disk management popup overlay surface (Mount, Unmount, LUKS Unlock/Lock, Eject,
//! Power Off, and Polkit-backed error feedback). Core OS system partitions (`/`, `/boot`, EFI)
//! are hidden by default and rendered in a separate card when toggled on.

use waywi::prelude::*;

use crate::app::disk::{DiskMenuAction, DiskUiState};
use crate::app::state::{HoverTarget, Msg, State};
use crate::theme::{
    BLUE, CRUST, GREEN, MANTLE, MAUVE, OVERLAY0, POPUP_TOP_MARGIN, RED, SUBTEXT1, SURFACE0,
    SURFACE1, TEXT, font_ui, font_ui_bold, font_ui_semibold,
};
use crate::widgets::text::marquee_slice;

pub const DISK_POPUP_WIDTH: f32 = 340.0;
const DISK_HEADER_HEIGHT: f32 = 32.0;
const DISK_SYS_HEADER_HEIGHT: f32 = 22.0;
const DISK_UNLOCK_BAR_HEIGHT: f32 = 32.0;
const DISK_ROW_HEIGHT: f32 = 38.0;
const DISK_ROW_GAP: f32 = 4.0;
const DISK_CARD_GAP: f32 = 6.0;
const DISK_VISIBLE_ROWS: usize = 6;
const DISK_PAD: f32 = 10.0;

pub fn build_disk_popup_surface(state: &State) -> Surface<Msg> {
    let p = state.disk_ui.popup_progress.clamp(0.0, 1.0);
    let ease = 1.0 - (1.0 - p).powi(3);

    let user_blocks = DiskUiState::user_blocks(&state.udisks);
    let sys_blocks = DiskUiState::system_blocks(&state.udisks);
    let show_sys_card = state.disk_ui.show_system && !sys_blocks.is_empty();

    let (main_card, main_h) = build_main_card(state, &user_blocks, sys_blocks.len());
    let (sys_card, sys_h) = if show_sys_card {
        let (card, h) = build_system_card(state, &sys_blocks);
        (Some(card), h)
    } else {
        (None, 0.0)
    };

    let full_height = if sys_card.is_some() {
        main_h + DISK_CARD_GAP + sys_h
    } else {
        main_h
    };
    let animated_h = (full_height * ease).max(1.0);
    let slide_offset = (1.0 - ease) * -14.0;
    let animated_y = (POPUP_TOP_MARGIN + slide_offset).max(0.0);

    let max_left = (state.bar_width - DISK_POPUP_WIDTH - 8.0).max(8.0);
    let left_x = state.disk_ui.popup_left_x.clamp(8.0, max_left);

    let mut popup_col = Column::new()
        .size(Size::new(
            DISK_POPUP_WIDTH.logical_px(),
            full_height.logical_px(),
        ))
        .gap(DISK_CARD_GAP.logical_px())
        .child(main_card);

    if let Some(card) = sys_card {
        popup_col = popup_col.child(card);
    }

    let animated_box = Clip::new(popup_col)
        .key("disk-popup-box")
        .position(Position::absolute(
            left_x.logical_px(),
            animated_y.logical_px(),
        ))
        .size(Size::new(
            DISK_POPUP_WIDTH.logical_px(),
            animated_h.logical_px(),
        ))
        .on_pointer_down(|_| Msg::Noop);

    let backdrop = Group::new()
        .key("disk-popup-backdrop")
        .size(Size::fill())
        .on_pointer_down(|_| Msg::CloseDiskPopup)
        .child(animated_box);

    let mut surface = Surface::floating("bar-disk-popup", Output::Primary)
        .layer(Layer::Overlay)
        .ignore_exclusive_zone()
        .anchor(Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right)
        .size(Size::fill())
        .input_mode(InputMode::Full)
        .keyboard_interactivity(if state.disk_ui.unlock_target.is_some() {
            KeyboardInteractivity::Exclusive
        } else {
            KeyboardInteractivity::OnDemand
        })
        .background(Color::TRANSPARENT)
        .on_key(|ev| Some(Msg::DiskUnlockKey(ev)))
        .on_ime(|ev| Some(Msg::DiskUnlockIme(ev)));

    if state.disk_ui.unlock_target.is_some() {
        let cursor_rect = GeoRect::new(
            (left_x + DISK_PAD + 24.0 + state.disk_ui.unlock_cursor_x).logical_px(),
            (animated_y + DISK_PAD + DISK_HEADER_HEIGHT + 6.0).logical_px(),
            2.0.logical_px(),
            DISK_UNLOCK_BAR_HEIGHT.logical_px(),
        );
        let ime_cfg = ImeConfig::enabled()
            .from_text_input(&state.disk_ui.unlock_input)
            .cursor_rect(cursor_rect);
        surface = surface.ime(ime_cfg);
    }

    surface.child(backdrop)
}

fn build_main_card(
    state: &State,
    user_blocks: &[&UDisksBlockDevice],
    sys_count: usize,
) -> (Element<Msg>, f32) {
    let visible_count = user_blocks.len().clamp(1, DISK_VISIBLE_ROWS);
    let body_height = if user_blocks.is_empty() {
        40.0
    } else {
        (visible_count as f32) * DISK_ROW_HEIGHT
            + (visible_count.saturating_sub(1) as f32) * DISK_ROW_GAP
    };
    let unlock_extra = if state.disk_ui.unlock_target.is_some() {
        DISK_UNLOCK_BAR_HEIGHT + 6.0
    } else {
        0.0
    };
    let err_extra = if state.disk_ui.last_error.is_some() {
        24.0
    } else {
        0.0
    };
    let card_h = DISK_PAD * 2.0 + DISK_HEADER_HEIGHT + 6.0 + unlock_extra + err_extra + body_height;

    let header = build_header(state, user_blocks, sys_count);

    let mut list_col = Column::new()
        .size(Size::new(Length::Fill, body_height.logical_px()))
        .gap(DISK_ROW_GAP.logical_px());

    if user_blocks.is_empty() {
        list_col = list_col.child(
            Row::new()
                .key("disk-empty")
                .size(Size::new(Length::Fill, 36.logical_px()))
                .padding(Edges::symmetric(0.logical_px(), 8.logical_px()))
                .align(Align::Center)
                .child(
                    Text::new("No removable or data drives")
                        .font(font_ui())
                        .font_size(11.logical_px())
                        .color(OVERLAY0),
                ),
        );
    } else {
        let max_start = user_blocks.len().saturating_sub(DISK_VISIBLE_ROWS);
        let start = state.disk_ui.scroll_index.min(max_start);
        for (offset, block) in user_blocks
            .iter()
            .skip(start)
            .take(DISK_VISIBLE_ROWS)
            .enumerate()
        {
            let idx = start + offset;
            let row_el = if state.disk_ui.menu_path.as_deref() == Some(block.object_path.as_str()) {
                build_device_menu(state, block, idx)
            } else {
                build_device_row(state, block, idx)
            };
            list_col = list_col.child(row_el);
        }
    }

    let mut inner_col = Column::new()
        .size(Size::fill())
        .padding(Edges::all(DISK_PAD.logical_px()))
        .gap(6.logical_px())
        .child(header);

    if state.disk_ui.unlock_target.is_some() {
        inner_col = inner_col.child(build_unlock_bar(state));
    }

    if let Some(ref err) = state.disk_ui.last_error {
        inner_col = inner_col.child(
            Row::new()
                .key("disk-err-banner")
                .size(Size::new(Length::Fill, 18.logical_px()))
                .align(Align::Center)
                .child(
                    Text::new(marquee_slice(err, 42, state.disk_ui.marquee_tick))
                        .font(font_ui())
                        .font_size(10.logical_px())
                        .color(RED),
                ),
        );
    }

    inner_col = inner_col.child(
        Clip::new(list_col)
            .size(Size::new(Length::Fill, body_height.logical_px()))
            .on_scroll(Msg::DiskPopupScroll),
    );

    let card = Group::new()
        .key("disk-main-card")
        .size(Size::new(
            DISK_POPUP_WIDTH.logical_px(),
            card_h.logical_px(),
        ))
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(CRUST)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), SURFACE1),
        )
        .child(inner_col)
        .into();

    (card, card_h)
}

/// Separate card for core OS system partitions (`/`, `/boot`, EFI) so they stay visually isolated
/// from user/removable drives, matching the two-card layout of the volume mixer popup.
fn build_system_card(state: &State, sys_blocks: &[&UDisksBlockDevice]) -> (Element<Msg>, f32) {
    let count = sys_blocks.len();
    let list_h = (count as f32) * DISK_ROW_HEIGHT + (count.saturating_sub(1) as f32) * DISK_ROW_GAP;
    let card_h = DISK_PAD * 2.0 + DISK_SYS_HEADER_HEIGHT + 6.0 + list_h;

    let mut list_col = Column::new()
        .size(Size::new(Length::Fill, list_h.logical_px()))
        .gap(DISK_ROW_GAP.logical_px());

    for (i, block) in sys_blocks.iter().enumerate() {
        let hover_idx = 1000 + i;
        let row_el = if state.disk_ui.menu_path.as_deref() == Some(block.object_path.as_str()) {
            build_device_menu(state, block, hover_idx)
        } else {
            build_device_row(state, block, hover_idx)
        };
        list_col = list_col.child(row_el);
    }

    let inner = Column::new()
        .size(Size::fill())
        .padding(Edges::all(DISK_PAD.logical_px()))
        .gap(6.logical_px())
        .child(
            Row::new()
                .size(Size::new(Length::Fill, DISK_SYS_HEADER_HEIGHT.logical_px()))
                .gap(6.logical_px())
                .align(Align::Center)
                .child(
                    Text::new("\u{f071}")
                        .font(font_ui())
                        .font_size(10.5.logical_px())
                        .color(RED),
                )
                .child(
                    Text::new(format!("System Partitions ({count})"))
                        .font(font_ui_semibold())
                        .font_size(10.5.logical_px())
                        .color(SUBTEXT1),
                ),
        )
        .child(list_col);

    let card = Group::new()
        .key("disk-system-card")
        .size(Size::new(
            DISK_POPUP_WIDTH.logical_px(),
            card_h.logical_px(),
        ))
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(CRUST)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), SURFACE1),
        )
        .child(inner)
        .into();

    (card, card_h)
}

fn build_header(
    state: &State,
    user_blocks: &[&UDisksBlockDevice],
    sys_count: usize,
) -> Element<Msg> {
    let mounted_cnt = user_blocks.iter().filter(|b| b.is_mounted()).count();

    let refresh_btn = header_btn(
        "disk-refresh-btn",
        "\u{f021} Refresh",
        HoverTarget::DiskRefresh,
        state.hovered == HoverTarget::DiskRefresh,
        false,
        MAUVE,
        Msg::DiskRefresh,
    );

    let mut row = Row::new()
        .size(Size::new(Length::Fill, DISK_HEADER_HEIGHT.logical_px()))
        .gap(8.logical_px())
        .align(Align::Center)
        .child(
            Text::new(format!(
                "\u{f0a0} Disks ({mounted_cnt}/{})",
                user_blocks.len()
            ))
            .font(font_ui_bold())
            .font_size(12.logical_px())
            .color(MAUVE),
        );

    if sys_count > 0 {
        row = row.child(header_btn(
            "disk-sys-toggle-btn",
            if state.disk_ui.show_system {
                "System On"
            } else {
                "System"
            },
            HoverTarget::DiskSystemToggle,
            state.hovered == HoverTarget::DiskSystemToggle,
            state.disk_ui.show_system,
            RED,
            Msg::DiskToggleSystem,
        ));
    }

    row.child(refresh_btn).into()
}

fn build_unlock_bar(state: &State) -> Element<Msg> {
    let masked = state.disk_ui.masked_unlock_text();
    let placeholder = format!("LUKS passphrase for {}...", state.disk_ui.unlock_label);
    let is_empty = state.disk_ui.unlock_input.display_text().is_empty();

    let unlock_btn = menu_btn(
        "disk-unlock-submit",
        "Unlock",
        HoverTarget::DiskUnlockSubmit,
        state.hovered == HoverTarget::DiskUnlockSubmit,
        GREEN,
        Msg::DiskUnlockSubmit,
    );
    let cancel_btn = menu_btn(
        "disk-unlock-cancel",
        "Cancel",
        HoverTarget::DiskUnlockCancel,
        state.hovered == HoverTarget::DiskUnlockCancel,
        RED,
        Msg::DiskUnlockCancel,
    );

    Group::new()
        .key("disk-unlock-bar")
        .size(Size::new(Length::Fill, DISK_UNLOCK_BAR_HEIGHT.logical_px()))
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(MANTLE)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), MAUVE),
        )
        .child(
            Row::new()
                .size(Size::fill())
                .padding(Edges::symmetric(0.logical_px(), 8.logical_px()))
                .gap(6.logical_px())
                .align(Align::Center)
                .child(
                    Text::new("\u{f023}")
                        .font(font_ui())
                        .font_size(11.logical_px())
                        .color(MAUVE),
                )
                .child(
                    Group::new()
                        .size(Size::new(Length::Fill, 18.logical_px()))
                        .child(
                            Text::new(if is_empty {
                                marquee_slice(&placeholder, 24, state.disk_ui.marquee_tick)
                            } else {
                                masked
                            })
                            .font(font_ui())
                            .font_size(10.5.logical_px())
                            .color(if is_empty { OVERLAY0 } else { TEXT })
                            .position(Position::relative_to_parent(
                                Percent::new(0.0),
                                Percent::new(50.0),
                            ))
                            .anchor(ElementAnchor::CenterLeft),
                        )
                        .child(
                            Rect::new()
                                .position(Position::absolute(
                                    state.disk_ui.unlock_cursor_x.logical_px(),
                                    3.logical_px(),
                                ))
                                .size(Size::px(1.5.logical_px(), 12.logical_px()))
                                .fill(MAUVE),
                        ),
                )
                .child(unlock_btn)
                .child(cancel_btn),
        )
        .into()
}

fn header_btn(
    key: &'static str,
    label: &'static str,
    target: HoverTarget,
    hovered: bool,
    active: bool,
    active_col: Color,
    on_click: Msg,
) -> Element<Msg> {
    let bg = if active {
        SURFACE1
    } else if hovered {
        SURFACE0
    } else {
        MANTLE
    };
    let fg = if active { active_col } else { SUBTEXT1 };
    let border_col = if active { active_col } else { SURFACE1 };

    Group::new()
        .key(key)
        .size(Size::auto())
        .on_hover(Msg::Hover(target))
        .on_leave(Msg::Leave(target))
        .on_click(on_click)
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(bg)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), border_col),
        )
        .child(
            Row::new()
                .padding(Edges::symmetric(3.logical_px(), 8.logical_px()))
                .align(Align::Center)
                .child(
                    Text::new(label)
                        .font(font_ui_semibold())
                        .font_size(10.5.logical_px())
                        .color(fg),
                ),
        )
        .into()
}

fn menu_btn(
    key: &'static str,
    label: &'static str,
    target: HoverTarget,
    hovered: bool,
    accent: Color,
    on_click: Msg,
) -> Element<Msg> {
    let bg = if hovered { SURFACE0 } else { MANTLE };

    Group::new()
        .key(key)
        .size(Size::auto())
        .on_hover(Msg::Hover(target))
        .on_leave(Msg::Leave(target))
        .on_click(on_click)
        .on_scroll(Msg::DiskPopupScroll)
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(bg)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), accent),
        )
        .child(
            Row::new()
                .padding(Edges::symmetric(2.logical_px(), 6.logical_px()))
                .align(Align::Center)
                .child(
                    Text::new(label)
                        .font(font_ui_semibold())
                        .font_size(10.logical_px())
                        .color(accent),
                ),
        )
        .into()
}

fn resolve_pending<'a>(state: &'a State, block: &UDisksBlockDevice) -> Option<&'a str> {
    if let Some(op) = state.disk_ui.pending_ops.get(&block.object_path) {
        return Some(*op);
    }
    if let Some(ref backing) = block.crypto_backing_device
        && let Some(op) = state.disk_ui.pending_ops.get(backing)
    {
        return Some(*op);
    }
    if let Some(drive) = state.udisks.drive_for_block(block)
        && let Some(op) = state.disk_ui.pending_ops.get(&drive.object_path)
    {
        return Some(*op);
    }
    None
}

fn build_device_row(state: &State, block: &UDisksBlockDevice, idx: usize) -> Element<Msg> {
    let hovered = state.hovered == HoverTarget::DiskDevice(idx);
    let pending_op = resolve_pending(state, block);
    let drive = state.udisks.drive_for_block(block);
    let is_removable = drive.is_some_and(|d| d.removable || d.is_usb());

    let bg = if hovered { SURFACE0 } else { MANTLE };
    let border_col = if pending_op.is_some() {
        MAUVE
    } else if block.is_mounted() {
        BLUE
    } else if is_removable {
        SURFACE1
    } else {
        SURFACE0
    };

    let icon = if block.is_encrypted && !block.is_unlocked() {
        "\u{f023}"
    } else if drive.is_some_and(UDisksDrive::is_usb) {
        "\u{f287}"
    } else if is_removable {
        "\u{f07c}"
    } else {
        "\u{f0a0}"
    };

    let dev_short = block
        .device
        .strip_prefix("/dev/")
        .unwrap_or(block.device.as_str());
    let size_gb = block.size_bytes as f64 / 1_000_000_000.0;
    let label = block.display_label();
    let title_raw = if label != block.device && label != block.preferred_device {
        format!("{label} ({dev_short} · {size_gb:.1}G)")
    } else if let Some(d) = drive
        && !d.display_name().is_empty()
    {
        format!("{} ({dev_short} · {size_gb:.1}G)", d.display_name())
    } else {
        format!("{dev_short} ({size_gb:.1}G {})", block.id_type)
    };
    let name_text = marquee_slice(&title_raw, 20, state.disk_ui.marquee_tick);

    let (status_text, status_col) = if let Some(op) = pending_op {
        (format!("{op}{}", state.disk_ui.pending_dots()), MAUVE)
    } else if block.is_mounted() {
        let mp = block.mount_points.first().map_or("/", String::as_str);
        let mp_short = if mp == "/" {
            String::from("mounted /")
        } else {
            let tail = mp.rsplit('/').next().unwrap_or(mp);
            format!("@/{}", marquee_slice(tail, 10, state.disk_ui.marquee_tick))
        };
        (mp_short, GREEN)
    } else if block.is_encrypted && !block.is_unlocked() {
        (String::from("locked"), MAUVE)
    } else {
        (String::from("mount"), SUBTEXT1)
    };

    let path = block.object_path.clone();
    Group::new()
        .key(("disk-dev", idx))
        .size(Size::new(Length::Fill, DISK_ROW_HEIGHT.logical_px()))
        .on_hover(Msg::Hover(HoverTarget::DiskDevice(idx)))
        .on_leave(Msg::Leave(HoverTarget::DiskDevice(idx)))
        .on_pointer_down(move |ev| Msg::DiskDevicePointerDown {
            path: path.clone(),
            ev,
        })
        .on_scroll(Msg::DiskPopupScroll)
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(bg)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), border_col),
        )
        .child(
            Row::new()
                .size(Size::fill())
                .padding(Edges::symmetric(0.logical_px(), 10.logical_px()))
                .gap(8.logical_px())
                .align(Align::Center)
                .child(
                    Text::new(icon)
                        .font(font_ui())
                        .font_size(12.logical_px())
                        .color(if block.is_mounted() || is_removable {
                            MAUVE
                        } else {
                            OVERLAY0
                        }),
                )
                .child(
                    Clip::new(
                        Text::new(name_text)
                            .font(font_ui_semibold())
                            .font_size(11.logical_px())
                            .color(TEXT),
                    )
                    .size(Size::px(184.logical_px(), 16.logical_px())),
                )
                .child(
                    Text::new(status_text)
                        .font(font_ui_semibold())
                        .font_size(10.logical_px())
                        .color(status_col),
                ),
        )
        .into()
}

fn build_device_menu(state: &State, block: &UDisksBlockDevice, idx: usize) -> Element<Msg> {
    let drive = state.udisks.drive_for_block(block);
    let can_eject = drive.is_some_and(|d| d.ejectable || d.removable);
    let can_power_off = drive.is_some_and(|d| d.can_power_off);
    let has_crypto = block.is_encrypted || block.crypto_backing_device.is_some();

    let dev_short = block
        .device
        .strip_prefix("/dev/")
        .unwrap_or(block.device.as_str());
    let name_text = marquee_slice(dev_short, 8, state.disk_ui.marquee_tick);

    let toggle_path = block.object_path.clone();
    let border_col = if block.is_mounted() { BLUE } else { SURFACE1 };

    let mut row = Row::new()
        .size(Size::fill())
        .padding(Edges::symmetric(0.logical_px(), 8.logical_px()))
        .gap(4.logical_px())
        .align(Align::Center)
        .child(
            Clip::new(
                Text::new(name_text)
                    .font(font_ui_semibold())
                    .font_size(10.5.logical_px())
                    .color(TEXT),
            )
            .size(Size::px(58.logical_px(), 16.logical_px())),
        );

    if block.has_filesystem {
        let (label, accent, action) = if block.is_mounted() {
            ("Unmount", GREEN, DiskMenuAction::Unmount { force: false })
        } else {
            ("Mount", GREEN, DiskMenuAction::Mount)
        };
        row = row.child(menu_btn(
            "disk-menu-mount",
            label,
            HoverTarget::DiskMenuMount,
            state.hovered == HoverTarget::DiskMenuMount,
            accent,
            Msg::DiskDeviceMenuAction {
                path: block.object_path.clone(),
                action,
            },
        ));
        if block.is_mounted() {
            row = row.child(menu_btn(
                "disk-menu-force-unmount",
                "Force",
                HoverTarget::DiskMenuForceUnmount,
                state.hovered == HoverTarget::DiskMenuForceUnmount,
                RED,
                Msg::DiskDeviceMenuAction {
                    path: block.object_path.clone(),
                    action: DiskMenuAction::Unmount { force: true },
                },
            ));
        }
    }

    if has_crypto {
        let is_locked = block.is_encrypted && !block.is_unlocked();
        let (label, action) = if is_locked {
            ("Unlock", DiskMenuAction::UnlockPrompt)
        } else {
            ("Lock", DiskMenuAction::Lock)
        };
        if !block.is_mounted() {
            row = row.child(menu_btn(
                "disk-menu-crypt",
                label,
                HoverTarget::DiskMenuCrypt,
                state.hovered == HoverTarget::DiskMenuCrypt,
                MAUVE,
                Msg::DiskDeviceMenuAction {
                    path: block.object_path.clone(),
                    action,
                },
            ));
        }
    }

    if can_eject {
        row = row.child(menu_btn(
            "disk-menu-eject",
            "Eject",
            HoverTarget::DiskMenuEject,
            state.hovered == HoverTarget::DiskMenuEject,
            MAUVE,
            Msg::DiskDeviceMenuAction {
                path: block.object_path.clone(),
                action: DiskMenuAction::Eject,
            },
        ));
    }

    if can_power_off {
        row = row.child(menu_btn(
            "disk-menu-poweroff",
            "PowerOff",
            HoverTarget::DiskMenuPowerOff,
            state.hovered == HoverTarget::DiskMenuPowerOff,
            RED,
            Msg::DiskDeviceMenuAction {
                path: block.object_path.clone(),
                action: DiskMenuAction::PowerOff,
            },
        ));
    }

    Group::new()
        .key(("disk-dev-menu", idx))
        .size(Size::new(Length::Fill, DISK_ROW_HEIGHT.logical_px()))
        .on_hover(Msg::Hover(HoverTarget::DiskDevice(idx)))
        .on_leave(Msg::Leave(HoverTarget::DiskDevice(idx)))
        .on_pointer_down(move |ev| {
            if ev.button == Some(MouseButton::Right) {
                Msg::DiskDevicePointerDown {
                    path: toggle_path.clone(),
                    ev,
                }
            } else {
                Msg::Noop
            }
        })
        .on_scroll(Msg::DiskPopupScroll)
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(MANTLE)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), border_col),
        )
        .child(row)
        .into()
}
