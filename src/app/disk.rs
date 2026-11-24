//! UDisks2 disk management popup state, LUKS passphrase input, and device menu actions.

use std::collections::HashMap;

use waywi::prelude::*;
use waywi::dbus::UDisksBlockDevice;

use crate::theme::font_ui;

const DISK_POPUP_ANIM_STEP: f32 = 16.0 / 180.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiskMenuAction {
    Mount,
    Unmount { force: bool },
    UnlockPrompt,
    Lock,
    Eject,
    PowerOff,
}

#[derive(Default)]
pub struct DiskUiState {
    pub popup_target: bool,
    pub popup_progress: f32,
    pub popup_left_x: f32,
    pub scroll_index: usize,
    /// Whether OS system partitions (`/`, `/boot`, EFI, etc.) are shown in a separate section.
    pub show_system: bool,
    /// Currently cycled mount index for the drawer Disk metric badge (middle-click).
    pub selected_mount_idx: usize,
    /// Object path of the block device whose context menu is expanded in the popup.
    pub menu_path: Option<String>,
    /// Object path of the LUKS encrypted block device currently awaiting a passphrase.
    pub unlock_target: Option<String>,
    pub unlock_label: String,
    pub unlock_input: TextInputState,
    pub unlock_cursor_x: f32,
    /// Active D-Bus operation per block or drive object path (`"mounting"`, `"unmounting"`, etc.).
    pub pending_ops: HashMap<String, &'static str>,
    pub anim_tick: u32,
    pub marquee_tick: u32,
    pub last_error: Option<String>,
}

impl DiskUiState {
    #[must_use]
    pub fn is_popup_visible(&self) -> bool {
        self.popup_target || self.popup_progress > 0.001
    }

    #[must_use]
    pub fn is_animating_popup(&self) -> bool {
        let target = if self.popup_target { 1.0 } else { 0.0 };
        (self.popup_progress - target).abs() > 0.001
    }

    #[must_use]
    pub fn has_pending_ops(&self) -> bool {
        !self.pending_ops.is_empty()
    }

    pub fn advance_pending_tick(&mut self) {
        self.anim_tick = self.anim_tick.wrapping_add(1);
    }

    pub fn advance_marquee(&mut self) {
        self.marquee_tick = self.marquee_tick.wrapping_add(1);
    }

    pub fn toggle_show_system(&mut self) {
        self.show_system = !self.show_system;
        self.menu_path = None;
    }

    #[must_use]
    pub fn pending_dots(&self) -> &'static str {
        match self.anim_tick % 4 {
            0 => ".",
            1 => "..",
            2 => "...",
            _ => "",
        }
    }

    pub fn set_pending(&mut self, object_path: String, label: &'static str) {
        self.pending_ops.insert(object_path, label);
    }

    pub fn clear_settled_pending(&mut self, snap: &UDisksSnapshot) {
        self.pending_ops.retain(|path, op| {
            if let Some(block) = snap.block_devices.iter().find(|b| b.object_path == *path) {
                match *op {
                    "mounting" => !block.is_mounted(),
                    "unmounting" => block.is_mounted(),
                    "unlocking" => !block.is_unlocked(),
                    "locking" => block.is_unlocked(),
                    _ => true,
                }
            } else if let Some(drive) = snap.drives.iter().find(|d| d.object_path == *path) {
                match *op {
                    "ejecting" => drive.media_available,
                    "powering off" => true,
                    _ => false,
                }
            } else {
                false
            }
        });
    }

    pub fn step_popup_animation(&mut self) {
        let target = if self.popup_target { 1.0_f32 } else { 0.0_f32 };
        if (self.popup_progress - target).abs() <= DISK_POPUP_ANIM_STEP {
            self.popup_progress = target;
        } else if self.popup_progress < target {
            self.popup_progress = (self.popup_progress + DISK_POPUP_ANIM_STEP).min(1.0);
        } else {
            self.popup_progress = (self.popup_progress - DISK_POPUP_ANIM_STEP).max(0.0);
        }
    }

    pub fn toggle_popup(&mut self) {
        self.popup_target = !self.popup_target;
        self.last_error = None;
        self.marquee_tick = 0;
        if !self.popup_target {
            self.menu_path = None;
            self.close_unlock_prompt();
        }
    }

    pub fn close_popup(&mut self) {
        self.popup_target = false;
        self.menu_path = None;
        self.close_unlock_prompt();
    }

    pub fn open_unlock_prompt(&mut self, block_path: String, label: String) {
        self.unlock_target = Some(block_path);
        self.unlock_label = label;
        self.unlock_input.clear();
        self.unlock_cursor_x = 0.0;
        self.last_error = None;
    }

    pub fn close_unlock_prompt(&mut self) {
        self.unlock_target = None;
        self.unlock_label.clear();
        self.unlock_input.clear();
        self.unlock_cursor_x = 0.0;
    }

    pub fn remeasure_unlock_input(&mut self, ctx: &mut EffectCtx<'_, crate::app::msg::Msg>) {
        let masked = self.masked_unlock_text();
        let char_count = self
            .unlock_input
            .display_text()
            .get(..self.unlock_input.display_cursor_bytes())
            .map_or(0, |s| s.chars().count());
        let cursor_byte = "•".len() * char_count;
        let metrics = ctx.measure_text_cursor(&masked, &font_ui(), 10.5.logical_px(), cursor_byte);
        self.unlock_cursor_x = metrics.cursor_x.get();
    }

    #[must_use]
    pub fn masked_unlock_text(&self) -> String {
        let count = self.unlock_input.display_text().chars().count();
        "•".repeat(count)
    }

    /// Returns user/data/removable UDisks2 block devices (excluding core OS system partitions),
    /// sorted with removable/USB drives first, then mounted volumes, then device name.
    #[must_use]
    pub fn user_blocks<'a>(snap: &'a UDisksSnapshot) -> Vec<&'a UDisksBlockDevice> {
        let mut list: Vec<&'a UDisksBlockDevice> = snap
            .block_devices
            .iter()
            .filter(|b| is_user_relevant_block(b) && !is_system_block(snap, b))
            .collect();

        list.sort_by(|a, b| {
            let a_rem = snap
                .drive_for_block(a)
                .is_some_and(|d| d.removable || d.is_usb());
            let b_rem = snap
                .drive_for_block(b)
                .is_some_and(|d| d.removable || d.is_usb());
            b_rem
                .cmp(&a_rem)
                .then_with(|| b.is_mounted().cmp(&a.is_mounted()))
                .then_with(|| a.device.cmp(&b.device))
        });
        list
    }

    /// Returns core OS system partitions (`/`, `/boot`, EFI, etc.) kept separate from user drives.
    #[must_use]
    pub fn system_blocks<'a>(snap: &'a UDisksSnapshot) -> Vec<&'a UDisksBlockDevice> {
        let mut list: Vec<&'a UDisksBlockDevice> = snap
            .block_devices
            .iter()
            .filter(|b| is_user_relevant_block(b) && is_system_block(snap, b))
            .collect();

        list.sort_by(|a, b| a.device.cmp(&b.device));
        list
    }
}

fn is_user_relevant_block(b: &UDisksBlockDevice) -> bool {
    if b.hint_ignore {
        return false;
    }
    if !(b.has_filesystem || b.is_encrypted) {
        return false;
    }
    if b.device.starts_with("/dev/loop")
        || b.device.starts_with("/dev/ram")
        || b.device.starts_with("/dev/zram")
    {
        return false;
    }
    if b.size_bytes == 0 && !b.is_mounted() {
        return false;
    }
    true
}

#[must_use]
pub fn is_system_block(snap: &UDisksSnapshot, b: &UDisksBlockDevice) -> bool {
    // Removable or USB drives are always treated as user drives, never hidden as OS system disks.
    if snap
        .drive_for_block(b)
        .is_some_and(|d| d.removable || d.is_usb())
    {
        return false;
    }

    for mp in &b.mount_points {
        if matches!(
            mp.as_str(),
            "/" | "/boot"
                | "/boot/efi"
                | "/efi"
                | "/usr"
                | "/var"
                | "/nix"
                | "/opt"
                | "/etc"
                | "/sysroot"
        ) || mp.starts_with("/boot/")
            || mp.starts_with("/var/")
            || mp.starts_with("/snap/")
        {
            return true;
        }
    }

    if !b.is_mounted() && b.hint_system {
        if b.id_type.eq_ignore_ascii_case("swap") {
            return true;
        }
        let label = b.id_label.trim();
        if label.eq_ignore_ascii_case("root")
            || label.eq_ignore_ascii_case("boot")
            || label.eq_ignore_ascii_case("efi")
            || label.eq_ignore_ascii_case("esp")
            || label.eq_ignore_ascii_case("recovery")
        {
            return true;
        }
        // Small unmounted internal FAT partitions (<= 2 GiB) are typically EFI system partitions.
        if b.id_type.eq_ignore_ascii_case("vfat") && b.size_bytes <= 2_147_483_648 {
            return true;
        }
    }

    false
}
