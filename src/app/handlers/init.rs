//! Initial state construction and snapshot seeding.

use std::collections::HashMap;
use std::time::SystemTime;

use waywi::desktop::icons::XdgIconLookup;
use waywi::prelude::*;
use waywi::system::LocalDateTime;

use crate::app::state::{
    AudioControlTarget, HoverTarget, NotificationStore, State, TrayEntry, WindowScroller,
    resolve_tray_icon, upsert_tray_entry_into,
};
use crate::theme::GENTOO_MAUVE_SVG;

pub fn init_state(ctx: &mut EffectCtx<'_>) -> Result<State> {
    let logo_asset = ctx.load_image_bytes_sized(GENTOO_MAUVE_SVG, 64).ok();
    let mut icon_lookup = XdgIconLookup::new();
    let mut path_asset_cache = HashMap::new();

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

    let (workspaces, active_ws_id, scroller) = init_hyprland(ctx);
    let audio = init_audio(ctx);
    let nm = init_network_manager(ctx);
    let bluetooth = init_bluetooth(ctx);
    let tray_items = init_tray(ctx, &mut icon_lookup, &mut path_asset_cache);
    let notifications = NotificationStore::load_lazy(None, &mut icon_lookup, &mut path_asset_cache, ctx);

    Ok(State {
        logo_asset,
        icon_lookup,
        path_asset_cache,
        bar_width,
        screen_height,
        workspaces,
        active_ws_id,
        scroller,
        notifications,
        tray_items,
        tray_menu: None,
        blink_phase: false,
        audio,
        audio_target: AudioControlTarget::Sink,
        cpu: None,
        gpu: None,
        memory: None,
        network: None,
        nm,
        disk_used_gb: 0.0,
        disk_free_gb: 0.0,
        disk_total_gb: 0.0,
        disk_pct_used: 0.0,
        disk_pct_free: 0.0,
        bluetooth,
        now: LocalDateTime::from_system_time(SystemTime::now()),
        clock_alt: false,
        cal_year_mode: false,
        cal_month_offset: 0,
        hovered: HoverTarget::None,
        drawer_target: false,
        drawer_progress: 0.0,
        drawer_leave_deadline: None,
    })
}

fn init_hyprland(ctx: &EffectCtx<'_>) -> (Vec<WorkspaceInfo>, i32, WindowScroller) {
    let mut scroller = WindowScroller::default();
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
        );
    };
    let workspaces = hc.workspaces().unwrap_or_default();
    let active_ws_id = workspaces
        .iter()
        .find(|w| w.focused)
        .map_or(1, |w| w.id.get());
    if let Ok(Some(win)) = hc.active_client() {
        scroller.set_window(win.class, win.title);
    }
    (workspaces, active_ws_id, scroller)
}

fn init_audio(ctx: &EffectCtx<'_>) -> Option<AudioSnapshot> {
    ctx.audio().ok().and_then(|ac| ac.snapshot().ok().flatten())
}

fn init_network_manager(ctx: &EffectCtx<'_>) -> Option<NetworkManagerSnapshot> {
    ctx.dbus()
        .ok()
        .and_then(|d| d.network_manager_snapshot().ok())
}

fn init_bluetooth(ctx: &EffectCtx<'_>) -> BluetoothSnapshot {
    ctx.dbus()
        .ok()
        .and_then(|d| d.bluetooth_snapshot().ok())
        .unwrap_or_default()
}

fn init_tray(
    ctx: &mut EffectCtx<'_>,
    lookup: &mut XdgIconLookup,
    cache: &mut HashMap<String, ImageAsset>,
) -> Vec<TrayEntry> {
    let Ok(dbus) = ctx.dbus() else {
        return Vec::new();
    };
    let Ok(services) = dbus.tray_list_items() else {
        return Vec::new();
    };
    let mut snapshots = Vec::new();
    for svc in services {
        if let Ok(snap) = dbus.tray_snapshot(&svc) {
            snapshots.push(snap);
        }
    }
    let mut entries = Vec::new();
    for snap in snapshots {
        let (icon, icon_signature) = resolve_tray_icon(&snap, lookup, cache, ctx);
        upsert_tray_entry_into(
            &mut entries,
            TrayEntry {
                snapshot: snap,
                icon,
                icon_signature,
            },
        );
    }
    entries
}
