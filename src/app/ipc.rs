//! External Unix control socket command parser for keybind & CLI triggers.

use super::msg::Msg;
use super::screenshot::ScreenshotMode;

pub const CONTROL_SOCKET_NAME: &str = "sina-desktop-widget";

/// Formatted CLI usage and IPC command reference printed by `sina-desktop-widget help`.
pub const CLI_HELP_TEXT: &str = "\
sina-desktop-widget — Native Wayland desktop bar & widget suite (WayWi)

USAGE:
  sina-desktop-widget                       Start the desktop widget daemon
  sina-desktop-widget [ACTION] <TARGET>     Send an IPC command to the running instance
  sina-desktop-widget help | -h | --help    Show this help message
  sina-desktop-widget version | -V          Show version information

ACTIONS:
  toggle <target>                           Toggle target (default when action is omitted)
  open | show <target>                      Open / enable target
  close | hide [target]                     Close target (defaults to 'all')

WIDGET TARGETS (with aliases):
  drun          (launcher, app, apps)       Application launcher bar
  notifications (notification, notif, nc)   Notification center popup
  dnd                                       Toggle Do Not Disturb
  clipboard     (clip)                      Clipboard history popup
  caffeine      (awake, inhibit)            Idle-inhibit / Keep-Awake toggle
  visualizer    (viz, spectrum)             Audio FFT visualizer toggle
  volume        (audio, mixer, vol)         Audio mixer & routing popup
  bluetooth     (bt)                        Bluetooth device manager popup
  disk          (disks, udisks, storage)    Mounted disks & UDisks2 popup
  mpris         (media, player)             Media player controller popup
  quick         (quickmenu, menu, power)    Quick Menu / Settings / Power drawer
  toolbox       (tools, tool)               Quick Menu Toolbox strip
  calendar      (cal)                       Calendar popup
  all                                       Close all open popups / overlays

SCREEN TOOLS & SUBCOMMANDS:
  annotate      (draw, sketch, paint)       Screen annotation & drawing overlay
  colorpicker   (picker, color, eyedropper) Pixel loupe color picker (HEX/RGB)
  zoom          (magnifier, mag)            Screen magnifier overlay
    zoom in | +                             Zoom in
    zoom out | -                            Zoom out
    zoom reset | center                     Reset zoom & pan
    zoom live                               Toggle Hyprland native live zoom
  screenshot    (shot, ss)                  Interactive screenshot overlay
    screenshot area | region                Area selection capture
    screenshot window | win                 Window capture
    screenshot full | screen | output       Instant full-screen capture

ENVIRONMENT VARIABLES:
  SINA_RENDERER=auto|gpu|software           Override configured renderer backend
  SINA_MEMORY=balanced|low|ultra-low        Override configured memory profile";

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
    Clipboard,
    Caffeine,
    Visualizer,
    Volume,
    Bluetooth,
    Disk,
    Mpris,
    QuickMenu,
    Toolbox,
    Calendar,
    Annotate,
    Zoom,
    ColorPicker,
    Screenshot(Option<ScreenshotMode>),
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

    // Direct zoom subcommands (`zoom in`, `zoom out`, `zoom reset`, `zoom live`).
    if matches!(first, "zoom" | "magnifier" | "mag") {
        match second {
            Some("in" | "+") => return Some(Msg::ZoomStep(1)),
            Some("out" | "-") => return Some(Msg::ZoomStep(-1)),
            Some("reset" | "center") => return Some(Msg::ZoomReset),
            Some("live") => return Some(Msg::ZoomToggleLive),
            _ => {}
        }
    }

    // Direct screenshot mode subcommands (`screenshot area | window | full`).
    if matches!(first, "screenshot" | "shot" | "ss") {
        match second {
            Some("area" | "region") => {
                return Some(Msg::IpcControl {
                    action: IpcAction::Open,
                    target: IpcTarget::Screenshot(Some(ScreenshotMode::Area)),
                });
            }
            Some("window" | "win") => {
                return Some(Msg::IpcControl {
                    action: IpcAction::Open,
                    target: IpcTarget::Screenshot(Some(ScreenshotMode::Window)),
                });
            }
            Some("full" | "screen" | "output") => {
                return Some(Msg::IpcControl {
                    action: IpcAction::Open,
                    target: IpcTarget::Screenshot(Some(ScreenshotMode::Full)),
                });
            }
            _ => {}
        }
    }

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
        "clipboard" | "clip" => IpcTarget::Clipboard,
        "caffeine" | "awake" | "inhibit" => IpcTarget::Caffeine,
        "visualizer" | "viz" | "spectrum" => IpcTarget::Visualizer,
        "volume" | "audio" | "mixer" | "vol" => IpcTarget::Volume,
        "bluetooth" | "bt" => IpcTarget::Bluetooth,
        "disk" | "disks" | "udisks" | "storage" => IpcTarget::Disk,
        "mpris" | "media" | "player" => IpcTarget::Mpris,
        "quick" | "quickmenu" | "menu" | "power" => IpcTarget::QuickMenu,
        "toolbox" | "tools" | "tool" => IpcTarget::Toolbox,
        "calendar" | "cal" => IpcTarget::Calendar,
        "annotate" | "draw" | "sketch" | "paint" => IpcTarget::Annotate,
        "zoom" | "magnifier" | "mag" => IpcTarget::Zoom,
        "colorpicker" | "picker" | "color" | "eyedropper" => IpcTarget::ColorPicker,
        "screenshot" | "shot" | "ss" => IpcTarget::Screenshot(None),
        "all" => IpcTarget::All,
        _ => return None,
    };

    Some(Msg::IpcControl { action, target })
}
