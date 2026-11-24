//! Notification center state, RAM-bounded lazy cache, and incoming ticker animation.

pub mod icons;
pub mod sound;
pub mod storage;
pub mod ticker;

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::UNIX_EPOCH;

use waywi::desktop::icons::XdgIconLookup;
use waywi::prelude::*;

use crate::app::asset_lru::PathAssetCache;

use icons::resolve_notification_icon;
pub use sound::{NotificationSoundPlan, play_notification_sound, resolve_notification_sound};
use storage::{
    MAX_DISK_NOTIFICATIONS, MAX_RAM_NOTIFICATIONS, PersistedStore, StoredNotification,
    clear_icon_cache, default_store_path, load_store_from_disk, save_cached_icon,
    save_store_to_disk,
};
pub use ticker::IncomingTicker;

#[derive(Debug, Clone)]
pub struct NotificationItem {
    pub stored: StoredNotification,
    pub icon: Option<ImageAsset>,
    pub icon_loaded: bool,
}

pub struct NotificationStore {
    pub items: Vec<NotificationItem>,
    pub total_count: usize,
    pub dnd: bool,
    pub center_target: bool,
    pub center_progress: f32,
    pub marquee_tick: u32,
    pub scroll_index: usize,
    pub action_scroll_x: HashMap<u32, f32>,
    pub active_ticker: Option<IncomingTicker>,
    pub last_sound_plan: NotificationSoundPlan,
    store_path: PathBuf,
    disk_overflow: Vec<StoredNotification>,
}

const CENTER_ANIM_STEP: f32 = 16.0 / 180.0;

impl NotificationStore {
    /// Loads persisted notification metadata from disk without decoding icons upfront so the
    /// initial bar frame renders immediately; bar icons decode lazily via `ensure_visible_icons`.
    #[must_use]
    pub fn load_lazy(store_path: Option<PathBuf>) -> Self {
        let path = store_path.unwrap_or_else(default_store_path);
        let persisted = load_store_from_disk(&path);
        let total_count = persisted.items.len();

        let mut in_ram = Vec::with_capacity(MAX_RAM_NOTIFICATIONS.min(total_count));
        let mut overflow = Vec::new();

        for (idx, stored) in persisted.items.into_iter().enumerate() {
            if idx < MAX_RAM_NOTIFICATIONS {
                in_ram.push(NotificationItem {
                    stored,
                    icon: None,
                    icon_loaded: false,
                });
            } else {
                overflow.push(stored);
            }
        }

        Self {
            items: in_ram,
            total_count,
            dnd: persisted.dnd,
            center_target: false,
            center_progress: 0.0,
            marquee_tick: 0,
            scroll_index: 0,
            action_scroll_x: HashMap::new(),
            active_ticker: None,
            last_sound_plan: NotificationSoundPlan::Silent,
            store_path: path,
            disk_overflow: overflow,
        }
    }

    pub fn push_incoming(
        &mut self,
        notif: DesktopNotification,
        icon_lookup: &mut XdgIconLookup,
        path_cache: &mut PathAssetCache,
        sounds: &mut crate::app::sounds::SoundEngine,
        ctx: &mut EffectCtx<'_, crate::app::msg::Msg>,
    ) {
        let sound_plan = resolve_notification_sound(&notif, self.dnd);
        play_notification_sound(&sound_plan, sounds, ctx);
        self.last_sound_plan = sound_plan;

        let cached_icon_path = if notif.transient {
            None
        } else {
            notif
                .image_data
                .as_ref()
                .and_then(|raw| save_cached_icon(&self.store_path, raw))
        };

        let icon = resolve_notification_icon(
            &notif.app_icon,
            notif.desktop_entry.as_deref(),
            notif.image_path.as_deref(),
            cached_icon_path.as_deref(),
            notif.image_data.as_ref(),
            icon_lookup,
            path_cache,
            ctx,
        );

        if !self.dnd {
            self.active_ticker = Some(IncomingTicker::from_notification(
                &notif.summary,
                &notif.body,
                icon,
                notif.id,
            ));
        }

        // Per XDG Desktop Notifications v1.2, transient notifications bypass persistence/history.
        if notif.transient {
            return;
        }

        let epoch = notif
            .received_at
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());

        let stored = StoredNotification {
            id: notif.id,
            app_name: notif.app_name,
            app_icon: notif.app_icon,
            desktop_entry: notif.desktop_entry,
            image_path: notif.image_path,
            cached_icon_path,
            summary: notif.summary,
            body: notif.body,
            actions: notif.actions,
            urgency: notif.urgency as u8,
            resident: notif.resident,
            received_epoch_secs: epoch,
            read: false,
        };

        if notif.replaces_id != 0 {
            self.items.retain(|i| i.stored.id != notif.replaces_id);
            self.disk_overflow.retain(|i| i.id != notif.replaces_id);
        }

        self.items.insert(
            0,
            NotificationItem {
                stored,
                icon,
                icon_loaded: true,
            },
        );

        while self.items.len() > MAX_RAM_NOTIFICATIONS {
            if let Some(evicted) = self.items.pop() {
                self.disk_overflow.insert(0, evicted.stored);
            }
        }
        if self.items.len() + self.disk_overflow.len() > MAX_DISK_NOTIFICATIONS {
            let max_overflow = MAX_DISK_NOTIFICATIONS.saturating_sub(self.items.len());
            let evicted: Vec<String> = self
                .disk_overflow
                .drain(max_overflow..)
                .filter_map(|s| s.cached_icon_path)
                .collect();
            for p in evicted {
                self.remove_icon_if_unreferenced(&p);
            }
        }
        self.total_count = self.items.len() + self.disk_overflow.len();
        self.persist();
    }

    pub fn ensure_visible_icons(
        &mut self,
        start: usize,
        count: usize,
        icon_lookup: &mut XdgIconLookup,
        path_cache: &mut PathAssetCache,
        ctx: &mut EffectCtx<'_, crate::app::msg::Msg>,
    ) {
        let end = (start + count).min(self.items.len());
        for item in &mut self.items[start.min(end)..end] {
            if !item.icon_loaded {
                item.icon = resolve_notification_icon(
                    &item.stored.app_icon,
                    item.stored.desktop_entry.as_deref(),
                    item.stored.image_path.as_deref(),
                    item.stored.cached_icon_path.as_deref(),
                    None,
                    icon_lookup,
                    path_cache,
                    ctx,
                );
                item.icon_loaded = true;
            }
        }
    }

    /// Lazily pulls the next batch from disk overflow into RAM when scrolling near the end.
    pub fn maybe_load_more_from_disk(&mut self, visible_end: usize) {
        if visible_end + 4 >= self.items.len() && !self.disk_overflow.is_empty() {
            let take_n = 20.min(self.disk_overflow.len());
            for stored in self.disk_overflow.drain(0..take_n) {
                self.items.push(NotificationItem {
                    stored,
                    icon: None,
                    icon_loaded: false,
                });
            }
        }
    }

    pub fn dismiss(&mut self, id: u32) -> bool {
        let mut removed_icons = Vec::new();
        self.items.retain(|i| {
            if i.stored.id == id {
                if let Some(p) = &i.stored.cached_icon_path {
                    removed_icons.push(p.clone());
                }
                false
            } else {
                true
            }
        });
        self.disk_overflow.retain(|i| {
            if i.id == id {
                if let Some(p) = &i.cached_icon_path {
                    removed_icons.push(p.clone());
                }
                false
            } else {
                true
            }
        });
        let after = self.items.len() + self.disk_overflow.len();
        let changed = after != self.total_count;
        self.total_count = after;
        self.action_scroll_x.remove(&id);
        if self.scroll_index >= self.items.len() && self.scroll_index > 0 {
            self.scroll_index = self.items.len().saturating_sub(1);
        }
        if changed {
            for p in removed_icons {
                self.remove_icon_if_unreferenced(&p);
            }
            self.persist();
            true
        } else {
            false
        }
    }

    fn remove_icon_if_unreferenced(&self, path: &str) {
        let still_used = self
            .items
            .iter()
            .any(|i| i.stored.cached_icon_path.as_deref() == Some(path))
            || self
                .disk_overflow
                .iter()
                .any(|i| i.cached_icon_path.as_deref() == Some(path));
        if !still_used {
            let _ = std::fs::remove_file(path);
        }
    }

    #[must_use]
    pub fn is_center_visible(&self) -> bool {
        self.center_target || self.center_progress > 0.001
    }

    #[must_use]
    pub fn is_animating_center(&self) -> bool {
        let target = if self.center_target { 1.0 } else { 0.0 };
        (self.center_progress - target).abs() > 0.001
    }

    pub fn step_center_animation(&mut self) {
        let target = if self.center_target { 1.0_f32 } else { 0.0_f32 };
        if (self.center_progress - target).abs() <= CENTER_ANIM_STEP {
            self.center_progress = target;
        } else if self.center_progress < target {
            self.center_progress = (self.center_progress + CENTER_ANIM_STEP).min(1.0);
        } else {
            self.center_progress = (self.center_progress - CENTER_ANIM_STEP).max(0.0);
        }
    }

    pub fn toggle_center(&mut self) -> bool {
        self.center_target = !self.center_target;
        if self.center_target {
            self.marquee_tick = 0;
        }
        self.center_target
    }

    pub fn close_center(&mut self) {
        self.center_target = false;
    }

    pub fn advance_marquee(&mut self) {
        self.marquee_tick = self.marquee_tick.wrapping_add(1);
    }

    #[must_use]
    pub fn is_resident(&self, id: u32) -> bool {
        self.items
            .iter()
            .find(|i| i.stored.id == id)
            .is_some_and(|i| i.stored.resident)
    }

    pub fn take_action_and_maybe_dismiss(&mut self, id: u32) -> Option<(u32, Option<String>, bool)> {
        let item = self.items.iter().find(|i| i.stored.id == id)?;
        let action_key = item.stored.actions.first().map(|(k, _)| k.clone());
        let resident = item.stored.resident;
        if resident && action_key.is_some() {
            return Some((id, action_key, true));
        }
        if self.dismiss(id) {
            Some((id, action_key, false))
        } else {
            None
        }
    }

    pub fn clear_all(&mut self) {
        self.items.clear();
        self.disk_overflow.clear();
        self.total_count = 0;
        self.scroll_index = 0;
        self.action_scroll_x.clear();
        clear_icon_cache(&self.store_path);
        self.persist();
    }

    pub fn toggle_dnd(&mut self, ctx: &EffectCtx<'_, crate::app::msg::Msg>) {
        self.dnd = !self.dnd;
        if self.dnd {
            self.active_ticker = None;
            if let Ok(player) = ctx.player() {
                player.stop_all();
            }
        }
        self.persist();
    }

    pub fn step_ticker(&mut self) {
        if let Some(ticker) = &mut self.active_ticker
            && !ticker.step()
        {
            self.active_ticker = None;
        }
    }

    fn persist(&self) {
        let mut all = Vec::with_capacity(self.total_count);
        for item in &self.items {
            all.push(item.stored.clone());
        }
        all.extend(self.disk_overflow.iter().cloned());
        save_store_to_disk(
            &self.store_path,
            &PersistedStore {
                dnd: self.dnd,
                items: all,
            },
        );
    }
}
