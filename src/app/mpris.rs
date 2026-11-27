//! MPRIS session state, active player selection, album art loading, and marquee scrolling.

use std::f32::consts::TAU;
use std::path::Path;
use std::time::Instant;

use crate::app::anim::RevealAnim;
use crate::app::asset_lru::PathAssetCache;
use crate::theme::{MPRIS_BAR_TEXT_WIDTH, font_ui, font_ui_bold, font_ui_semibold};
use crate::widgets::text::format_artist_album;
use waywi::prelude::*;

/// Inner horizontal clip width of the MPRIS bar text column (`MPRIS_BAR_TEXT_WIDTH - 2*4px` padding).
pub const MPRIS_BAR_TEXT_INNER_W: f32 = MPRIS_BAR_TEXT_WIDTH - 8.0;
/// Inner horizontal clip width of the MPRIS popup text column (`310 - 24 pad - 52 art - 12 gap`).
pub const MPRIS_POPUP_TEXT_INNER_W: f32 = 222.0;
/// Pixel gap between the end of a scrolling bar label and its next seamless loop.
pub const MPRIS_MARQUEE_GAP_PX: f32 = 28.0;
/// Smooth horizontal scroll speed in logical px/s for overflowing MPRIS bar labels.
const MPRIS_MARQUEE_SPEED_PX_S: f32 = 26.0;
/// Initial hold before starting / restarting a seamless marquee loop.
const MPRIS_MARQUEE_PAUSE_SECS: f32 = 1.6;
/// Vinyl album disc angular velocity (~8.4s per revolution) while playing.
const MPRIS_DISC_SPIN_RAD_S: f32 = 0.75;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MprisControlAction {
    PlayPause,
    Stop,
    Previous,
    Next,
    ToggleShuffle,
    CycleLoop,
}

#[derive(Default)]
pub struct MprisState {
    pub players: Vec<MprisSnapshot>,
    pub selected_service: Option<String>,
    pub art_asset: Option<ImageAsset>,
    pub art_url_loaded: String,
    pub marquee_tick: u32,
    pub sub_ticks: u8,
    pub popup_anim: RevealAnim,
    pub popup_left_x: f32,
    /// Current vinyl disc rotation angle in radians (advances via VSync while playing).
    pub disc_angle_rad: f32,
    /// Exact measured bar top-line text (`artist - album`) and pixel width.
    pub bar_top_text: String,
    pub bar_top_width_px: f32,
    pub bar_top_scroll_px: f32,
    pub bar_top_pause_secs: f32,
    /// Exact measured bar bottom-line text (`title`) and pixel width.
    pub bar_bot_text: String,
    pub bar_bot_width_px: f32,
    pub bar_bot_scroll_px: f32,
    pub bar_bot_pause_secs: f32,
    /// Exact measured popup title (`11.5px` bold) & subtitle (`10.5px` regular) widths and scroll state.
    pub popup_title_width_px: f32,
    pub popup_title_scroll_px: f32,
    pub popup_title_pause_secs: f32,
    pub popup_sub_width_px: f32,
    pub popup_sub_scroll_px: f32,
    pub popup_sub_pause_secs: f32,
    last_bar_anim_tick: Option<Instant>,
}

impl MprisState {
    #[must_use]
    pub fn active_player(&self) -> Option<&MprisSnapshot> {
        if let Some(svc) = &self.selected_service
            && let Some(found) = self.players.iter().find(|p| &p.service_name == svc)
        {
            return Some(found);
        }
        self.players
            .iter()
            .find(|p| p.status == MprisPlaybackStatus::Playing)
            .or_else(|| {
                self.players
                    .iter()
                    .find(|p| p.status == MprisPlaybackStatus::Paused)
            })
            .or_else(|| self.players.first())
    }

    pub fn active_player_mut(&mut self) -> Option<&mut MprisSnapshot> {
        let target_svc = self.active_player().map(|p| p.service_name.clone())?;
        self.players
            .iter_mut()
            .find(|p| p.service_name == target_svc)
    }

    #[must_use]
    pub fn has_active_session(&self) -> bool {
        self.active_player().is_some_and(|p| {
            p.status != MprisPlaybackStatus::Stopped
                || p.title.as_deref().is_some_and(|t| !t.trim().is_empty())
        })
    }

    #[must_use]
    pub const fn is_popup_open(&self) -> bool {
        self.popup_anim.is_open()
    }

    #[must_use]
    pub fn popup_progress(&self) -> f32 {
        self.popup_anim.progress()
    }

    #[must_use]
    pub fn is_popup_visible(&self) -> bool {
        self.popup_anim.is_visible()
    }

    #[must_use]
    pub fn is_playing(&self) -> bool {
        self.active_player()
            .is_some_and(|p| p.status == MprisPlaybackStatus::Playing)
    }

    #[must_use]
    pub fn is_animating_popup(&self) -> bool {
        self.popup_anim.is_animating()
    }

    /// True while the bar vinyl disc or any pixel-exact MPRIS marquee needs smooth VSync frames.
    #[must_use]
    pub fn is_animating_bar(&self) -> bool {
        (self.has_active_session() && self.is_playing())
            || (self.is_popup_visible()
                && (self.popup_title_overflows() || self.popup_sub_overflows()))
    }

    #[must_use]
    pub const fn top_overflows(&self) -> bool {
        self.bar_top_width_px > MPRIS_BAR_TEXT_INNER_W + 0.5
    }

    #[must_use]
    pub const fn bot_overflows(&self) -> bool {
        self.bar_bot_width_px > MPRIS_BAR_TEXT_INNER_W + 0.5
    }

    #[must_use]
    pub const fn popup_title_overflows(&self) -> bool {
        self.popup_title_width_px > MPRIS_POPUP_TEXT_INNER_W + 0.5
    }

    #[must_use]
    pub const fn popup_sub_overflows(&self) -> bool {
        self.popup_sub_width_px > MPRIS_POPUP_TEXT_INNER_W + 0.5
    }

    pub fn advance_popup_animation(&mut self, now: Instant) {
        self.popup_anim.advance(now);
    }

    /// Advances smooth VSync vinyl disc rotation and pixel-exact bar & popup marquee offsets.
    pub fn advance_bar_animation(&mut self, now: Instant) {
        if !self.is_animating_bar() {
            self.last_bar_anim_tick = None;
            return;
        }
        let dt = self.last_bar_anim_tick.map_or(1.0 / 60.0, |prev| {
            now.saturating_duration_since(prev)
                .as_secs_f32()
                .clamp(1.0 / 240.0, 0.05)
        });
        self.last_bar_anim_tick = Some(now);

        if self.has_active_session() && self.is_playing() {
            if self.art_asset.is_some() {
                self.disc_angle_rad = (self.disc_angle_rad + dt * MPRIS_DISC_SPIN_RAD_S) % TAU;
            }
            step_pixel_marquee(
                self.bar_top_width_px,
                MPRIS_BAR_TEXT_INNER_W,
                &mut self.bar_top_scroll_px,
                &mut self.bar_top_pause_secs,
                dt,
            );
            step_pixel_marquee(
                self.bar_bot_width_px,
                MPRIS_BAR_TEXT_INNER_W,
                &mut self.bar_bot_scroll_px,
                &mut self.bar_bot_pause_secs,
                dt,
            );
        }
        if self.is_popup_visible() {
            step_pixel_marquee(
                self.popup_title_width_px,
                MPRIS_POPUP_TEXT_INNER_W,
                &mut self.popup_title_scroll_px,
                &mut self.popup_title_pause_secs,
                dt,
            );
            step_pixel_marquee(
                self.popup_sub_width_px,
                MPRIS_POPUP_TEXT_INNER_W,
                &mut self.popup_sub_scroll_px,
                &mut self.popup_sub_pause_secs,
                dt,
            );
        }
    }

    pub fn open_popup(&mut self) {
        self.popup_anim.set_open(true);
        self.reset_popup_marquee();
    }

    pub fn toggle_popup(&mut self) {
        if self.popup_anim.toggle() {
            self.reset_popup_marquee();
        }
    }

    fn reset_popup_marquee(&mut self) {
        self.popup_title_scroll_px = 0.0;
        self.popup_title_pause_secs = MPRIS_MARQUEE_PAUSE_SECS;
        self.popup_sub_scroll_px = 0.0;
        self.popup_sub_pause_secs = MPRIS_MARQUEE_PAUSE_SECS;
    }

    pub fn close_popup(&mut self) {
        self.popup_anim.set_open(false);
    }

    pub fn set_players(
        &mut self,
        list: Vec<MprisSnapshot>,
        path_cache: &mut PathAssetCache,
        ctx: &mut EffectCtx<'_, crate::app::msg::Msg>,
    ) {
        self.players = list;
        self.sync_selection_and_art(path_cache, ctx);
    }

    pub fn upsert_player(
        &mut self,
        snap: MprisSnapshot,
        path_cache: &mut PathAssetCache,
        ctx: &mut EffectCtx<'_, crate::app::msg::Msg>,
    ) {
        if let Some(slot) = self
            .players
            .iter_mut()
            .find(|p| p.service_name == snap.service_name)
        {
            *slot = snap;
        } else {
            self.players.push(snap);
        }
        self.sync_selection_and_art(path_cache, ctx);
    }

    pub fn remove_player(
        &mut self,
        service_name: &str,
        path_cache: &mut PathAssetCache,
        ctx: &mut EffectCtx<'_, crate::app::msg::Msg>,
    ) {
        self.players.retain(|p| p.service_name != service_name);
        if self.selected_service.as_deref() == Some(service_name) {
            self.selected_service = None;
        }
        self.sync_selection_and_art(path_cache, ctx);
    }

    pub fn apply_seeked(&mut self, service_name: &str, position_us: i64) {
        if let Some(p) = self
            .players
            .iter_mut()
            .find(|p| p.service_name == service_name)
        {
            p.position_us = Some(position_us.max(0));
        }
    }

    pub fn advance_tick(&mut self) {
        self.marquee_tick = self.marquee_tick.wrapping_add(1);
        self.sub_ticks = (self.sub_ticks + 1) % 5;
        if self.sub_ticks == 0
            && let Some(p) = self.active_player_mut()
            && p.status == MprisPlaybackStatus::Playing
            && let Some(pos) = p.position_us
        {
            let next = pos.saturating_add(1_000_000);
            p.position_us = Some(p.length_us.map_or(next, |len| next.min(len.max(0))));
        }
    }

    fn sync_selection_and_art(&mut self, path_cache: &mut PathAssetCache, ctx: &mut EffectCtx<'_, crate::app::msg::Msg>) {
        if let Some(playing) = self
            .players
            .iter()
            .find(|p| p.status == MprisPlaybackStatus::Playing)
        {
            self.selected_service = Some(playing.service_name.clone());
        } else if self.selected_service.is_none()
            && let Some(first) = self.players.first()
        {
            self.selected_service = Some(first.service_name.clone());
        }

        let (top_raw, bot_raw) = if let Some(player) = self.active_player() {
            let top = format_artist_album(player);
            let bot = player
                .title
                .as_deref()
                .filter(|s| !s.trim().is_empty())
                .or(player.identity.as_deref())
                .unwrap_or("Playing")
                .trim()
                .to_owned();
            (top, bot)
        } else {
            (String::new(), String::new())
        };

        if top_raw != self.bar_top_text {
            self.bar_top_text = top_raw;
            if self.bar_top_text.is_empty() {
                self.bar_top_width_px = 0.0;
                self.popup_sub_width_px = 0.0;
            } else {
                let bar_opts = TextLayoutOptions::new(font_ui(), 9.5.logical_px()).line_height(1.05);
                self.bar_top_width_px = ctx
                    .measure_text_cursor_with(&self.bar_top_text, &bar_opts, self.bar_top_text.len())
                    .size
                    .width
                    .get();
                let popup_opts = TextLayoutOptions::new(font_ui(), 10.5.logical_px());
                self.popup_sub_width_px = ctx
                    .measure_text_cursor_with(
                        &self.bar_top_text,
                        &popup_opts,
                        self.bar_top_text.len(),
                    )
                    .size
                    .width
                    .get();
            }
            self.bar_top_scroll_px = 0.0;
            self.bar_top_pause_secs = MPRIS_MARQUEE_PAUSE_SECS;
            self.popup_sub_scroll_px = 0.0;
            self.popup_sub_pause_secs = MPRIS_MARQUEE_PAUSE_SECS;
        }

        if bot_raw != self.bar_bot_text {
            self.bar_bot_text = bot_raw;
            if self.bar_bot_text.is_empty() {
                self.bar_bot_width_px = 0.0;
                self.popup_title_width_px = 0.0;
            } else {
                let bar_opts =
                    TextLayoutOptions::new(font_ui_semibold(), 10.0.logical_px()).line_height(1.05);
                self.bar_bot_width_px = ctx
                    .measure_text_cursor_with(&self.bar_bot_text, &bar_opts, self.bar_bot_text.len())
                    .size
                    .width
                    .get();
                let popup_opts = TextLayoutOptions::new(font_ui_bold(), 11.5.logical_px());
                self.popup_title_width_px = ctx
                    .measure_text_cursor_with(
                        &self.bar_bot_text,
                        &popup_opts,
                        self.bar_bot_text.len(),
                    )
                    .size
                    .width
                    .get();
            }
            self.bar_bot_scroll_px = 0.0;
            self.bar_bot_pause_secs = MPRIS_MARQUEE_PAUSE_SECS;
            self.popup_title_scroll_px = 0.0;
            self.popup_title_pause_secs = MPRIS_MARQUEE_PAUSE_SECS;
        }

        if !self.is_animating_bar() {
            self.last_bar_anim_tick = None;
        }

        let art_url = self
            .active_player()
            .and_then(|p| p.art_url.clone())
            .unwrap_or_default();

        if art_url == self.art_url_loaded {
            return;
        }
        self.art_url_loaded.clone_from(&art_url);
        self.art_asset = resolve_art_url(&art_url, path_cache, ctx);
        self.disc_angle_rad = 0.0;
    }
}

fn step_pixel_marquee(
    width_px: f32,
    box_w: f32,
    scroll_px: &mut f32,
    pause_secs: &mut f32,
    dt: f32,
) {
    if width_px <= box_w + 0.5 {
        *scroll_px = 0.0;
        *pause_secs = MPRIS_MARQUEE_PAUSE_SECS;
        return;
    }
    if *pause_secs > 0.0 {
        *pause_secs = (*pause_secs - dt).max(0.0);
        return;
    }
    let cycle = width_px + MPRIS_MARQUEE_GAP_PX;
    let next = *scroll_px + dt * MPRIS_MARQUEE_SPEED_PX_S;
    if next >= cycle {
        *scroll_px = 0.0;
        *pause_secs = MPRIS_MARQUEE_PAUSE_SECS;
    } else {
        *scroll_px = next;
    }
}

fn resolve_art_url(
    art_url: &str,
    path_cache: &mut PathAssetCache,
    ctx: &mut EffectCtx<'_, crate::app::msg::Msg>,
) -> Option<ImageAsset> {
    if art_url.is_empty() {
        return None;
    }
    let path_str = art_url.strip_prefix("file://").unwrap_or(art_url);
    if !Path::new(path_str).is_absolute() || !Path::new(path_str).exists() {
        return None;
    }
    let sig = format!("mpris-art:{path_str}");
    if let Some(&cached) = path_cache.get(&sig) {
        return Some(cached);
    }
    if let Ok(asset) = ctx.load_image_sized(Path::new(path_str), 64) {
        path_cache.insert(sig, asset);
        return Some(asset);
    }
    None
}
