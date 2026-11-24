//! Shared headless runtime and fixture builders for tests.

use std::sync::Arc;
use std::time::Instant;

use waywi::prelude::*;
use waywi::dbus::{TrayCategory, TrayStatus};
use waywi::system::{
    ByteSize, CpuSnapshot, LoadAverage, MemorySnapshot, NetInterfaceKind, NetInterfaceSnapshot,
    NetLinkState, NetworkSnapshot,
};

use crate::SinaDesktopWidget;
use crate::app::Msg;

pub fn synthetic_rt() -> HeadlessRuntime<SinaDesktopWidget> {
    HeadlessRuntime::<SinaDesktopWidget>::new_with_renderer(
        Capabilities::new(),
        vec![OutputInfo {
            id: OutputId(1),
            name: Some(String::from("eDP-1")),
            logical_size: LogicalSize::new(1920.logical_px(), 1080.logical_px()),
            ..OutputInfo::default()
        }],
        RendererPreference::Software,
    )
    .expect("headless bar runtime boots")
}

pub fn maybe_dump_argb8888_ppm(name: &str, width: u32, height: u32, buf: &[u8]) {
    let Ok(dir) = std::env::var("WAYWI_BAR_DUMP_SCRATCH_DIR") else {
        return;
    };
    let path = std::path::Path::new(&dir).join(format!("{name}.ppm"));
    let mut out = format!("P6\n{width} {height}\n255\n").into_bytes();
    out.reserve(buf.len());
    for px in buf.as_chunks::<4>().0 {
        // Software ARGB8888 little-endian memory layout is [B, G, R, A].
        let (b, g, r, a) = (px[0], px[1], px[2], px[3]);
        // Un-premultiply over dark wallpaper background [30, 30, 46] so alpha compositing reads clearly.
        let inv_a = 255 - a as u16;
        let cr = (r as u16 + (30 * inv_a) / 255).min(255) as u8;
        let cg = (g as u16 + (30 * inv_a) / 255).min(255) as u8;
        let cb = (b as u16 + (46 * inv_a) / 255).min(255) as u8;
        out.extend_from_slice(&[cr, cg, cb]);
    }
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(path, out);
}

pub fn populate_sample_metrics(rt: &mut HeadlessRuntime<SinaDesktopWidget>) {
    rt.enqueue(Msg::HyprEvent(HyprlandEvent::ActiveWindow {
        class: String::from("foot"),
        title: String::from("foot"),
    }));
    rt.enqueue(Msg::AudioUpdated(Arc::new(AudioSnapshot {
        default_sink_id: Some(1),
        sinks: vec![AudioDevice {
            id: 1,
            name: String::from("alsa_output.pci-0000_0c_00.6.analog-stereo"),
            description: String::from("Ryzen HD Audio Controller Line Out"),
            is_default: true,
            volume: AudioVolume {
                linear: 0.24,
                ..AudioVolume::default()
            },
            ..AudioDevice::default()
        }],
        ..AudioSnapshot::default()
    })));
    rt.enqueue(Msg::CpuSampled(CpuSnapshot {
        total_percent: 3.0,
        per_core_percent: vec![3.0],
        per_core_freq_khz: vec![2_500_000],
        load_avg: LoadAverage {
            one: 0.42,
            ..LoadAverage::default()
        },
        logical_cores: 8,
        timestamp: Instant::now(),
    }));
    rt.enqueue(Msg::MemorySampled(MemorySnapshot {
        total: ByteSize::from_kib(16 * 1024 * 1024),
        available: ByteSize::from_kib(10 * 1024 * 1024),
        used: ByteSize::from_kib(4 * 1024 * 1024),
        swap_total: ByteSize::from_kib(4 * 1024 * 1024),
        swap_used: ByteSize::ZERO,
        swap_available: ByteSize::from_kib(4 * 1024 * 1024),
        psi: None,
    }));
    rt.enqueue(Msg::NetworkSampled(NetworkSnapshot {
        interfaces: vec![NetInterfaceSnapshot {
            name: String::from("wlan0"),
            kind: NetInterfaceKind::Wifi,
            link_state: NetLinkState::Up,
            carrier: Some(true),
            link_speed_mbps: Some(866),
            ipv4: Some(String::from("192.168.1.42")),
            ipv6: None,
            gateway: Some(String::from("192.168.1.1")),
            wireless_signal_dbm: Some(-55),
            wireless_freq_mhz: Some(5180),
            rx_total: ByteSize::from_bytes(1024 * 1024),
            tx_total: ByteSize::from_bytes(512 * 1024),
            rx_bytes_per_sec: 196.0,
            tx_bytes_per_sec: 114.0,
            rx_packets_per_sec: 2.0,
            tx_packets_per_sec: 1.0,
        }],
        total_rx_bytes_per_sec: 196.0,
        total_tx_bytes_per_sec: 114.0,
        timestamp: Instant::now(),
    }));
}

pub fn make_tray_snap(bus_name: &str) -> TrayItemSnapshot {
    TrayItemSnapshot {
        service: format!("{bus_name}/StatusNotifierItem"),
        bus_name: bus_name.to_owned(),
        object_path: String::from("/StatusNotifierItem"),
        id: String::from("Fcitx"),
        title: String::from("Fcitx Input Method"),
        category: TrayCategory::ApplicationStatus,
        status: TrayStatus::Active,
        icon_name: String::new(),
        icon_theme_path: None,
        icon_pixmap: Vec::new(),
        attention_icon_name: String::new(),
        overlay_icon_name: String::new(),
        tooltip: None,
        menu_path: Some(String::from("/MenuBar")),
        item_is_menu: false,
    }
}
