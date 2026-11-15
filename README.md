# sina-desktop-widget

A native Wayland desktop bar built in Rust using [`waywi`](https://crates.io/crates/waywi/0.1.1) and styled with the Catppuccin Mocha palette.

## Features

- **Left Bar**
  - Launcher logo button (left-click launches `wofi --show drun --allow-images`)
  - Hyprland active & inactive workspace pills with click-to-switch
  - Active window title marquee scroller for long titles
- **Right Bar**
  - System tray (`StatusNotifierItem`) with attention blink and interactive floating D-Bus tray menu
  - PipeWire audio module (scroll to adjust volume, left-click to mute, middle-click to switch between Speaker and Mic)
  - CPU & Memory modules (click to launch `btop` in `footclient`)
  - Network module with Wi-Fi / Modem / Ethernet tooltip details via NetworkManager
  - Expandable hover drawer revealing **GPU**, **Disk (`/`)**, and **Bluetooth** (click to launch `bluetui` in `footclient`)
  - Clock module (left-click toggles 24h time / full date, hover shows floating calendar, right-click toggles month/year view, scroll shifts months)
- **Visuals**
  - 24px top panel with powerline/angled separators and auto-sized floating tooltips
  - Catppuccin Mocha color scheme

## Requirements

- Rust
- Wayland compositor (Hyprland IPC supported for workspaces & active window)
- System services: PipeWire, D-Bus, NetworkManager, BlueZ
- Fonts: `MesloLGS Nerd Font` and `BlexMono Nerd Font`
- Optional launcher/TUI apps for click actions: `wofi`, `footclient`, `btop`, `bluetui`

## Build & Run

```bash
# Check / build
cargo build --release

# Run
cargo run --release

# Run headless tests
cargo test
```

## Project Structure

```text
src/
├── main.rs                 # Entry point & Application trait impl
├── theme.rs                # Catppuccin Mocha colors, dimensions, fonts
├── app/
│   ├── msg.rs              # Message enum
│   ├── types.rs            # HoverTarget, TrayEntry, TrayMenuPopup, AudioControlTarget
│   ├── scroller.rs         # Active window marquee scroller
│   ├── tray_cache.rs       # Tray deduplication & XDG/pixmap icon cache
│   ├── state.rs            # Core State & drawer animation logic
│   ├── handlers/           # init, update, events, subscriptions
│   └── format/             # audio, system (cpu/mem/disk), network, gpu, bluetooth
├── modules/
│   ├── left/               # logo, workspaces, window_scroll
│   ├── right.rs            # Right bar composition
│   ├── tray.rs             # System tray icons
│   ├── drawer.rs           # Expandable GPU/Disk/Bluetooth drawer
│   └── clock.rs            # Clock module
├── widgets/                # calendar, metric, separators, tooltip, tray_menu
└── tests/                  # Headless rendering, interaction, and formatting tests
```
