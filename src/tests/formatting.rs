//! Metric formatting, tooltip sizing, window scroller, and datetime tests.

use std::time::Instant;

use waywi::prelude::*;
use waywi::system::{
    ByteSize, DiskSnapshot, GpuDeviceSnapshot, GpuSnapshot, GpuVendor, NetInterfaceKind,
    NetInterfaceSnapshot, NetLinkState, NetworkSnapshot, days_in_month,
};

use crate::app::Msg;
use crate::app::state::{HoverTarget, WindowScroller};

use super::support::{maybe_dump_argb8888_ppm, synthetic_rt};

#[test]
fn gpu_metric_formats_clock_and_power_with_memclock_fallback() {
    let mut rt = synthetic_rt();
    // Flush initial live System::gpu subscription tick before injecting deterministic test snapshots.
    rt.step().unwrap();
    let base_gpu = GpuDeviceSnapshot {
        card_name: String::from("card0"),
        vendor: GpuVendor::Amd,
        pci_vendor_id: Some(0x1002),
        pci_device_id: Some(0x73df),
        driver: Some(String::from("amdgpu")),
        model_name: Some(String::from("Radeon RX 6700 XT")),
        utilization_percent: Some(12.0),
        vram_used: None,
        vram_total: None,
        gtt_used: None,
        gtt_total: None,
        core_clock_mhz: Some(500),
        memory_clock_mhz: Some(456),
        temperature_celsius: Some(45.0),
        hotspot_celsius: None,
        power_watts: Some(13.2),
        fan_rpm: None,
    };
    rt.enqueue(Msg::GpuSampled(GpuSnapshot {
        gpus: vec![base_gpu.clone()],
        timestamp: Instant::now(),
    }));
    rt.step().unwrap();
    let m = rt.state().gpu_metric();
    assert_eq!(m.lines[0].text, "500MHz");
    assert_eq!(m.lines[1].text, "13W");
    assert!(m.tooltip.contains("Mem Clock: 456 MHz"));

    let mut no_power = base_gpu;
    no_power.power_watts = None;
    rt.enqueue(Msg::GpuSampled(GpuSnapshot {
        gpus: vec![no_power],
        timestamp: Instant::now(),
    }));
    rt.step().unwrap();
    let m_fallback = rt.state().gpu_metric();
    assert_eq!(m_fallback.lines[1].text, "456MHz");
}

#[test]
fn tooltip_auto_sizing_accommodates_long_text_without_truncation() {
    let mut rt = synthetic_rt();
    rt.enqueue(Msg::AudioUpdated(AudioSnapshot {
        linear: 0.72,
        db: None,
        muted: false,
        default_sink: Some(AudioDevice {
            id: 45,
            name: String::from("alsa_output.pci-0000_0c_00.6.analog-stereo"),
            description: String::from(
                "Ryzen HD Audio Controller Realtek ALC1220 Analog Line-Out (Hardware Route Volume)",
            ),
        }),
        devices: Vec::new(),
        ..AudioSnapshot::default()
    }));
    rt.enqueue(Msg::Hover(HoverTarget::Wireplumber));
    rt.step().unwrap();

    let sz = rt
        .surface_logical_size("bar-tooltip")
        .expect("wireplumber tooltip surface");
    assert!(
        sz.width.get() > 420.0,
        "auto-sized tooltip must expand beyond old 380px cap, got {}",
        sz.width.get()
    );
    if let Some(buf) = rt.rendered_buffer("bar-tooltip") {
        maybe_dump_argb8888_ppm(
            "tooltip_long",
            sz.width.get().round() as u32,
            sz.height.get().round() as u32,
            buf,
        );
    }
}

#[test]
fn workspaces_audio_muted_network_disconnected_and_drawer_formatting() {
    let mut rt = synthetic_rt();
    rt.enqueue(Msg::HyprEvent(HyprlandEvent::WorkspaceV2 {
        id: WorkspaceId::new(1),
        name: String::from("1"),
    }));
    rt.step().unwrap();

    rt.enqueue(Msg::DiskSampled(DiskSnapshot {
        mounts: vec![waywi::system::DiskMount {
            device: String::from("/dev/nvme0n1p2"),
            mount_point: std::path::PathBuf::from("/"),
            fs_type: String::from("btrfs"),
            total: ByteSize::from_bytes(500_000_000_000),
            available: ByteSize::from_bytes(320_000_000_000),
            free: ByteSize::from_bytes(320_000_000_000),
            used: ByteSize::from_bytes(180_000_000_000),
        }],
        io: Vec::new(),
        total_read_bytes_per_sec: 0.0,
        total_write_bytes_per_sec: 0.0,
        timestamp: Instant::now(),
    }));
    rt.enqueue(Msg::AudioUpdated(AudioSnapshot {
        linear: 0.5,
        db: None,
        muted: true,
        default_sink: None,
        devices: Vec::new(),
        ..AudioSnapshot::default()
    }));
    rt.enqueue(Msg::NetworkSampled(NetworkSnapshot {
        interfaces: vec![NetInterfaceSnapshot {
            name: String::from("wlan0"),
            kind: NetInterfaceKind::Wifi,
            link_state: NetLinkState::Down,
            carrier: Some(false),
            link_speed_mbps: None,
            ipv4: None,
            ipv6: None,
            gateway: None,
            wireless_signal_dbm: None,
            wireless_freq_mhz: None,
            rx_total: ByteSize::ZERO,
            tx_total: ByteSize::ZERO,
            rx_bytes_per_sec: 0.0,
            tx_bytes_per_sec: 0.0,
            rx_packets_per_sec: 0.0,
            tx_packets_per_sec: 0.0,
        }],
        total_rx_bytes_per_sec: 0.0,
        total_tx_bytes_per_sec: 0.0,
        timestamp: Instant::now(),
    }));
    rt.enqueue(Msg::BluetoothUpdated(BluetoothSnapshot {
        adapters: vec![BluetoothAdapter {
            object_path: String::from("/org/bluez/hci0"),
            name: String::from("hci0"),
            alias: String::from("ThinkPad BT"),
            powered: true,
            ..Default::default()
        }],
        devices: vec![BluetoothDevice {
            name: String::from("WH-1000XM5"),
            alias: String::from("XM5"),
            connected: true,
            battery_percentage: Some(85),
            ..Default::default()
        }],
    }));
    rt.step().unwrap();

    let (wp_bar, _, wp_muted) = rt.state().wireplumber_display();
    assert!(wp_muted);
    assert_eq!(wp_bar, "\u{f075f} 0%\n\u{f036c} 0%");

    let (net_bar, net_tip, net_disc) = rt.state().network_display();
    assert!(net_disc);
    assert_eq!(net_bar, "\u{f092d} 0/0");
    assert_eq!(net_tip, "Disconnected");

    let (disk_bar, _) = rt.state().disk_display();
    assert_eq!(disk_bar, "\u{f0a0} 180.0G Used\n\u{f0a0} 320.0G Free");

    let (cpu_bar, cpu_tip) = rt.state().cpu_display();
    assert!(cpu_bar.contains('%') && cpu_tip.contains("Usage:"));

    let (mem_bar, mem_tip) = rt.state().memory_display();
    assert!(mem_bar.contains('%') && mem_tip.contains("RAM:"));

    let (bt_bar, bt_tip) = rt.state().bluetooth_display();
    assert_eq!(bt_bar, "\u{f00b1} connected\n\u{f0949} 85%");
    assert!(bt_tip.contains("XM5 (85% Battery)"));

    // Verify CRUST top-left pixel matches little-endian ARGB8888 [B=27, G=17, R=17, A=255].
    let buf = rt.rendered_buffer("bar").expect("rendered buffer");
    assert_eq!(&buf[0..4], &[27, 17, 17, 255]);
}

#[test]
fn window_scroller_truncates_and_scrolls_long_utf8_titles_safely() {
    let mut scroller = WindowScroller::default();
    let long = "Firefox — Building WayWi Native Compositor Bar with Unicode \u{2728} Icons";
    scroller.set_window("firefox".into(), long.into());
    assert!(scroller.needs_scroll());
    let initial = scroller.visible_text();
    assert_eq!(initial.chars().count(), crate::theme::MAX_WINDOW_CHARS);

    for _ in 0..10 {
        scroller.advance();
    }
    let scrolled = scroller.visible_text();
    assert_eq!(scrolled.chars().count(), crate::theme::MAX_WINDOW_CHARS);
    assert_ne!(scrolled, initial);
}

#[test]
fn native_waywi_datetime_units_and_calendar_work_accurately() {
    assert_eq!(days_in_month(2024, 2), 29);
    assert_eq!(days_in_month(2025, 2), 28);
    let dt = LocalDateTime::from_system_time(
        std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000),
    );
    assert!(dt.year >= 2023);
    assert!(!dt.format_hm(TimeFormat::H12).is_empty());
}
