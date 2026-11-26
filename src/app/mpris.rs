//! MPRIS session state, active player selection, album art loading, and marquee scrolling.

use std::path::Path;
use std::time::Instant;

use crate::app::anim::RevealAnim;
use crate::app::asset_lru::PathAssetCache;
use waywi::prelude::*;

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
    pub fn is_animating_popup(&self) -> bool {
        self.popup_anim.is_animating()
    }

    pub fn advance_popup_animation(&mut self, now: Instant) {
        self.popup_anim.advance(now);
    }

    pub fn open_popup(&mut self) {
        self.popup_anim.set_open(true);
    }

    pub fn toggle_popup(&mut self) {
        self.popup_anim.toggle();
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

        let art_url = self
            .active_player()
            .and_then(|p| p.art_url.clone())
            .unwrap_or_default();

        if art_url == self.art_url_loaded {
            return;
        }
        self.art_url_loaded.clone_from(&art_url);
        self.art_asset = resolve_art_url(&art_url, path_cache, ctx);
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
