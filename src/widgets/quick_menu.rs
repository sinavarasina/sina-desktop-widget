//! Quick status/settings popup overlay with live clock, uptime/user summary, layered
//! Yuki Nagato witch sprite (`21_witch_profile_right`), and minimalist icon-only power strip.

use waywi::prelude::*;
use waywi::system::TimeFormat;

use crate::app::quick_menu::{
    PowerAction, SPRITE_BASE_H, SPRITE_BASE_W, SPRITE_EYE_H, SPRITE_EYE_W, SPRITE_EYE_X,
    SPRITE_EYE_Y, SPRITE_MOUTH_H, SPRITE_MOUTH_W, SPRITE_MOUTH_X, SPRITE_MOUTH_Y, YukiGaze,
};
use crate::app::state::{HoverTarget, Msg, State};
use crate::theme::{
    CRUST, MANTLE, MAUVE, OVERLAY0, POPUP_TOP_MARGIN, RED, SUBTEXT1, SURFACE0, SURFACE1, TEXT,
    font_ui, font_ui_bold, font_ui_semibold,
};
use crate::widgets::text::marquee_slice;

pub const QUICK_MENU_WIDTH: f32 = 396.0;
const QUICK_CARD_HEIGHT: f32 = 192.0;
const QUICK_PAD: f32 = 10.0;
const QUICK_CARD_GAP: f32 = 6.0;
const POWER_CARD_HEIGHT: f32 = 50.0;
const POWER_BTN_SIZE: f32 = 32.0;

const SPRITE_SCALE: f32 = 0.50;
const SPRITE_FRAME_W: f32 = 168.0;
const SPRITE_FRAME_H: f32 = 172.0;

pub fn build_quick_menu_surface(state: &State) -> Surface<Msg> {
    let p = state.quick_menu.popup_progress.clamp(0.0, 1.0);
    let ease = 1.0 - (1.0 - p).powi(3);

    let main_card = build_main_card(state);
    let power_card = if state.quick_menu.power_menu_open {
        Some(build_power_card(state))
    } else {
        None
    };

    let full_height = if power_card.is_some() {
        QUICK_CARD_HEIGHT + QUICK_CARD_GAP + POWER_CARD_HEIGHT
    } else {
        QUICK_CARD_HEIGHT
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

    if let Some(card) = power_card {
        col = col.child(card);
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

    Surface::floating("bar-quick-menu", Output::Primary)
        .layer(Layer::Overlay)
        .ignore_exclusive_zone()
        .anchor(Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right)
        .size(Size::fill())
        .input_mode(InputMode::Full)
        .keyboard_interactivity(KeyboardInteractivity::OnDemand)
        .background(Color::TRANSPARENT)
        .on_key(|ev| {
            if ev.key == Key::Escape {
                Some(Msg::CloseQuickMenu)
            } else {
                None
            }
        })
        .child(backdrop)
}

fn build_main_card(state: &State) -> Element<Msg> {
    let time_str = state.now.format_hms(TimeFormat::H24);
    let date_str = state.now.format_long_date();
    let uptime_str = state
        .sysinfo
        .uptime
        .as_ref()
        .map_or_else(|| String::from("up --"), format_compact_uptime);
    let status_line = format!("{uptime_str} · logged as {}", state.quick_menu.username);

    let left_w = QUICK_MENU_WIDTH - QUICK_PAD * 2.0 - SPRITE_FRAME_W - 10.0;
    let inner_h = QUICK_CARD_HEIGHT - QUICK_PAD * 2.0;

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
        .size(Size::px(34.logical_px(), 34.logical_px()))
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
                .optical_bounds(true)
                .color(power_fg)
                .position(Position::relative_to_parent(
                    Percent::new(50.0),
                    Percent::new(50.0),
                ))
                .anchor(ElementAnchor::Center),
        );

    // Right of the Power icon: when a 3-second countdown is active, show `<action> in <N>s`
    // with a compact `Cancel` pill button directly underneath; otherwise show hover hint / error.
    let status_slot: Element<Msg> = if let Some(pending) = state.quick_menu.pending_power {
        let cancel_hovered = state.hovered == HoverTarget::QuickPowerCancel;
        let cancel_btn = Group::new()
            .key("quick-power-cancel")
            .size(Size::auto())
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

    let bottom_bar = Row::new()
        .size(Size::new(left_w.logical_px(), 36.logical_px()))
        .gap(8.logical_px())
        .align(Align::Center)
        .position(Position::relative_to_parent(
            Percent::new(0.0),
            Percent::new(100.0),
        ))
        .anchor(ElementAnchor::BottomLeft)
        .child(power_btn)
        .child(status_slot);

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
        .size(Size::px(left_w.logical_px(), inner_h.logical_px()))
        .child(top_info)
        .child(bottom_bar);

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
        .size(Size::px(
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
        .size(Size::px(
            SPRITE_FRAME_W.logical_px(),
            SPRITE_FRAME_H.logical_px(),
        ))
        .on_hover(Msg::Hover(HoverTarget::QuickSprite))
        .on_leave(Msg::Leave(HoverTarget::QuickSprite))
        .on_click(Msg::QuickMenuSpriteClick)
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(MANTLE)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), border_col),
        );

    if let Some(ref assets) = state.quick_menu.sprite_assets {
        let base_w = SPRITE_BASE_W * SPRITE_SCALE;
        let base_h = SPRITE_BASE_H * SPRITE_SCALE;

        let glance = hovered || state.quick_menu.gaze == YukiGaze::SideGlanceViewer;
        let eye_sheet = if glance {
            &assets.eye_glance
        } else {
            &assets.eye_forward
        };
        let eye_key = if glance {
            "yuki-eye-glance"
        } else {
            "yuki-eye-forward"
        };

        let mouth_el = if state.quick_menu.mouth_reacting {
            Sprite::new(&assets.mouth)
                .key("yuki-mouth-react")
                .playing("react")
                .on_animation_end(Msg::QuickMenuSpriteReactEnd)
                .filter(ImageFilter::Bicubic)
                .position(Position::absolute(
                    (SPRITE_MOUTH_X * SPRITE_SCALE).logical_px(),
                    (SPRITE_MOUTH_Y * SPRITE_SCALE).logical_px(),
                ))
                .size(Size::px(
                    (SPRITE_MOUTH_W * SPRITE_SCALE).logical_px(),
                    (SPRITE_MOUTH_H * SPRITE_SCALE).logical_px(),
                ))
        } else {
            Sprite::new(&assets.mouth)
                .key("yuki-mouth-idle")
                .frame(0)
                .filter(ImageFilter::Bicubic)
                .position(Position::absolute(
                    (SPRITE_MOUTH_X * SPRITE_SCALE).logical_px(),
                    (SPRITE_MOUTH_Y * SPRITE_SCALE).logical_px(),
                ))
                .size(Size::px(
                    (SPRITE_MOUTH_W * SPRITE_SCALE).logical_px(),
                    (SPRITE_MOUTH_H * SPRITE_SCALE).logical_px(),
                ))
        };

        let layered = Group::new()
            .size(Size::px(base_w.logical_px(), base_h.logical_px()))
            .position(Position::relative_to_parent(
                Percent::new(50.0),
                Percent::new(100.0),
            ))
            .anchor(ElementAnchor::BottomCenter)
            .child(
                Image::new(assets.base)
                    .size(Size::px(base_w.logical_px(), base_h.logical_px()))
                    .filter(ImageFilter::Bicubic),
            )
            .child(
                Sprite::new(eye_sheet)
                    .key(eye_key)
                    .playing("blink")
                    .filter(ImageFilter::Bicubic)
                    .position(Position::absolute(
                        (SPRITE_EYE_X * SPRITE_SCALE).logical_px(),
                        (SPRITE_EYE_Y * SPRITE_SCALE).logical_px(),
                    ))
                    .size(Size::px(
                        (SPRITE_EYE_W * SPRITE_SCALE).logical_px(),
                        (SPRITE_EYE_H * SPRITE_SCALE).logical_px(),
                    )),
            )
            .child(mouth_el);

        frame = frame.child(
            Clip::new(layered).size(Size::px(
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
        .size(Size::px(
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
        .size(Size::px(
            POWER_BTN_SIZE.logical_px(),
            POWER_BTN_SIZE.logical_px(),
        ))
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
                .optical_bounds(true)
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
