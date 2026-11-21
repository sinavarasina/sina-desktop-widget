//! Notification sound resolution and playback honoring DND and XDG Notifications v1.2 hints
//! (`suppress-sound`, `sound-file`, `sound-name`, `category`, `urgency`).

use waywi::prelude::*;

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
            Some(cat) if cat.starts_with("im") || cat.starts_with("email") => {
                NotificationSoundPlan::ThemeName {
                    primary: "message-new-instant",
                    fallback_theme: Some("message"),
                    fallback_preset: SoundPreset::Notification,
                }
            }
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

/// Plays a resolved [`NotificationSoundPlan`] through WayWi's native `AudioPlayer`.
#[cfg(not(test))]
pub fn play_notification_sound(plan: &NotificationSoundPlan, ctx: &EffectCtx<'_>) {
    match plan {
        NotificationSoundPlan::Silent => {}
        NotificationSoundPlan::File(path) => {
            if let Ok(asset) = ctx.load_sound(path) {
                ctx.play_sound_volume(asset, NOTIF_SOUND_VOLUME);
            } else {
                ctx.play_sound_preset(SoundPreset::Notification, NOTIF_SOUND_VOLUME);
            }
        }
        NotificationSoundPlan::CustomThemeName {
            name,
            fallback_preset,
        } => {
            if ctx.play_system_sound(name, NOTIF_SOUND_VOLUME).is_err() {
                ctx.play_sound_preset(*fallback_preset, NOTIF_SOUND_VOLUME);
            }
        }
        NotificationSoundPlan::ThemeName {
            primary,
            fallback_theme,
            fallback_preset,
        } => {
            if ctx.play_system_sound(primary, NOTIF_SOUND_VOLUME).is_ok() {
                return;
            }
            if let Some(fb) = fallback_theme
                && ctx.play_system_sound(fb, NOTIF_SOUND_VOLUME).is_ok()
            {
                return;
            }
            ctx.play_sound_preset(*fallback_preset, NOTIF_SOUND_VOLUME);
        }
    }
}

#[cfg(test)]
pub fn play_notification_sound(_plan: &NotificationSoundPlan, _ctx: &EffectCtx<'_>) {
    let _ = NOTIF_SOUND_VOLUME;
}
