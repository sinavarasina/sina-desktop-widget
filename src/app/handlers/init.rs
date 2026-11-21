//! Fast, non-blocking initial state construction; live D-Bus/PipeWire snapshots arrive
//! asynchronously from subscriptions while heavy scans defer to `Msg::StartupHydrate`.

use std::time::SystemTime;

use waywi::desktop::icons::XdgIconLookup;
use waywi::prelude::*;
use waywi::system::LocalDateTime;
use waywi::system::os::{read_hostname, read_os_info, read_uptime};

use crate::app::asset_lru::PathAssetCache;
use crate::app::state::{
    AudioControlTarget, AudioUiState, BluetoothUiState, DesktopSpriteState, DiskUiState,
    DrunState, ForegroundBooster, HoverTarget, MprisState, NotificationStore, QuickMenuState,
    State, SysInfoState, WindowInfoState, WindowScroller,
};
use crate::theme::{GENTOO_MAUVE_SVG, WALLPAPER_PNG};

/// Decodes the embedded wallpaper PNG and marks it `AssetRetention::ReclaimAfterPresent` so
/// its ~33 MiB RGBA8 pixel buffer is immediately reclaimed after presenting to the compositor.
pub fn load_embedded_wallpaper(ctx: &mut EffectCtx<'_>) -> Option<ImageAsset> {
    let wp = ctx.load_image_bytes(WALLPAPER_PNG).ok()?;
    let _ = ctx.set_asset_retention(wp, AssetRetention::ReclaimAfterPresent);
    Some(wp)
}

pub fn init_state(ctx: &mut EffectCtx<'_>) -> Result<State> {
    let logo_asset = ctx.load_image_bytes_sized(GENTOO_MAUVE_SVG, 64).ok();
    #[cfg(not(test))]
    let wallpaper_asset = load_embedded_wallpaper(ctx);
    #[cfg(test)]
    let wallpaper_asset = None;
    let icon_lookup = XdgIconLookup::new();
    let path_asset_cache = PathAssetCache::default();

    let bar_width = ctx
        .outputs()
        .first()
        .map(|o| o.logical_size.width.get())
        .filter(|w| *w >= 320.0)
        .unwrap_or(1920.0);
    let screen_height = ctx
        .outputs()
        .first()
        .map(|o| o.logical_size.height.get())
        .filter(|h| *h >= 240.0)
        .unwrap_or(1080.0);

    let (workspaces, active_ws_id, scroller, window_info, hypr_version) = init_hyprland(ctx);
    let sysinfo = SysInfoState::from_waywi(
        read_os_info().ok(),
        read_hostname().ok(),
        hypr_version,
        read_uptime().ok(),
    );
    let notifications = NotificationStore::load_lazy(None);

    Ok(State {
        logo_asset,
        wallpaper_asset,
        desktop_sprite: DesktopSpriteState::default(),
        icon_lookup,
        path_asset_cache,
        bar_width,
        screen_height,
        workspaces,
        active_ws_id,
        sysinfo,
        scroller,
        window_info,
        booster: ForegroundBooster::default(),
        drun: DrunState::default(),
        notifications,
        mpris: MprisState::default(),
        tray_items: Vec::new(),
        tray_menu: None,
        blink_phase: false,
        audio: None,
        audio_target: AudioControlTarget::Sink,
        audio_ui: AudioUiState::default(),
        cpu: None,
        cpu_temp_celsius: None,
        cpu_power_watts: None,
        cpu_power_tracker: super::super::state::CpuPowerTracker::default(),
        gpu: None,
        memory: None,
        network: None,
        nm: None,
        disk_used_gb: 0.0,
        disk_free_gb: 0.0,
        disk_total_gb: 0.0,
        disk_pct_used: 0.0,
        disk_pct_free: 0.0,
        disk_mounts: Vec::new(),
        udisks: UDisksSnapshot::default(),
        disk_ui: DiskUiState::default(),
        bluetooth: BluetoothSnapshot::default(),
        bt_ui: BluetoothUiState::default(),
        now: LocalDateTime::from_system_time(SystemTime::now()),
        clock_alt: false,
        calendar_open: false,
        cal_year_mode: false,
        cal_month_offset: 0,
        clock_menu_target: false,
        clock_menu_progress: 0.0,
        clock_menu_leave_deadline: None,
        quick_menu: QuickMenuState::default(),
        hovered: HoverTarget::None,
        hover_anchor_x: None,
        startup_hydrated: false,
        drawer_target: false,
        drawer_progress: 0.0,
        drawer_leave_deadline: None,
    })
}

fn init_hyprland(
    ctx: &EffectCtx<'_>,
) -> (
    Vec<WorkspaceInfo>,
    i32,
    WindowScroller,
    WindowInfoState,
    Option<waywi::compositor::hyprland::HyprlandVersion>,
) {
    let mut scroller = WindowScroller::default();
    let mut window_info = WindowInfoState::default();
    let Ok(hc) = ctx.hyprland() else {
        return (
            vec![WorkspaceInfo {
                id: WorkspaceId::new(1),
                name: String::from("1"),
                monitor: String::new(),
                windows: 0,
                focused: true,
            }],
            1,
            scroller,
            window_info,
            None,
        );
    };
    let workspaces = hc.workspaces().unwrap_or_default();
    let active_ws_id = workspaces
        .iter()
        .find(|w| w.focused)
        .map_or(1, |w| w.id.get());
    let hypr_version = hc.version().ok();
    if let Ok(Some(win)) = hc.active_client() {
        scroller.set_window(win.class.clone(), win.title.clone());
        // Defer `.desktop` directory scanning to `Msg::StartupHydrate` after frame 1 paints.
        window_info.set_client(Some(win), &[]);
    }
    (
        workspaces,
        active_ws_id,
        scroller,
        window_info,
        hypr_version,
    )
}
