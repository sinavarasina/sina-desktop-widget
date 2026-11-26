//! Quick status/settings popup overlay with live clock, uptime/user summary, layered
//! Yuki Nagato witch sprite (`21_witch_profile_right`), expandable Settings drawer,
//! and minimalist icon-only power strip.

pub mod settings;

use waywi::prelude::*;

use crate::app::quick_menu::{PowerAction, YukiGaze};
use crate::app::state::{HoverTarget, Msg, State};
use crate::theme::{
    CRUST, MANTLE, MAUVE, OVERLAY0, POPUP_TOP_MARGIN, RED, SUBTEXT1, SURFACE0, SURFACE1, TEXT,
    font_ui, font_ui_bold, font_ui_semibold,
};
use crate::widgets::text::marquee_slice;

use settings::{SETTINGS_PANEL_HEIGHT, build_settings_panel};

pub const QUICK_MENU_WIDTH: f32 = 396.0;
const QUICK_CARD_HEIGHT: f32 = 192.0;
const QUICK_PAD: f32 = 10.0;
const QUICK_CARD_GAP: f32 = 6.0;
const POWER_CARD_HEIGHT: f32 = 50.0;
const POWER_BTN_SIZE: f32 = 32.0;

const SPRITE_FRAME_W: f32 = 168.0;
const SPRITE_FRAME_H: f32 = 172.0;

pub fn build_quick_menu_surface(state: &State) -> Surface<Msg> {
    let ease = state.quick_menu.popup_progress();

    let main_card = build_main_card(state);
    let lower_panel: Option<(Element<Msg>, f32)> = if state.quick_menu.settings_open {
        Some((build_settings_panel(state), SETTINGS_PANEL_HEIGHT))
    } else if state.quick_menu.power_menu_open {
        Some((build_power_card(state), POWER_CARD_HEIGHT))
    } else {
        None
    };

    let full_height = match lower_panel {
        Some((_, extra_h)) => QUICK_CARD_HEIGHT + QUICK_CARD_GAP + extra_h,
        None => QUICK_CARD_HEIGHT,
    };
    let animated_h = (full_height * ease).max(1.0);
    let slide_offset = (1.0 - ease) * -14.0;
    let animated_y = (POPUP_TOP_MARGIN + slide_offset).max(0.0);
    let left_x = (state.bar_width - QUICK_MENU_WIDTH - 8.0).max(8.0);

    let mut col = Column::new()
        .size(Size::new(
            QUICK_MENU_WIDTH.logical_px(),
            full_height.logical_px(),
        ))
        .gap(QUICK_CARD_GAP.logical_px())
        .child(main_card);

    if let Some((panel, _)) = lower_panel {
        col = col.child(panel);
    }

    let animated_box = Clip::new(col)
        .key("quick-menu-box")
        .position(Position::absolute(
            left_x.logical_px(),
            animated_y.logical_px(),
        ))
        .size(Size::new(
            QUICK_MENU_WIDTH.logical_px(),
            animated_h.logical_px(),
        ))
        .on_pointer_down(|_| Msg::Noop);

    let backdrop = Group::new()
        .key("quick-menu-backdrop")
        .size(Size::fill())
        .on_pointer_down(|_| Msg::CloseQuickMenu)
        .child(animated_box);

    let input_focused = state.quick_menu.settings_open && state.settings_ui.wallpaper_focused;
    let mut surface = Surface::floating("bar-quick-menu", Output::Primary)
        .layer(Layer::Overlay)
        .ignore_exclusive_zone()
        .anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT)
        .size(Size::fill())
        .input_mode(InputMode::Full)
        .keyboard_interactivity(if input_focused {
            KeyboardInteractivity::Exclusive
        } else {
            KeyboardInteractivity::OnDemand
        })
        .background(Color::TRANSPARENT)
        .on_close(Msg::CloseQuickMenu)
        .on_key(|ev| Some(Msg::SettingsKey(ev)))
        .on_ime(|ev| Some(Msg::SettingsIme(ev)));

    if input_focused {
        let cursor_rect = Bounds::new(
            (left_x + 120.0 + state.settings_ui.wallpaper_cursor_x.min(176.0)).logical_px(),
            (animated_y + QUICK_CARD_HEIGHT + QUICK_CARD_GAP + 76.0).logical_px(),
            2.0.logical_px(),
            22.0.logical_px(),
        );
        let ime_cfg = ImeConfig::enabled()
            .from_text_input(&state.settings_ui.wallpaper_input)
            .cursor_rect(cursor_rect);
        surface = surface.ime(ime_cfg);
    }

    surface.child(backdrop)
}

fn build_main_card(state: &State) -> Element<Msg> {
    let time_str = state.now.format_hms(state.time_format());
    let date_str = state.now.format_long_date();
    let uptime_str = state
        .sysinfo
        .uptime
        .as_ref()
        .map_or_else(|| String::from("up --"), format_compact_uptime);
    let status_line = format!("{uptime_str} · logged as {}", state.quick_menu.username);

    let left_w = QUICK_MENU_WIDTH - QUICK_PAD * 2.0 - SPRITE_FRAME_W - 10.0;
    let inner_h = QUICK_CARD_HEIGHT - QUICK_PAD * 2.0;

    let settings_hovered = state.hovered == HoverTarget::QuickSettingsToggle;
    let settings_active = state.quick_menu.settings_open;
    let settings_bg = if settings_active {
        SURFACE1
    } else if settings_hovered {
        SURFACE0
    } else {
        MANTLE
    };
    let settings_border = if settings_active || settings_hovered {
        MAUVE
    } else {
        SURFACE1
    };
    let settings_fg = if settings_active || settings_hovered {
        MAUVE
    } else {
        TEXT
    };

    let settings_btn = Group::new()
        .key("quick-settings-toggle")
        .size(Size::new(34.logical_px(), 34.logical_px()))
        .cursor(CursorShape::Pointer)
        .on_hover(Msg::Hover(HoverTarget::QuickSettingsToggle))
        .on_leave(Msg::Leave(HoverTarget::QuickSettingsToggle))
        .on_click(Msg::QuickMenuToggleSettings)
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(settings_bg)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), settings_border),
        )
        .child(
            Text::new("\u{f013}")
                .font(font_ui())
                .font_size(14.logical_px())
                .bounds_mode(TextBoundsMode::Optical)
                .color(settings_fg)
                .position(Position::relative_to_parent(
                    Percent::new(50.0),
                    Percent::new(50.0),
                ))
                .anchor(ElementAnchor::Center),
        );

    let settings_hint = if settings_active {
        (state.settings_ui.active_section.title(), MAUVE)
    } else if settings_hovered {
        ("Settings", SUBTEXT1)
    } else {
        ("", OVERLAY0)
    };

    let settings_row = Row::new()
        .size(Size::new(left_w.logical_px(), 34.logical_px()))
        .gap(8.logical_px())
        .align(Align::Center)
        .child(settings_btn)
        .child(
            Text::new(settings_hint.0)
                .font(font_ui_semibold())
                .font_size(10.logical_px())
                .color(settings_hint.1),
        );

    let power_hovered = state.hovered == HoverTarget::QuickPowerToggle;
    let power_active = state.quick_menu.power_menu_open;
    let power_bg = if power_active {
        SURFACE1
    } else if power_hovered {
        SURFACE0
    } else {
        MANTLE
    };
    let power_border = if power_active || power_hovered {
        RED
    } else {
        SURFACE1
    };
    let power_fg = if power_active || power_hovered {
        RED
    } else {
        TEXT
    };

    let power_btn = Group::new()
        .key("quick-power-toggle")
        .size(Size::new(34.logical_px(), 34.logical_px()))
        .cursor(CursorShape::Pointer)
        .on_hover(Msg::Hover(HoverTarget::QuickPowerToggle))
        .on_leave(Msg::Leave(HoverTarget::QuickPowerToggle))
        .on_click(Msg::QuickMenuTogglePower)
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(power_bg)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), power_border),
        )
        .child(
            Text::new("\u{f011}")
                .font(font_ui())
                .font_size(14.logical_px())
                .bounds_mode(TextBoundsMode::Optical)
                .color(power_fg)
                .position(Position::relative_to_parent(
                    Percent::new(50.0),
                    Percent::new(50.0),
                ))
                .anchor(ElementAnchor::Center),
        );

    // Right of the Power icon: when a countdown is active, show `<action> in <N>s`
    // with a compact `Cancel` pill button directly underneath; otherwise show hover hint / error.
    let status_slot: Element<Msg> = if let Some(pending) = state.quick_menu.pending_power {
        let cancel_hovered = state.hovered == HoverTarget::QuickPowerCancel;
        let cancel_btn = Group::new()
            .key("quick-power-cancel")
            .size(Size::auto())
            .cursor(CursorShape::Pointer)
            .on_hover(Msg::Hover(HoverTarget::QuickPowerCancel))
            .on_leave(Msg::Leave(HoverTarget::QuickPowerCancel))
            .on_click(Msg::QuickMenuCancelPower)
            .child(
                Rect::new()
                    .size(Size::fill())
                    .fill(if cancel_hovered { SURFACE0 } else { MANTLE })
                    .corner_radius(0.logical_px())
                    .border(1.logical_px(), RED),
            )
            .child(
                Row::new()
                    .padding(Edges::symmetric(2.logical_px(), 8.logical_px()))
                    .align(Align::Center)
                    .child(
                        Text::new("Cancel")
                            .font(font_ui_semibold())
                            .font_size(9.5.logical_px())
                            .color(RED),
                    ),
            );

        Column::new()
            .size(Size::auto())
            .gap(3.logical_px())
            .align(Align::Start)
            .child(
                Text::new(format!(
                    "{} in {}s",
                    pending.action.label(),
                    pending.seconds_left
                ))
                .font(font_ui_semibold())
                .font_size(10.logical_px())
                .color(pending.action.accent()),
            )
            .child(cancel_btn)
            .into()
    } else {
        let (hint_text, hint_col) = if let Some(ref err) = state.quick_menu.power_error {
            (marquee_slice(err, 22, 0), RED)
        } else if let HoverTarget::QuickPowerAction(action) = state.hovered {
            let support = action.support(&state.quick_menu.logind.power_capabilities);
            let suffix = if support.needs_auth() { " · auth" } else { "" };
            (format!("{}{suffix}", action.label()), action.accent())
        } else if state.hovered == HoverTarget::QuickPowerToggle {
            (String::from("Power options"), SUBTEXT1)
        } else {
            (String::new(), OVERLAY0)
        };

        Text::new(hint_text)
            .font(font_ui_semibold())
            .font_size(10.logical_px())
            .color(hint_col)
            .into()
    };

    let power_row = Row::new()
        .size(Size::new(left_w.logical_px(), 36.logical_px()))
        .gap(8.logical_px())
        .align(Align::Center)
        .child(power_btn)
        .child(status_slot);

    let bottom_stack = Column::new()
        .size(Size::new(left_w.logical_px(), 76.logical_px()))
        .gap(6.logical_px())
        .align(Align::Start)
        .position(Position::relative_to_parent(
            Percent::new(0.0),
            Percent::new(100.0),
        ))
        .anchor(ElementAnchor::BottomLeft)
        .child(settings_row)
        .child(power_row);

    let top_info = Column::new()
        .size(Size::new(left_w.logical_px(), Length::Auto))
        .gap(3.logical_px())
        .child(
            Text::new(time_str)
                .font(font_ui_bold())
                .font_size(24.logical_px())
                .color(TEXT),
        )
        .child(
            Text::new(date_str)
                .font(font_ui_semibold())
                .font_size(11.5.logical_px())
                .color(MAUVE),
        )
        .child(
            Text::new(marquee_slice(&status_line, 28, 0))
                .font(font_ui())
                .font_size(9.5.logical_px())
                .color(SUBTEXT1),
        );

    let left_col = Group::new()
        .size(Size::new(left_w.logical_px(), inner_h.logical_px()))
        .child(top_info)
        .child(bottom_stack);

    let sprite_box = build_sprite_box(state);

    let content = Row::new()
        .size(Size::fill())
        .padding(Edges::all(QUICK_PAD.logical_px()))
        .gap(10.logical_px())
        .align(Align::Center)
        .child(left_col)
        .child(sprite_box);

    Group::new()
        .key("quick-main-card")
        .size(Size::new(
            QUICK_MENU_WIDTH.logical_px(),
            QUICK_CARD_HEIGHT.logical_px(),
        ))
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(CRUST)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), SURFACE1),
        )
        .child(content)
        .into()
}

fn build_sprite_box(state: &State) -> Element<Msg> {
    let hovered = state.hovered == HoverTarget::QuickSprite;
    let border_col = if hovered { MAUVE } else { SURFACE0 };

    let mut frame = Group::new()
        .key("quick-sprite-box")
        .size(Size::new(
            SPRITE_FRAME_W.logical_px(),
            SPRITE_FRAME_H.logical_px(),
        ))
        .cursor(CursorShape::Pointer)
        .on_hover(Msg::Hover(HoverTarget::QuickSprite))
        .on_leave(Msg::Leave(HoverTarget::QuickSprite))
        .on_click(Msg::QuickMenuSpriteClick)
        .on_scroll(Msg::QuickMenuSpriteScroll)
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(MANTLE)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), border_col),
        );

    if let Some(ref assets) = state.quick_menu.sprite_assets {
        let pose_idx = state.quick_menu.sprite_pose_idx;
        let spec = state.quick_menu.current_pose_spec();
        let scale = (SPRITE_FRAME_W / spec.base_w)
            .min(SPRITE_FRAME_H / spec.base_h)
            .min(0.50);
        let base_w = spec.base_w * scale;
        let base_h = spec.base_h * scale;

        let glance = hovered || state.quick_menu.gaze == YukiGaze::SideGlanceViewer;
        let eye_sheet = if glance {
            &assets.eye_glance
        } else {
            &assets.eye_forward
        };
        let eye_key = if glance {
            ("yuki-eye-glance", pose_idx)
        } else {
            ("yuki-eye-forward", pose_idx)
        };
        let mouth_sheet = if glance {
            assets.mouth_alt.as_ref().unwrap_or(&assets.mouth)
        } else {
            &assets.mouth
        };
        let mouth_variant = u8::from(glance && assets.mouth_alt.is_some());

        let mouth_el = if state.quick_menu.mouth_reacting {
            Sprite::new(mouth_sheet)
                .key(("yuki-mouth-react", pose_idx, mouth_variant))
                .playing("react")
                .on_animation_end(Msg::QuickMenuSpriteReactEnd)
                .filter(ImageFilter::Bicubic)
                .position(Position::absolute(
                    (spec.mouth_x * scale).logical_px(),
                    (spec.mouth_y * scale).logical_px(),
                ))
                .size(Size::new(
                    (spec.mouth_w * scale).logical_px(),
                    (spec.mouth_h * scale).logical_px(),
                ))
        } else {
            Sprite::new(mouth_sheet)
                .key(("yuki-mouth-idle", pose_idx, mouth_variant))
                .frame(0)
                .filter(ImageFilter::Bicubic)
                .position(Position::absolute(
                    (spec.mouth_x * scale).logical_px(),
                    (spec.mouth_y * scale).logical_px(),
                ))
                .size(Size::new(
                    (spec.mouth_w * scale).logical_px(),
                    (spec.mouth_h * scale).logical_px(),
                ))
        };

        let layered = Group::new()
            .key(("yuki-pose-layer", pose_idx))
            .size(Size::new(base_w.logical_px(), base_h.logical_px()))
            .position(Position::relative_to_parent(
                Percent::new(50.0),
                Percent::new(100.0),
            ))
            .anchor(ElementAnchor::BottomCenter)
            .child(
                Image::new(assets.base)
                    .size(Size::new(base_w.logical_px(), base_h.logical_px()))
                    .filter(ImageFilter::Bicubic),
            )
            .child(
                Sprite::new(eye_sheet)
                    .key(eye_key)
                    .playing("blink")
                    .filter(ImageFilter::Bicubic)
                    .position(Position::absolute(
                        (spec.eye_x * scale).logical_px(),
                        (spec.eye_y * scale).logical_px(),
                    ))
                    .size(Size::new(
                        (spec.eye_w * scale).logical_px(),
                        (spec.eye_h * scale).logical_px(),
                    )),
            )
            .child(mouth_el);

        frame = frame.child(
            Clip::new(layered).size(Size::new(
                SPRITE_FRAME_W.logical_px(),
                SPRITE_FRAME_H.logical_px(),
            )),
        );
    }

    frame.into()
}

/// Minimalist icon-only power strip dynamically populated from `LogindPowerCapabilities`.
fn build_power_card(state: &State) -> Element<Msg> {
    let actions = state.quick_menu.supported_power_actions();
    let mut row = Row::new()
        .size(Size::auto())
        .gap(6.logical_px())
        .align(Align::Center)
        .position(Position::relative_to_parent(
            Percent::new(50.0),
            Percent::new(50.0),
        ))
        .anchor(ElementAnchor::Center);

    for (idx, (action, _support)) in actions.into_iter().enumerate() {
        row = row.child(power_icon_btn(state, action, idx));
    }

    Group::new()
        .key("quick-power-card")
        .size(Size::new(
            QUICK_MENU_WIDTH.logical_px(),
            POWER_CARD_HEIGHT.logical_px(),
        ))
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(CRUST)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), SURFACE1),
        )
        .child(row)
        .into()
}

fn power_icon_btn(state: &State, action: PowerAction, idx: usize) -> Element<Msg> {
    let target = HoverTarget::QuickPowerAction(action);
    let hovered = state.hovered == target;
    let accent = action.accent();
    let bg = if hovered { SURFACE0 } else { MANTLE };
    let border = if hovered { accent } else { SURFACE1 };
    let fg = if hovered { accent } else { SUBTEXT1 };

    Group::new()
        .key(("quick-power-btn", idx))
        .size(Size::new(
            POWER_BTN_SIZE.logical_px(),
            POWER_BTN_SIZE.logical_px(),
        ))
        .cursor(CursorShape::Pointer)
        .on_hover(Msg::Hover(target))
        .on_leave(Msg::Leave(target))
        .on_click(Msg::QuickMenuPowerAction(action))
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(bg)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), border),
        )
        .child(
            Text::new(action.icon())
                .font(font_ui())
                .font_size(13.logical_px())
                .bounds_mode(TextBoundsMode::Optical)
                .color(fg)
                .position(Position::relative_to_parent(
                    Percent::new(50.0),
                    Percent::new(50.0),
                ))
                .anchor(ElementAnchor::Center),
        )
        .into()
}

fn format_compact_uptime(snap: &waywi::system::UptimeSnapshot) -> String {
    let total_mins = snap.uptime.as_secs() / 60;
    let days = total_mins / (60 * 24);
    let hours = (total_mins / 60) % 24;
    let mins = total_mins % 60;
    if days > 0 {
        format!("up {days}d {hours}h {mins}m")
    } else if hours > 0 {
        format!("up {hours}h {mins}m")
    } else {
        format!("up {mins}m")
    }
}
