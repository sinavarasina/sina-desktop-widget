//! Notification sound resolution and playback honoring DND and XDG Notifications v1.2 hints
//! (`suppress-sound`, `sound-file`, `sound-name`, `category`, `urgency`).

use waywi::prelude::*;

use crate::app::sounds::SoundEngine;

const NOTIF_SOUND_VOLUME: f32 = 0.85;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum NotificationSoundPlan {
    #[default]
    Silent,
    File(String),
    CustomThemeName {
        name: String,
        fallback_preset: SoundPreset,
    },
    ThemeName {
        primary: &'static str,
        fallback_theme: Option<&'static str>,
        fallback_preset: SoundPreset,
    },
}

/// Decides which sound (if any) should play for an incoming notification.
///
/// - When `dnd` is enabled, ALL notification sounds are unconditionally silenced.
/// - When `notif.suppress_sound` (`"suppress-sound"` hint) is set, remains silent.
/// - Explicit `"sound-file"` and `"sound-name"` hints take priority over urgency/category defaults.
/// - `NotificationUrgency::Low` notifications stay silent unless they explicitly specify a sound.
#[must_use]
pub fn resolve_notification_sound(
    notif: &DesktopNotification,
    dnd: bool,
) -> NotificationSoundPlan {
    if dnd || notif.suppress_sound {
        return NotificationSoundPlan::Silent;
    }

    if let Some(path) = notif
        .sound_file
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return NotificationSoundPlan::File(path.to_owned());
    }

    let fallback_preset = preset_for_urgency(notif.urgency);
    if let Some(name) = notif
        .sound_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return NotificationSoundPlan::CustomThemeName {
            name: name.to_owned(),
            fallback_preset,
        };
    }

    match notif.urgency {
        NotificationUrgency::Low => NotificationSoundPlan::Silent,
        NotificationUrgency::Critical => NotificationSoundPlan::ThemeName {
            primary: "dialog-warning",
            fallback_theme: Some("bell"),
            fallback_preset: SoundPreset::Warning,
        },
        NotificationUrgency::Normal => match notif.category.as_deref() {
            Some(cat) if cat.starts_with("email") => NotificationSoundPlan::ThemeName {
                primary: "message-new-email",
                fallback_theme: Some("message-new-instant"),
                fallback_preset: SoundPreset::Notification,
            },
            Some(cat) if cat.starts_with("im") => NotificationSoundPlan::ThemeName {
                primary: "message-new-instant",
                fallback_theme: Some("message"),
                fallback_preset: SoundPreset::Notification,
            },
            Some("device.added") => NotificationSoundPlan::ThemeName {
                primary: "device-added",
                fallback_theme: Some("bell"),
                fallback_preset: SoundPreset::Notification,
            },
            Some("device.removed") => NotificationSoundPlan::ThemeName {
                primary: "device-removed",
                fallback_theme: Some("bell"),
                fallback_preset: SoundPreset::Notification,
            },
            Some("device.error") => NotificationSoundPlan::ThemeName {
                primary: "dialog-error",
                fallback_theme: Some("error"),
                fallback_preset: SoundPreset::Warning,
            },
            Some("network.connected") => NotificationSoundPlan::ThemeName {
                primary: "network-connectivity-established",
                fallback_theme: Some("message"),
                fallback_preset: SoundPreset::Notification,
            },
            Some("network.disconnected") => NotificationSoundPlan::ThemeName {
                primary: "network-connectivity-lost",
                fallback_theme: Some("dialog-warning"),
                fallback_preset: SoundPreset::Warning,
            },
            Some("network.error") => NotificationSoundPlan::ThemeName {
                primary: "network-connectivity-error",
                fallback_theme: Some("dialog-error"),
                fallback_preset: SoundPreset::Warning,
            },
            Some("transfer.complete") => NotificationSoundPlan::ThemeName {
                primary: "complete-download",
                fallback_theme: Some("complete"),
                fallback_preset: SoundPreset::Notification,
            },
            Some("transfer.error") => NotificationSoundPlan::ThemeName {
                primary: "dialog-error",
                fallback_theme: Some("error"),
                fallback_preset: SoundPreset::Warning,
            },
            Some(cat) if cat.starts_with("battery") || cat.starts_with("power") => {
                NotificationSoundPlan::ThemeName {
                    primary: "battery-low",
                    fallback_theme: Some("dialog-warning"),
                    fallback_preset: SoundPreset::Warning,
                }
            }
            _ => NotificationSoundPlan::ThemeName {
                primary: "message",
                fallback_theme: Some("bell"),
                fallback_preset: SoundPreset::Notification,
            },
        },
    }
}

#[must_use]
const fn preset_for_urgency(urgency: NotificationUrgency) -> SoundPreset {
    match urgency {
        NotificationUrgency::Critical => SoundPreset::Warning,
        NotificationUrgency::Low | NotificationUrgency::Normal => SoundPreset::Notification,
    }
}

/// Plays a resolved [`NotificationSoundPlan`] through the O(1)-cached [`SoundEngine`] and WayWi's
/// native `AudioPlayer`.
#[cfg(not(test))]
pub fn play_notification_sound(
    plan: &NotificationSoundPlan,
    sounds: &mut SoundEngine,
    ctx: &EffectCtx<'_>,
) {
    if !sounds.event_sounds_enabled || !sounds.notification_sounds_enabled {
        return;
    }
    let vol = (NOTIF_SOUND_VOLUME * sounds.volume_scale).clamp(0.0, 1.0);
    match plan {
        NotificationSoundPlan::Silent => {}
        NotificationSoundPlan::File(path) => {
            if let Ok(asset) = ctx.load_sound(path) {
                ctx.play_sound_volume(asset, vol);
            } else {
                ctx.play_sound_preset(SoundPreset::Notification, vol);
            }
        }
        NotificationSoundPlan::CustomThemeName {
            name,
            fallback_preset,
        } => {
            if !sounds.play_theme_name_cached(name, vol, ctx) {
                ctx.play_sound_preset(*fallback_preset, vol);
            }
        }
        NotificationSoundPlan::ThemeName {
            primary,
            fallback_theme,
            fallback_preset,
        } => {
            if sounds.play_theme_name_cached(primary, vol, ctx) {
                return;
            }
            if let Some(fb) = fallback_theme
                && sounds.play_theme_name_cached(fb, vol, ctx)
            {
                return;
            }
            ctx.play_sound_preset(*fallback_preset, vol);
        }
    }
}

#[cfg(test)]
pub fn play_notification_sound(
    _plan: &NotificationSoundPlan,
    _sounds: &mut SoundEngine,
    _ctx: &EffectCtx<'_>,
) {
    let _ = NOTIF_SOUND_VOLUME;
}
