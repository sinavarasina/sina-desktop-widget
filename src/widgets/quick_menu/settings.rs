//! Expandable Quick Menu Settings panel with a scrollable vertical section rail,
//! Powerline-notched section banner, and scrollable Catppuccin Mocha sharp controls
//! (toggles, live IME path input, value/action badges, inline dropdowns, and steppers).

use waywi::prelude::*;

use crate::app::config::{
    CONFIG_FILE_NAME, DesktopPoseMode, MemoryProfile, MetricPlacement, QUICK_MENU_POSE_LABELS,
    RendererMode, RightMetricSlot, SettingStepperId, SettingToggleId, SettingsDropdown,
    SettingsSection, WallpaperFit,
};
use crate::app::state::{HoverTarget, Msg, State};
use crate::theme::{
    BLUE, CRUST, GREEN, MANTLE, MAUVE, OVERLAY0, RED, SUBTEXT1, SURFACE0, SURFACE1, TEXT, font_ui,
    font_ui_semibold,
};
use crate::widgets::quick_menu::QUICK_MENU_WIDTH;
use crate::widgets::text::marquee_slice;

pub const SETTINGS_PANEL_HEIGHT: f32 = 278.0;
pub const SETTINGS_RAIL_W: f32 = 50.0;
const SETTINGS_GAP: f32 = 6.0;
const SETTINGS_BODY_W: f32 = QUICK_MENU_WIDTH - SETTINGS_RAIL_W - SETTINGS_GAP;

const RAIL_PAD: f32 = 4.0;
const RAIL_TAB_H: f32 = 48.0;
const RAIL_TAB_GAP: f32 = 4.0;
pub const SETTINGS_RAIL_VIEW_H: f32 = SETTINGS_PANEL_HEIGHT - RAIL_PAD * 2.0;

const BODY_PAD: f32 = 8.0;
const HEADER_H: f32 = 24.0;
const HEADER_GAP: f32 = 8.0;
pub const SETTINGS_LIST_VIEW_H: f32 = SETTINGS_PANEL_HEIGHT - BODY_PAD * 2.0 - HEADER_H - HEADER_GAP;
const LIST_CONTENT_W: f32 = SETTINGS_BODY_W - BODY_PAD * 2.0 - 6.0;

const ROW_H: f32 = 30.0;
const SUBSECTION_H: f32 = 18.0;
const ROW_GAP: f32 = 6.0;
const DROPDOWN_OPT_H: f32 = 24.0;
const DROPDOWN_OPT_GAP: f32 = 3.0;
const DROPDOWN_BOX_W: f32 = 136.0;
const ERROR_LINE_H: f32 = 16.0;

/// Total logical height of all vertical section tabs for rail scroll clamping.
#[must_use]
pub fn settings_rail_content_height() -> f32 {
    let n = SettingsSection::ALL.len();
    (n as f32) * RAIL_TAB_H + (n.saturating_sub(1) as f32) * RAIL_TAB_GAP
}

/// Total logical height of the active section's settings rows, subsections, open dropdowns,
/// and optional error line for exact scroll clamping.
#[must_use]
pub fn settings_list_content_height(state: &State) -> f32 {
    let (base_rows, subsections): (usize, usize) = match state.settings_ui.active_section {
        SettingsSection::General => (8, 2),
        SettingsSection::Audio => (6, 2),
        SettingsSection::Desktop => (10, 2),
        SettingsSection::Bar => (6 + state.config.widgets.right_metrics.len(), 2),
    };

    let item_count = base_rows + subsections;
    let mut total = (base_rows as f32) * ROW_H
        + (subsections as f32) * SUBSECTION_H
        + (item_count.saturating_sub(1) as f32) * ROW_GAP;

    if state.settings_ui.active_section == SettingsSection::Desktop
        && state.settings_ui.wallpaper_error.is_some()
    {
        total += ERROR_LINE_H + ROW_GAP;
    }

    let extra_opts = match (
        state.settings_ui.active_section,
        state.settings_ui.open_dropdown,
    ) {
        (SettingsSection::General, Some(SettingsDropdown::Renderer)) => RendererMode::ALL.len(),
        (SettingsSection::General, Some(SettingsDropdown::MemoryProfile)) => {
            MemoryProfile::ALL.len()
        }
        (SettingsSection::Desktop, Some(SettingsDropdown::WallpaperFit)) => WallpaperFit::ALL.len(),
        (SettingsSection::Desktop, Some(SettingsDropdown::DesktopPose)) => {
            DesktopPoseMode::LABELS.len()
        }
        (SettingsSection::Desktop, Some(SettingsDropdown::QuickMenuPose)) => {
            QUICK_MENU_POSE_LABELS.len()
        }
        _ => 0,
    };

    if extra_opts > 0 {
        total += (extra_opts as f32) * DROPDOWN_OPT_H
            + (extra_opts.saturating_sub(1) as f32) * DROPDOWN_OPT_GAP
            + ROW_GAP;
    }

    total
}

/// Builds the two-column Settings drawer (left scrollable vertical rail + right scrollable pane).
pub fn build_settings_panel(state: &State) -> Element<Msg> {
    Row::new()
        .key("quick-settings-panel")
        .size(Size::new(
            QUICK_MENU_WIDTH.logical_px(),
            SETTINGS_PANEL_HEIGHT.logical_px(),
        ))
        .gap(SETTINGS_GAP.logical_px())
        .child(build_section_rail(state))
        .child(build_section_body(state))
        .into()
}

fn build_section_rail(state: &State) -> Element<Msg> {
    let tab_w = SETTINGS_RAIL_W - RAIL_PAD * 2.0;
    let content_h = settings_rail_content_height();
    let max_scroll = (content_h - SETTINGS_RAIL_VIEW_H).max(0.0);
    let scroll_y = state.settings_ui.rail_scroll_y.clamp(0.0, max_scroll);

    let mut tab_col = Column::new()
        .size(Size::new(tab_w.logical_px(), content_h.logical_px()))
        .gap(RAIL_TAB_GAP.logical_px())
        .position(Position::absolute(0.logical_px(), (-scroll_y).logical_px()));

    for section in SettingsSection::ALL {
        let idx = section.index();
        let active = state.settings_ui.active_section == section;
        let hovered = state.hovered == HoverTarget::SettingsTab(idx);
        let bg = if active {
            MANTLE
        } else if hovered {
            SURFACE0
        } else {
            Color::TRANSPARENT
        };
        let border = if active {
            SURFACE1
        } else if hovered {
            SURFACE0
        } else {
            Color::TRANSPARENT
        };
        let icon_col = if active {
            MAUVE
        } else if hovered {
            TEXT
        } else {
            SUBTEXT1
        };
        let label_col = if active || hovered { TEXT } else { OVERLAY0 };

        let mut tab = Group::new()
            .key(("settings-tab", idx))
            .size(Size::new(tab_w.logical_px(), RAIL_TAB_H.logical_px()))
            .cursor(CursorShape::Pointer)
            .on_hover(Msg::Hover(HoverTarget::SettingsTab(idx)))
            .on_leave(Msg::Leave(HoverTarget::SettingsTab(idx)))
            .on_click(Msg::SettingsSelectSection(section))
            .child(
                Rect::new()
                    .size(Size::fill())
                    .fill(bg)
                    .corner_radius(0.logical_px())
                    .border(1.logical_px(), border),
            );

        if active {
            tab = tab.child(
                Rect::new()
                    .position(Position::absolute(0.logical_px(), 0.logical_px()))
                    .size(Size::new(2.logical_px(), RAIL_TAB_H.logical_px()))
                    .fill(MAUVE),
            );
        }

        tab = tab.child(
            Column::new()
                .size(Size::auto())
                .gap(3.logical_px())
                .align(Align::Center)
                .position(Position::relative_to_parent(
                    Percent::new(50.0),
                    Percent::new(50.0),
                ))
                .anchor(ElementAnchor::Center)
                .child(
                    Text::new(section.icon())
                        .font(font_ui())
                        .font_size(12.5.logical_px())
                        .bounds_mode(TextBoundsMode::Optical)
                        .color(icon_col),
                )
                .child(
                    Text::new(section.short_label())
                        .font(font_ui_semibold())
                        .font_size(8.5.logical_px())
                        .color(label_col),
                ),
        );

        tab_col = tab_col.child(tab);
    }

    let viewport = Clip::new(
        Group::new()
            .size(Size::new(
                tab_w.logical_px(),
                SETTINGS_RAIL_VIEW_H.logical_px(),
            ))
            .child(tab_col),
    )
    .size(Size::new(
        tab_w.logical_px(),
        SETTINGS_RAIL_VIEW_H.logical_px(),
    ))
    .position(Position::absolute(
        RAIL_PAD.logical_px(),
        RAIL_PAD.logical_px(),
    ))
    .on_scroll(Msg::SettingsRailScroll);

    let mut rail = Group::new()
        .key("settings-section-rail")
        .size(Size::new(
            SETTINGS_RAIL_W.logical_px(),
            SETTINGS_PANEL_HEIGHT.logical_px(),
        ))
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(CRUST)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), SURFACE1),
        )
        .child(viewport);

    if max_scroll > 0.5 {
        rail = rail.child(build_scrollbar_thumb(
            SETTINGS_RAIL_W - 3.0,
            RAIL_PAD,
            SETTINGS_RAIL_VIEW_H,
            content_h,
            scroll_y,
        ));
    }

    rail.into()
}

fn build_section_body(state: &State) -> Element<Msg> {
    let section = state.settings_ui.active_section;
    let inner_w = SETTINGS_BODY_W - BODY_PAD * 2.0;

    let banner = build_notched_section_banner(section.title(), inner_w);
    let content_h = settings_list_content_height(state);
    let max_scroll = (content_h - SETTINGS_LIST_VIEW_H).max(0.0);
    let scroll_y = state.settings_ui.list_scroll_y.clamp(0.0, max_scroll);

    let rows = match section {
        SettingsSection::General => build_general_rows(state),
        SettingsSection::Audio => build_audio_rows(state),
        SettingsSection::Desktop => build_desktop_rows(state),
        SettingsSection::Bar => build_bar_rows(state),
    };

    let mut list_col = Column::new()
        .size(Size::new(
            LIST_CONTENT_W.logical_px(),
            content_h.logical_px(),
        ))
        .gap(ROW_GAP.logical_px())
        .position(Position::absolute(0.logical_px(), (-scroll_y).logical_px()));

    for row_el in rows {
        list_col = list_col.child(row_el);
    }

    let mut viewport_group = Group::new()
        .size(Size::new(
            inner_w.logical_px(),
            SETTINGS_LIST_VIEW_H.logical_px(),
        ))
        .child(
            Clip::new(
                Group::new()
                    .size(Size::new(
                        LIST_CONTENT_W.logical_px(),
                        SETTINGS_LIST_VIEW_H.logical_px(),
                    ))
                    .child(list_col),
            )
            .size(Size::new(
                LIST_CONTENT_W.logical_px(),
                SETTINGS_LIST_VIEW_H.logical_px(),
            ))
            .on_scroll(Msg::SettingsListScroll),
        );

    if max_scroll > 0.5 {
        viewport_group = viewport_group.child(build_scrollbar_thumb(
            inner_w - 2.0,
            0.0,
            SETTINGS_LIST_VIEW_H,
            content_h,
            scroll_y,
        ));
    }

    let inner = Column::new()
        .size(Size::fill())
        .padding(Edges::all(BODY_PAD.logical_px()))
        .gap(HEADER_GAP.logical_px())
        .child(banner)
        .child(viewport_group);

    Group::new()
        .key("settings-section-body")
        .size(Size::new(
            SETTINGS_BODY_W.logical_px(),
            SETTINGS_PANEL_HEIGHT.logical_px(),
        ))
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(CRUST)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), SURFACE1),
        )
        .child(inner)
        .into()
}

/// Powerline-notched section title banner (`>  Section  <`) matching the bar's sharp chevron language.
fn build_notched_section_banner(title: &'static str, width: f32) -> Element<Msg> {
    let ribbon_canvas = Canvas::new(
        Size::new(width.logical_px(), HEADER_H.logical_px()),
        move |p| {
            let notch = 9.0_f32;
            let mid_y = HEADER_H * 0.5;
            let mut pb = PathBuilder::new();
            pb.move_to(0.5.logical_px(), 0.5.logical_px())
                .line_to((width - 0.5).logical_px(), 0.5.logical_px())
                .line_to((width - notch).logical_px(), mid_y.logical_px())
                .line_to((width - 0.5).logical_px(), (HEADER_H - 0.5).logical_px())
                .line_to(0.5.logical_px(), (HEADER_H - 0.5).logical_px())
                .line_to(notch.logical_px(), mid_y.logical_px())
                .close();
            p.fill_path(&pb, SURFACE0);
            p.stroke_path(&pb, 1.0.logical_px(), SURFACE1);

            let mut left_accent = PathBuilder::new();
            left_accent
                .move_to(2.0.logical_px(), 1.5.logical_px())
                .line_to((notch + 1.5).logical_px(), mid_y.logical_px())
                .line_to(2.0.logical_px(), (HEADER_H - 1.5).logical_px());
            p.stroke_path(&left_accent, 1.2.logical_px(), MAUVE);

            let mut right_accent = PathBuilder::new();
            right_accent
                .move_to((width - 2.0).logical_px(), 1.5.logical_px())
                .line_to((width - notch - 1.5).logical_px(), mid_y.logical_px())
                .line_to((width - 2.0).logical_px(), (HEADER_H - 1.5).logical_px());
            p.stroke_path(&right_accent, 1.2.logical_px(), MAUVE);
        },
    )
    .cache_key(("settings-banner", title, width.to_bits()))
    .key("settings-banner-bg");

    Group::new()
        .key(("settings-header", title))
        .size(Size::new(width.logical_px(), HEADER_H.logical_px()))
        .child(ribbon_canvas)
        .child(
            Text::new(title)
                .font(font_ui_semibold())
                .font_size(10.5.logical_px())
                .color(TEXT)
                .position(Position::relative_to_parent(
                    Percent::new(50.0),
                    Percent::new(50.0),
                ))
                .anchor(ElementAnchor::Center),
        )
        .into()
}

fn build_scrollbar_thumb(
    x: f32,
    top_y: f32,
    view_h: f32,
    content_h: f32,
    scroll_y: f32,
) -> Element<Msg> {
    let ratio = (view_h / content_h).clamp(0.15, 1.0);
    let thumb_h = (view_h * ratio).max(18.0);
    let max_scroll = (content_h - view_h).max(1.0);
    let track_span = (view_h - thumb_h).max(0.0);
    let thumb_y = top_y + (scroll_y / max_scroll) * track_span;

    Group::new()
        .position(Position::absolute(x.logical_px(), top_y.logical_px()))
        .size(Size::new(2.logical_px(), view_h.logical_px()))
        .child(Rect::new().size(Size::fill()).fill(SURFACE0))
        .child(
            Rect::new()
                .position(Position::absolute(
                    0.logical_px(),
                    (thumb_y - top_y).logical_px(),
                ))
                .size(Size::new(2.logical_px(), thumb_h.logical_px()))
                .fill(MAUVE),
        )
        .into()
}

// --- Section Row Builders ---

/// Minimalist sharp subsection divider (`| TITLE ───────`) matching Catppuccin Mocha geometry.
fn subsection_header(key: &'static str, title: &'static str) -> Element<Msg> {
    Row::new()
        .key(("settings-sub", key))
        .size(Size::new(
            LIST_CONTENT_W.logical_px(),
            SUBSECTION_H.logical_px(),
        ))
        .gap(6.logical_px())
        .align(Align::Center)
        .child(
            Rect::new()
                .size(Size::new(2.logical_px(), 10.logical_px()))
                .fill(MAUVE),
        )
        .child(
            Text::new(title)
                .font(font_ui_semibold())
                .font_size(8.5.logical_px())
                .color(MAUVE),
        )
        .child(
            Rect::new()
                .size(Size::new(Length::Fill, 1.logical_px()))
                .fill(SURFACE0),
        )
        .into()
}

fn build_general_rows(state: &State) -> Vec<Element<Msg>> {
    let g = &state.config.general;
    let countdown_label = if g.power_countdown_secs == 0 {
        String::from("Off")
    } else {
        format!("{}s", g.power_countdown_secs)
    };
    let mut rows = vec![
        subsection_header("gen-clock", "CLOCK & POWER"),
        toggle_row(
            state,
            0,
            "24-Hour Clock",
            g.clock_24h,
            SettingToggleId::Clock24h,
        ),
        toggle_row(
            state,
            1,
            "Date Clock Default",
            g.clock_alt_default,
            SettingToggleId::ClockAltDefault,
        ),
        toggle_row(
            state,
            2,
            "Foreground Booster",
            g.foreground_booster,
            SettingToggleId::ForegroundBooster,
        ),
        stepper_row(
            state,
            3,
            "Power Countdown",
            &countdown_label,
            SettingStepperId::PowerCountdownSecs,
        ),
        subsection_header("gen-cfg", "RUNTIME & CONFIG"),
        dropdown_row(
            state,
            6,
            "Renderer (restart)",
            g.renderer.label(),
            SettingsDropdown::Renderer,
        ),
    ];

    if state.settings_ui.open_dropdown == Some(SettingsDropdown::Renderer) {
        let labels: Vec<&'static str> = RendererMode::ALL.iter().map(|r| r.label()).collect();
        rows.push(dropdown_options_block(
            state,
            SettingsDropdown::Renderer,
            &labels,
            g.renderer.index(),
            180,
        ));
    }

    rows.push(dropdown_row(
        state,
        7,
        "Memory (restart)",
        g.memory_profile.label(),
        SettingsDropdown::MemoryProfile,
    ));
    if state.settings_ui.open_dropdown == Some(SettingsDropdown::MemoryProfile) {
        let labels: Vec<&'static str> = MemoryProfile::ALL.iter().map(|m| m.label()).collect();
        rows.push(dropdown_options_block(
            state,
            SettingsDropdown::MemoryProfile,
            &labels,
            g.memory_profile.index(),
            190,
        ));
    }

    rows.push(value_row(
        state,
        4,
        "Config Storage",
        CONFIG_FILE_NAME,
        SUBTEXT1,
    ));
    rows.push(action_row(
        state,
        5,
        "Reset Defaults",
        "Reset",
        RED,
        Msg::SettingsResetDefaults,
    ));
    rows
}

fn build_audio_rows(state: &State) -> Vec<Element<Msg>> {
    let a = &state.config.audio;
    let vol_label = format!("{}%", a.sound_volume_pct);
    let theme_label = marquee_slice(&state.sounds.detected_theme, 16, 0);
    vec![
        subsection_header("audio-engine", "SOUND ENGINE"),
        toggle_row(
            state,
            10,
            "XDG Sound Theme",
            a.xdg_sounds_enabled,
            SettingToggleId::XdgSoundsEnabled,
        ),
        toggle_row(
            state,
            11,
            "UI Feedback Sounds",
            a.ui_feedback_sounds,
            SettingToggleId::UiFeedbackSounds,
        ),
        toggle_row(
            state,
            12,
            "Notification Sounds",
            a.notification_sounds,
            SettingToggleId::NotificationSounds,
        ),
        stepper_row(
            state,
            13,
            "Sound Volume",
            &vol_label,
            SettingStepperId::SoundVolumePct,
        ),
        subsection_header("audio-theme", "THEME"),
        value_row(state, 14, "Active Theme", &theme_label, MAUVE),
        action_row(
            state,
            15,
            "Preview Cue",
            "Test",
            MAUVE,
            Msg::SettingsPreviewSound,
        ),
    ]
}

fn build_desktop_rows(state: &State) -> Vec<Element<Msg>> {
    let wp = &state.config.wallpaper;
    let sp = &state.config.sprite;
    let mut rows = vec![
        subsection_header("desk-wp", "WALLPAPER"),
        toggle_row(
            state,
            20,
            "Enable Wallpaper",
            wp.enabled,
            SettingToggleId::WallpaperEnabled,
        ),
        build_wallpaper_path_row(state, 21),
    ];

    if let Some(ref err) = state.settings_ui.wallpaper_error {
        rows.push(
            Row::new()
                .key("settings-wp-err")
                .size(Size::new(LIST_CONTENT_W.logical_px(), ERROR_LINE_H.logical_px()))
                .padding(Edges::symmetric(0.logical_px(), 6.logical_px()))
                .align(Align::Center)
                .child(
                    Text::new(marquee_slice(err, 44, 0))
                        .font(font_ui())
                        .font_size(9.logical_px())
                        .color(RED),
                )
                .into(),
        );
    }

    rows.push(dropdown_row(
        state,
        22,
        "Fit Mode",
        wp.fit.label(),
        SettingsDropdown::WallpaperFit,
    ));
    if state.settings_ui.open_dropdown == Some(SettingsDropdown::WallpaperFit) {
        let labels: Vec<&'static str> = WallpaperFit::ALL.iter().map(|f| f.label()).collect();
        rows.push(dropdown_options_block(
            state,
            SettingsDropdown::WallpaperFit,
            &labels,
            wp.fit.index(),
            200,
        ));
    }

    // Companion / Character Sprite subsection (Yuki Nagato)
    rows.push(subsection_header("desk-yuki", "COMPANION · YUKI"));
    rows.push(toggle_row(
        state,
        30,
        "Desktop Sprite",
        sp.desktop_enabled,
        SettingToggleId::SpriteDesktopEnabled,
    ));
    rows.push(toggle_row(
        state,
        31,
        "Workspace Rotate",
        sp.rotate_on_workspace,
        SettingToggleId::SpriteRotateOnWorkspace,
    ));
    rows.push(toggle_row(
        state,
        32,
        "Notif Mouth React",
        sp.mouth_on_notification,
        SettingToggleId::SpriteMouthOnNotification,
    ));
    rows.push(dropdown_row(
        state,
        33,
        "Desktop Pose",
        sp.desktop_pose.label(),
        SettingsDropdown::DesktopPose,
    ));
    if state.settings_ui.open_dropdown == Some(SettingsDropdown::DesktopPose) {
        rows.push(dropdown_options_block(
            state,
            SettingsDropdown::DesktopPose,
            &DesktopPoseMode::LABELS,
            sp.desktop_pose.index(),
            300,
        ));
    }

    let qm_label = QUICK_MENU_POSE_LABELS[sp.quick_menu_pose % QUICK_MENU_POSE_LABELS.len()];
    rows.push(dropdown_row(
        state,
        34,
        "Quick Menu Pose",
        qm_label,
        SettingsDropdown::QuickMenuPose,
    ));
    if state.settings_ui.open_dropdown == Some(SettingsDropdown::QuickMenuPose) {
        rows.push(dropdown_options_block(
            state,
            SettingsDropdown::QuickMenuPose,
            &QUICK_MENU_POSE_LABELS,
            sp.quick_menu_pose % QUICK_MENU_POSE_LABELS.len(),
            350,
        ));
    }

    let scale_label = format!("{}%", sp.desktop_scale_pct);
    let interval_label = format!("{}s", sp.rotate_interval_secs);
    rows.push(stepper_row(
        state,
        35,
        "Desktop Scale",
        &scale_label,
        SettingStepperId::DesktopScalePct,
    ));
    rows.push(stepper_row(
        state,
        36,
        "Rotate Interval",
        &interval_label,
        SettingStepperId::SpriteRotateIntervalSecs,
    ));
    rows
}

fn build_bar_rows(state: &State) -> Vec<Element<Msg>> {
    let w = &state.config.widgets;
    let mut rows = vec![
        subsection_header("bar-left", "LEFT & UTILITY"),
        toggle_row(
            state,
            40,
            "System Info",
            w.show_sysinfo,
            SettingToggleId::WidgetSysInfo,
        ),
        toggle_row(
            state,
            41,
            "Workspaces",
            w.show_workspaces,
            SettingToggleId::WidgetWorkspaces,
        ),
        toggle_row(
            state,
            42,
            "Window Title",
            w.show_window_info,
            SettingToggleId::WidgetWindowInfo,
        ),
        toggle_row(
            state,
            43,
            "Notifications",
            w.show_notifications,
            SettingToggleId::WidgetNotifications,
        ),
        toggle_row(
            state,
            44,
            "Media (MPRIS)",
            w.show_mpris,
            SettingToggleId::WidgetMpris,
        ),
        toggle_row(
            state,
            45,
            "System Tray",
            w.show_tray,
            SettingToggleId::WidgetTray,
        ),
        subsection_header("bar-metrics", "METRICS & DRAWER"),
    ];

    let total = w.right_metrics.len();
    for (idx, slot) in w.right_metrics.iter().copied().enumerate() {
        rows.push(metric_slot_row(state, 50 + idx, idx, total, slot));
    }
    rows
}

/// Unified reorderable metric row: `[ Label ]  [ Bar | Drawer | Off ]  [ ▲ ] [ ▼ ]`.
fn metric_slot_row(
    state: &State,
    row_id: usize,
    idx: usize,
    total: usize,
    slot: RightMetricSlot,
) -> Element<Msg> {
    let place_target = HoverTarget::SettingsMetricPlace(idx);
    let up_target = HoverTarget::SettingsMetricUp(idx);
    let down_target = HoverTarget::SettingsMetricDown(idx);
    let place_hovered = state.hovered == place_target;
    let up_hovered = state.hovered == up_target;
    let down_hovered = state.hovered == down_target;
    let can_up = idx > 0;
    let can_down = idx + 1 < total;

    let (place_accent, place_border) = match slot.placement {
        MetricPlacement::Bar => (MAUVE, MAUVE),
        MetricPlacement::Drawer => (BLUE, BLUE),
        MetricPlacement::Hidden => (OVERLAY0, SURFACE1),
    };

    let place_btn = Group::new()
        .key(("settings-metric-place", slot.id as usize))
        .size(Size::new(56.logical_px(), 22.logical_px()))
        .cursor(CursorShape::Pointer)
        .on_hover(Msg::Hover(place_target))
        .on_leave(Msg::Leave(place_target))
        .on_click(Msg::SettingsCycleMetricPlacement(slot.id))
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(if place_hovered { SURFACE0 } else { CRUST })
                .corner_radius(0.logical_px())
                .border(1.logical_px(), place_border),
        )
        .child(
            Text::new(slot.placement.label())
                .font(font_ui_semibold())
                .font_size(9.5.logical_px())
                .color(place_accent)
                .position(Position::relative_to_parent(
                    Percent::new(50.0),
                    Percent::new(50.0),
                ))
                .anchor(ElementAnchor::Center),
        );

    let arrow_btn = |key_tag: &'static str,
                     glyph: &'static str,
                     target: HoverTarget,
                     hovered: bool,
                     enabled: bool,
                     delta: i32| {
        let mut g = Group::new()
            .key((key_tag, slot.id as usize))
            .size(Size::new(20.logical_px(), 22.logical_px()))
            .child(
                Rect::new()
                    .size(Size::fill())
                    .fill(if enabled && hovered { SURFACE0 } else { CRUST })
                    .corner_radius(0.logical_px())
                    .border(
                        1.logical_px(),
                        if enabled && hovered {
                            MAUVE
                        } else if enabled {
                            SURFACE1
                        } else {
                            SURFACE0
                        },
                    ),
            )
            .child(
                Text::new(glyph)
                    .font(font_ui())
                    .font_size(9.5.logical_px())
                    .bounds_mode(TextBoundsMode::Optical)
                    .color(if !enabled {
                        SURFACE1
                    } else if hovered {
                        MAUVE
                    } else {
                        SUBTEXT1
                    })
                    .position(Position::relative_to_parent(
                        Percent::new(50.0),
                        Percent::new(50.0),
                    ))
                    .anchor(ElementAnchor::Center),
            );
        if enabled {
            g = g
                .cursor(CursorShape::Pointer)
                .on_hover(Msg::Hover(target))
                .on_leave(Msg::Leave(target))
                .on_click(Msg::SettingsMoveMetric { id: slot.id, delta });
        }
        g
    };

    let controls = Row::new()
        .size(Size::auto())
        .gap(4.logical_px())
        .align(Align::Center)
        .position(Position::absolute(
            (LIST_CONTENT_W - 8.0).logical_px(),
            (ROW_H * 0.5).logical_px(),
        ))
        .anchor(ElementAnchor::CenterRight)
        .child(place_btn)
        .child(arrow_btn(
            "settings-metric-up",
            "\u{f0d8}",
            up_target,
            up_hovered,
            can_up,
            -1,
        ))
        .child(arrow_btn(
            "settings-metric-dn",
            "\u{f0d7}",
            down_target,
            down_hovered,
            can_down,
            1,
        ));

    let any_hover = place_hovered || (can_up && up_hovered) || (can_down && down_hovered);
    let label_col = if slot.placement == MetricPlacement::Hidden {
        OVERLAY0
    } else if any_hover {
        TEXT
    } else {
        SUBTEXT1
    };

    let inner = Group::new()
        .size(Size::fill())
        .child(row_label(slot.id.label(), label_col))
        .child(controls)
        .into();

    row_shell(row_id, any_hover, None, inner).into()
}

// --- Shared Control Primitives ---

fn row_shell(
    row_id: usize,
    hovered: bool,
    border_override: Option<Color>,
    content: Element<Msg>,
) -> Group<Msg> {
    let border = border_override.unwrap_or(if hovered { SURFACE1 } else { SURFACE0 });
    Group::new()
        .key(("settings-row", row_id))
        .size(Size::new(LIST_CONTENT_W.logical_px(), ROW_H.logical_px()))
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(MANTLE)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), border),
        )
        .child(content)
}

fn row_label(label: &'static str, color: Color) -> Element<Msg> {
    Text::new(label)
        .font(font_ui())
        .font_size(10.5.logical_px())
        .color(color)
        .position(Position::absolute(8.logical_px(), (ROW_H * 0.5).logical_px()))
        .anchor(ElementAnchor::CenterLeft)
        .into()
}

fn toggle_row(
    state: &State,
    row_id: usize,
    label: &'static str,
    checked: bool,
    toggle_id: SettingToggleId,
) -> Element<Msg> {
    let target = HoverTarget::SettingsRow(row_id);
    let hovered = state.hovered == target;

    let track_w = 34.0_f32;
    let track_h = 16.0_f32;
    let thumb_w = 12.0_f32;
    let thumb_h = 10.0_f32;
    let thumb_x = if checked { track_w - thumb_w - 3.0 } else { 3.0 };
    let track_border = if checked { MAUVE } else { SURFACE1 };
    let thumb_col = if checked { MAUVE } else { OVERLAY0 };

    let switch = Group::new()
        .position(Position::absolute(
            (LIST_CONTENT_W - 8.0).logical_px(),
            (ROW_H * 0.5).logical_px(),
        ))
        .anchor(ElementAnchor::CenterRight)
        .size(Size::new(track_w.logical_px(), track_h.logical_px()))
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(CRUST)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), track_border),
        )
        .child(
            Rect::new()
                .position(Position::absolute(thumb_x.logical_px(), 3.logical_px()))
                .size(Size::new(thumb_w.logical_px(), thumb_h.logical_px()))
                .fill(thumb_col),
        );

    let inner = Group::new()
        .size(Size::fill())
        .child(row_label(label, if hovered { TEXT } else { SUBTEXT1 }))
        .child(switch)
        .into();

    row_shell(row_id, hovered, None, inner)
        .cursor(CursorShape::Pointer)
        .on_hover(Msg::Hover(target))
        .on_leave(Msg::Leave(target))
        .on_click(Msg::SettingsToggle(toggle_id))
        .into()
}

fn build_wallpaper_path_row(state: &State, row_id: usize) -> Element<Msg> {
    let focused = state.settings_ui.wallpaper_focused;
    let display = state.settings_ui.wallpaper_input.display_text();
    let is_empty = display.is_empty();
    let input_w = 186.0_f32;
    let input_h = 22.0_f32;

    // Show the tail of long paths so the user always sees what they are typing.
    let visible_text = if is_empty {
        String::from("Built-in dark Gentoo")
    } else {
        tail_chars(&display, 26)
    };
    let cursor_x = state
        .settings_ui
        .wallpaper_cursor_x
        .clamp(0.0, input_w - 10.0);

    let mut input_box = Group::new()
        .key("settings-wp-input")
        .size(Size::new(input_w.logical_px(), input_h.logical_px()))
        .cursor(CursorShape::Text)
        .on_click(Msg::SettingsWallpaperPathFocus)
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(CRUST)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), if focused { MAUVE } else { SURFACE1 }),
        )
        .child(
            Text::new(visible_text)
                .font(font_ui())
                .font_size(9.5.logical_px())
                .color(if is_empty { OVERLAY0 } else { TEXT })
                .position(Position::absolute(
                    5.logical_px(),
                    (input_h * 0.5).logical_px(),
                ))
                .anchor(ElementAnchor::CenterLeft),
        );

    if focused {
        input_box = input_box.child(
            Rect::new()
                .position(Position::absolute(
                    (5.0 + cursor_x).logical_px(),
                    5.logical_px(),
                ))
                .size(Size::new(1.5.logical_px(), 12.logical_px()))
                .fill(MAUVE),
        );
    }

    let apply_hovered = state.hovered == HoverTarget::SettingsPathApply;
    let apply_btn = Group::new()
        .key("settings-wp-apply")
        .size(Size::new(22.logical_px(), input_h.logical_px()))
        .cursor(CursorShape::Pointer)
        .on_hover(Msg::Hover(HoverTarget::SettingsPathApply))
        .on_leave(Msg::Leave(HoverTarget::SettingsPathApply))
        .on_click(Msg::SettingsWallpaperPathApply)
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(if apply_hovered { SURFACE0 } else { CRUST })
                .corner_radius(0.logical_px())
                .border(1.logical_px(), if apply_hovered { GREEN } else { SURFACE1 }),
        )
        .child(
            Text::new("\u{f00c}")
                .font(font_ui())
                .font_size(9.5.logical_px())
                .bounds_mode(TextBoundsMode::Optical)
                .color(if apply_hovered { GREEN } else { SUBTEXT1 })
                .position(Position::relative_to_parent(
                    Percent::new(50.0),
                    Percent::new(50.0),
                ))
                .anchor(ElementAnchor::Center),
        );

    let clear_hovered = state.hovered == HoverTarget::SettingsPathClear;
    let clear_btn = Group::new()
        .key("settings-wp-clear")
        .size(Size::new(22.logical_px(), input_h.logical_px()))
        .cursor(CursorShape::Pointer)
        .on_hover(Msg::Hover(HoverTarget::SettingsPathClear))
        .on_leave(Msg::Leave(HoverTarget::SettingsPathClear))
        .on_click(Msg::SettingsWallpaperPathClear)
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(if clear_hovered { SURFACE0 } else { CRUST })
                .corner_radius(0.logical_px())
                .border(1.logical_px(), if clear_hovered { RED } else { SURFACE1 }),
        )
        .child(
            Text::new("\u{f00d}")
                .font(font_ui())
                .font_size(9.5.logical_px())
                .bounds_mode(TextBoundsMode::Optical)
                .color(if clear_hovered { RED } else { SUBTEXT1 })
                .position(Position::relative_to_parent(
                    Percent::new(50.0),
                    Percent::new(50.0),
                ))
                .anchor(ElementAnchor::Center),
        );

    let controls = Row::new()
        .size(Size::auto())
        .gap(4.logical_px())
        .align(Align::Center)
        .position(Position::absolute(
            (LIST_CONTENT_W - 6.0).logical_px(),
            (ROW_H * 0.5).logical_px(),
        ))
        .anchor(ElementAnchor::CenterRight)
        .child(input_box)
        .child(apply_btn)
        .child(clear_btn);

    let inner = Group::new()
        .size(Size::fill())
        .child(row_label("Path", if focused { TEXT } else { SUBTEXT1 }))
        .child(controls)
        .into();

    row_shell(row_id, focused, focused.then_some(MAUVE), inner).into()
}

fn value_row(
    state: &State,
    row_id: usize,
    label: &'static str,
    value: &str,
    accent: Color,
) -> Element<Msg> {
    let pill = Group::new()
        .size(Size::new(116.logical_px(), 20.logical_px()))
        .position(Position::absolute(
            (LIST_CONTENT_W - 8.0).logical_px(),
            (ROW_H * 0.5).logical_px(),
        ))
        .anchor(ElementAnchor::CenterRight)
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(CRUST)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), SURFACE1),
        )
        .child(
            Text::new(value.to_owned())
                .font(font_ui_semibold())
                .font_size(9.5.logical_px())
                .color(accent)
                .position(Position::relative_to_parent(
                    Percent::new(50.0),
                    Percent::new(50.0),
                ))
                .anchor(ElementAnchor::Center),
        );

    let hovered = state.hovered == HoverTarget::SettingsRow(row_id);
    let inner = Group::new()
        .size(Size::fill())
        .child(row_label(label, SUBTEXT1))
        .child(pill)
        .into();

    row_shell(row_id, hovered, None, inner).into()
}

fn action_row(
    state: &State,
    row_id: usize,
    label: &'static str,
    button_label: &'static str,
    accent: Color,
    msg: Msg,
) -> Element<Msg> {
    let target = HoverTarget::SettingsAction(row_id);
    let hovered = state.hovered == target;

    let btn = Group::new()
        .key(("settings-action", row_id))
        .size(Size::new(92.logical_px(), 20.logical_px()))
        .position(Position::absolute(
            (LIST_CONTENT_W - 8.0).logical_px(),
            (ROW_H * 0.5).logical_px(),
        ))
        .anchor(ElementAnchor::CenterRight)
        .cursor(CursorShape::Pointer)
        .on_hover(Msg::Hover(target))
        .on_leave(Msg::Leave(target))
        .on_click(msg)
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(if hovered { SURFACE0 } else { CRUST })
                .corner_radius(0.logical_px())
                .border(1.logical_px(), if hovered { accent } else { SURFACE1 }),
        )
        .child(
            Text::new(button_label)
                .font(font_ui_semibold())
                .font_size(9.5.logical_px())
                .color(if hovered { accent } else { TEXT })
                .position(Position::relative_to_parent(
                    Percent::new(50.0),
                    Percent::new(50.0),
                ))
                .anchor(ElementAnchor::Center),
        );

    let inner = Group::new()
        .size(Size::fill())
        .child(row_label(label, if hovered { TEXT } else { SUBTEXT1 }))
        .child(btn)
        .into();

    row_shell(row_id, hovered, None, inner).into()
}

fn dropdown_row(
    state: &State,
    row_id: usize,
    label: &'static str,
    current_value: &'static str,
    dropdown: SettingsDropdown,
) -> Element<Msg> {
    let target = HoverTarget::SettingsRow(row_id);
    let hovered = state.hovered == target;
    let is_open = state.settings_ui.open_dropdown == Some(dropdown);
    let caret = if is_open { "\u{f0d8}" } else { "\u{f0d7}" };

    let trigger = Group::new()
        .size(Size::new(DROPDOWN_BOX_W.logical_px(), 22.logical_px()))
        .position(Position::absolute(
            (LIST_CONTENT_W - 8.0).logical_px(),
            (ROW_H * 0.5).logical_px(),
        ))
        .anchor(ElementAnchor::CenterRight)
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(if is_open || hovered { SURFACE0 } else { CRUST })
                .corner_radius(0.logical_px())
                .border(1.logical_px(), if is_open { MAUVE } else { SURFACE1 }),
        )
        .child(
            Text::new(current_value)
                .font(font_ui_semibold())
                .font_size(9.5.logical_px())
                .color(if is_open { MAUVE } else { TEXT })
                .position(Position::absolute(8.logical_px(), 11.logical_px()))
                .anchor(ElementAnchor::CenterLeft),
        )
        .child(
            Text::new(caret)
                .font(font_ui())
                .font_size(9.5.logical_px())
                .bounds_mode(TextBoundsMode::Optical)
                .color(if is_open { MAUVE } else { SUBTEXT1 })
                .position(Position::absolute(
                    (DROPDOWN_BOX_W - 8.0).logical_px(),
                    11.logical_px(),
                ))
                .anchor(ElementAnchor::CenterRight),
        );

    let inner = Group::new()
        .size(Size::fill())
        .child(row_label(
            label,
            if is_open || hovered { TEXT } else { SUBTEXT1 },
        ))
        .child(trigger)
        .into();

    row_shell(row_id, hovered, is_open.then_some(MAUVE), inner)
        .cursor(CursorShape::Pointer)
        .on_hover(Msg::Hover(target))
        .on_leave(Msg::Leave(target))
        .on_click(Msg::SettingsToggleDropdown(dropdown))
        .into()
}

fn dropdown_options_block(
    state: &State,
    dropdown: SettingsDropdown,
    options: &[&'static str],
    active_index: usize,
    hover_base: usize,
) -> Element<Msg> {
    let block_h = (options.len() as f32) * DROPDOWN_OPT_H
        + (options.len().saturating_sub(1) as f32) * DROPDOWN_OPT_GAP;

    let mut opt_col = Column::new()
        .size(Size::new(DROPDOWN_BOX_W.logical_px(), block_h.logical_px()))
        .gap(DROPDOWN_OPT_GAP.logical_px())
        .position(Position::absolute(
            (LIST_CONTENT_W - 8.0).logical_px(),
            0.logical_px(),
        ))
        .anchor(ElementAnchor::TopRight);

    for (idx, &opt_label) in options.iter().enumerate() {
        let target = HoverTarget::SettingsDropdownOpt(hover_base + idx);
        let hovered = state.hovered == target;
        let selected = idx == active_index;
        let bg = if selected || hovered { SURFACE0 } else { CRUST };
        let border = if selected {
            MAUVE
        } else if hovered {
            SURFACE1
        } else {
            SURFACE0
        };
        let fg = if selected {
            MAUVE
        } else if hovered {
            TEXT
        } else {
            SUBTEXT1
        };

        let opt = Group::new()
            .key(("settings-dd-opt", hover_base + idx))
            .size(Size::new(
                DROPDOWN_BOX_W.logical_px(),
                DROPDOWN_OPT_H.logical_px(),
            ))
            .cursor(CursorShape::Pointer)
            .on_hover(Msg::Hover(target))
            .on_leave(Msg::Leave(target))
            .on_click(Msg::SettingsSelectDropdown {
                dropdown,
                index: idx,
            })
            .child(
                Rect::new()
                    .size(Size::fill())
                    .fill(bg)
                    .corner_radius(0.logical_px())
                    .border(1.logical_px(), border),
            )
            .child(
                Text::new(opt_label)
                    .font(if selected {
                        font_ui_semibold()
                    } else {
                        font_ui()
                    })
                    .font_size(9.5.logical_px())
                    .color(fg)
                    .position(Position::relative_to_parent(
                        Percent::new(50.0),
                        Percent::new(50.0),
                    ))
                    .anchor(ElementAnchor::Center),
            );

        opt_col = opt_col.child(opt);
    }

    Group::new()
        .key(("settings-dd-block", hover_base))
        .size(Size::new(LIST_CONTENT_W.logical_px(), block_h.logical_px()))
        .child(opt_col)
        .into()
}

fn stepper_row(
    state: &State,
    row_id: usize,
    label: &'static str,
    value_label: &str,
    stepper_id: SettingStepperId,
) -> Element<Msg> {
    let minus_target = HoverTarget::SettingsStepperMinus(row_id);
    let plus_target = HoverTarget::SettingsStepperPlus(row_id);
    let minus_hovered = state.hovered == minus_target;
    let plus_hovered = state.hovered == plus_target;

    let step_btn = |key_tag: &'static str,
                    glyph: &'static str,
                    target: HoverTarget,
                    hovered: bool,
                    delta: i32| {
        Group::new()
            .key((key_tag, row_id))
            .size(Size::new(22.logical_px(), 22.logical_px()))
            .cursor(CursorShape::Pointer)
            .on_hover(Msg::Hover(target))
            .on_leave(Msg::Leave(target))
            .on_click(Msg::SettingsStep {
                id: stepper_id,
                delta,
            })
            .child(
                Rect::new()
                    .size(Size::fill())
                    .fill(if hovered { SURFACE0 } else { CRUST })
                    .corner_radius(0.logical_px())
                    .border(1.logical_px(), if hovered { MAUVE } else { SURFACE1 }),
            )
            .child(
                Text::new(glyph)
                    .font(font_ui_semibold())
                    .font_size(11.logical_px())
                    .color(if hovered { MAUVE } else { TEXT })
                    .position(Position::relative_to_parent(
                        Percent::new(50.0),
                        Percent::new(50.0),
                    ))
                    .anchor(ElementAnchor::Center),
            )
    };

    let val_box = Group::new()
        .size(Size::new(58.logical_px(), 22.logical_px()))
        .child(
            Rect::new()
                .size(Size::fill())
                .fill(CRUST)
                .corner_radius(0.logical_px())
                .border(1.logical_px(), SURFACE1),
        )
        .child(
            Text::new(value_label.to_owned())
                .font(font_ui_semibold())
                .font_size(9.5.logical_px())
                .color(TEXT)
                .position(Position::relative_to_parent(
                    Percent::new(50.0),
                    Percent::new(50.0),
                ))
                .anchor(ElementAnchor::Center),
        );

    let controls = Row::new()
        .size(Size::auto())
        .gap(4.logical_px())
        .align(Align::Center)
        .position(Position::absolute(
            (LIST_CONTENT_W - 8.0).logical_px(),
            (ROW_H * 0.5).logical_px(),
        ))
        .anchor(ElementAnchor::CenterRight)
        .child(step_btn("settings-step-m", "-", minus_target, minus_hovered, -1))
        .child(val_box)
        .child(step_btn("settings-step-p", "+", plus_target, plus_hovered, 1));

    let inner = Group::new()
        .size(Size::fill())
        .child(row_label(
            label,
            if minus_hovered || plus_hovered {
                TEXT
            } else {
                SUBTEXT1
            },
        ))
        .child(controls)
        .into();

    row_shell(row_id, minus_hovered || plus_hovered, None, inner).into()
}

fn tail_chars(s: &str, max_chars: usize) -> String {
    let count = s.chars().count();
    if count <= max_chars {
        return s.to_owned();
    }
    let skip = count.saturating_sub(max_chars.saturating_sub(1));
    let tail: String = s.chars().skip(skip).collect();
    format!("…{tail}")
}
