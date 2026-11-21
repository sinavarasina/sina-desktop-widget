//! External Unix control socket command parser for keybind & CLI triggers.

use super::msg::Msg;

pub const CONTROL_SOCKET_NAME: &str = "sina-desktop-widget";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcAction {
    Toggle,
    Open,
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcTarget {
    Drun,
    Notifications,
    Dnd,
    Volume,
    Bluetooth,
    Disk,
    Mpris,
    All,
}

/// Parses a single newline-trimmed command from `Control::socket` into an application `Msg`.
#[must_use]
pub fn parse_ipc_command(raw: &str) -> Option<Msg> {
    let trimmed = raw.trim().to_ascii_lowercase();
    if trimmed.is_empty() {
        return None;
    }
    let cleaned = trimmed.trim_start_matches('-');
    let mut parts = cleaned.split_whitespace();
    let first = parts.next()?;
    let second = parts.next();

    let (action, target_tok) = match (first, second) {
        ("toggle", Some(t)) => (IpcAction::Toggle, t),
        ("open" | "show", Some(t)) => (IpcAction::Open, t),
        ("close" | "hide", Some(t)) => (IpcAction::Close, t),
        ("close" | "hide", None) => (IpcAction::Close, "all"),
        (t, None) => (IpcAction::Toggle, t),
        _ => return None,
    };

    let target = match target_tok {
        "drun" | "launcher" | "app" | "apps" => IpcTarget::Drun,
        "notifications" | "notification" | "notif" | "nc" => IpcTarget::Notifications,
        "dnd" => IpcTarget::Dnd,
        "volume" | "audio" | "mixer" | "vol" => IpcTarget::Volume,
        "bluetooth" | "bt" => IpcTarget::Bluetooth,
        "disk" | "disks" | "udisks" | "storage" => IpcTarget::Disk,
        "mpris" | "media" | "player" => IpcTarget::Mpris,
        "all" => IpcTarget::All,
        _ => return None,
    };

    Some(Msg::IpcControl { action, target })
}
