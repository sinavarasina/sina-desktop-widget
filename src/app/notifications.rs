//! Notification center state, RAM-bounded lazy cache, and incoming ticker animation.

pub mod icons;
pub mod sound;
pub mod storage;
pub mod ticker;

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Instant, UNIX_EPOCH};

use waywi::desktop::icons::XdgIconLookup;
use waywi::prelude::*;

use crate::app::anim::RevealAnim;
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
    /// Strictly unique UI/interaction ID across persisted + live sessions.
    pub uid: u32,
    /// `true` for historical notifications loaded from disk (previous sessions).
    pub from_disk: bool,
    pub stored: StoredNotification,
    pub icon: Option<ImageAsset>,
    pub icon_loaded: bool,
}

const DISK_UID_BASE: u32 = 0x8000_0000;
const LIVE_UID_FALLBACK_BASE: u32 = 0x4000_0000;

pub struct NotificationStore {
    pub items: Vec<NotificationItem>,
    pub total_count: usize,
    pub dnd: bool,
    pub center_anim: RevealAnim,
    pub marquee_tick: u32,
    pub scroll_index: usize,
    pub action_scroll_x: HashMap<u32, f32>,
    pub active_ticker: Option<IncomingTicker>,
    pub last_sound_plan: NotificationSoundPlan,
    store_path: PathBuf,
    disk_overflow: Vec<StoredNotification>,
    next_disk_uid: u32,
    next_live_uid: u32,
}

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
        let mut next_disk_uid = DISK_UID_BASE;

        for (idx, stored) in persisted.items.into_iter().enumerate() {
            if idx < MAX_RAM_NOTIFICATIONS {
                let uid = next_disk_uid;
                next_disk_uid = next_disk_uid.wrapping_add(1);
                in_ram.push(NotificationItem {
                    uid,
                    from_disk: true,
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
            center_anim: RevealAnim::popup(),
            marquee_tick: 0,
            scroll_index: 0,
            action_scroll_x: HashMap::new(),
            active_ticker: None,
            last_sound_plan: NotificationSoundPlan::Silent,
            store_path: path,
            disk_overflow: overflow,
            next_disk_uid,
            next_live_uid: LIVE_UID_FALLBACK_BASE,
        }
    }

    fn allocate_live_uid(&mut self, preferred: u32) -> u32 {
        if preferred != 0 && !self.items.iter().any(|i| i.uid == preferred) {
            return preferred;
        }
        loop {
            let candidate = self.next_live_uid;
            self.next_live_uid = self.next_live_uid.wrapping_add(1);
            if candidate != 0 && !self.items.iter().any(|i| i.uid == candidate) {
                return candidate;
            }
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
            self.items
                .retain(|i| i.from_disk || i.stored.id != notif.replaces_id);
        }

        let uid = self.allocate_live_uid(stored.id);
        self.items.insert(
            0,
            NotificationItem {
                uid,
                from_disk: false,
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
                let uid = self.next_disk_uid;
                self.next_disk_uid = self.next_disk_uid.wrapping_add(1);
                self.items.push(NotificationItem {
                    uid,
                    from_disk: true,
                    stored,
                    icon: None,
                    icon_loaded: false,
                });
            }
        }
    }

    /// Dismisses exactly one card by its unique UI `uid`.
    /// Returns `Some(live_dbus_id)` when the dismissed card belonged to the current D-Bus session.
    pub fn dismiss_uid(&mut self, uid: u32) -> Option<Option<u32>> {
        let pos = self.items.iter().position(|i| i.uid == uid)?;
        let removed = self.items.remove(pos);
        self.total_count = self.items.len() + self.disk_overflow.len();
        self.action_scroll_x.remove(&uid);
        if self.scroll_index >= self.items.len() && self.scroll_index > 0 {
            self.scroll_index = self.items.len().saturating_sub(1);
        }
        if let Some(p) = removed.stored.cached_icon_path {
            self.remove_icon_if_unreferenced(&p);
        }
        self.persist();
        Some((!removed.from_disk).then_some(removed.stored.id))
    }

    /// Dismisses a live current-session notification by D-Bus `id` (e.g. `ClosedByCaller`).
    pub fn dismiss(&mut self, id: u32) -> bool {
        if let Some(pos) = self
            .items
            .iter()
            .position(|i| !i.from_disk && i.stored.id == id)
        {
            let uid = self.items[pos].uid;
            return self.dismiss_uid(uid).is_some();
        }
        if let Some(pos) = self.items.iter().position(|i| i.uid == id) {
            let uid = self.items[pos].uid;
            return self.dismiss_uid(uid).is_some();
        }
        false
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
    pub const fn is_center_open(&self) -> bool {
        self.center_anim.is_open()
    }

    #[must_use]
    pub fn center_progress(&self) -> f32 {
        self.center_anim.progress()
    }

    #[must_use]
    pub fn is_center_visible(&self) -> bool {
        self.center_anim.is_visible()
    }

    #[must_use]
    pub fn is_animating_center(&self) -> bool {
        self.center_anim.is_animating()
    }

    pub fn advance_center_animation(&mut self, now: Instant) {
        self.center_anim.advance(now);
    }

    pub fn open_center(&mut self) {
        self.center_anim.set_open(true);
        self.marquee_tick = 0;
    }

    pub fn toggle_center(&mut self) -> bool {
        let open = self.center_anim.toggle();
        if open {
            self.marquee_tick = 0;
        }
        open
    }

    pub fn close_center(&mut self) {
        self.center_anim.set_open(false);
    }

    pub fn advance_marquee(&mut self) {
        self.marquee_tick = self.marquee_tick.wrapping_add(1);
    }

    #[must_use]
    pub fn is_resident(&self, uid: u32) -> bool {
        self.items
            .iter()
            .find(|i| i.uid == uid)
            .is_some_and(|i| i.stored.resident)
    }

    #[must_use]
    pub fn dbus_id_for_uid(&self, uid: u32) -> Option<u32> {
        self.items
            .iter()
            .find(|i| i.uid == uid && !i.from_disk)
            .map(|i| i.stored.id)
    }

    pub fn take_action_and_maybe_dismiss(
        &mut self,
        uid: u32,
    ) -> Option<(Option<u32>, Option<String>, bool)> {
        let item = self.items.iter().find(|i| i.uid == uid)?;
        let action_key = item.stored.actions.first().map(|(k, _)| k.clone());
        let resident = item.stored.resident;
        let live_dbus_id = (!item.from_disk).then_some(item.stored.id);
        if resident && action_key.is_some() {
            return Some((live_dbus_id, action_key, true));
        }
        self.dismiss_uid(uid)
            .map(|dbus_id| (dbus_id, action_key, false))
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
