//! Headless rendering and steady-state allocation tests.

use super::support::{maybe_dump_argb8888_ppm, populate_sample_metrics, synthetic_rt};

#[test]
fn renders_24px_bar_with_non_empty_pixels_and_zero_steady_state_heap_fallbacks() {
    let mut rt = synthetic_rt();
    populate_sample_metrics(&mut rt);
    rt.step().unwrap();

    let buf = rt.rendered_buffer("bar").expect("rendered bar buffer");
    // 1920 * 24 * 4 = 184_320 bytes
    assert_eq!(buf.len(), 1920 * 24 * 4);
    maybe_dump_argb8888_ppm("bar_default", 1920, 24, buf);

    // Verify left edge has rendered Gentoo/crust opaque pixels and right edge has clock pixels,
    // while transparent center gap stays alpha == 0.
    assert_eq!(buf[3], 255);
    assert_eq!(buf[(12 * 1920 + 960) * 4 + 3], 0);
    assert_eq!(buf[(12 * 1920 + 1915) * 4 + 3], 255);

    // Steady state step must have zero arena heap fallbacks.
    rt.step().unwrap();
    assert_eq!(rt.arena_heap_fallbacks(), 0);
}

#[test]
fn embedded_wallpaper_reclaims_pixels_after_present_and_sprite_evicts_on_close() {
    use crate::app::Msg;

    let mut rt = synthetic_rt();
    assert!(rt.state().wallpaper_asset.is_none());

    // Loading the embedded 4K wallpaper with ReclaimAfterPresent presents it and WayWi
    // immediately reclaims the 33 MB decoded RGBA8 pixels down to the 241 KB PNG bytes.
    rt.enqueue(Msg::TestLoadWallpaper);
    rt.step().unwrap();

    let wp = rt
        .state()
        .wallpaper_asset
        .expect("embedded wallpaper loaded on StartupHydrate");
    let wp_buf = rt
        .rendered_buffer("desktop-wallpaper")
        .expect("wallpaper surface rendered");
    assert_eq!(wp_buf.len(), 1920 * 1080 * 4);
    assert!(
        !rt.asset_pixels_resident(wp),
        "decoded 4K wallpaper RGBA8 pixels must be reclaimed after presenting"
    );

    // Opening Quick Status lazily decodes the 4 Yuki Nagato sprite assets.
    rt.enqueue(Msg::ClockMenuClick);
    for _ in 0..16 {
        rt.enqueue(Msg::QuickMenuAnimTick);
    }
    rt.step().unwrap();

    let sprites = rt
        .state()
        .quick_menu
        .sprite_assets
        .clone()
        .expect("Yuki sprite assets loaded");
    assert!(rt.asset_pixels_resident(sprites.base));
    assert!(rt.asset_pixels_resident(sprites.eye_forward.asset));

    // Closing Quick Status finishes slide-up and evicts all 4 sprite RGBA8 buffers back to ~52 KB.
    rt.enqueue(Msg::CloseQuickMenu);
    for _ in 0..16 {
        rt.enqueue(Msg::QuickMenuAnimTick);
    }
    rt.step().unwrap();
    assert!(!rt.state().quick_menu.is_popup_visible());
    assert!(!rt.asset_pixels_resident(sprites.base));
    assert!(!rt.asset_pixels_resident(sprites.eye_forward.asset));
    assert!(!rt.asset_pixels_resident(sprites.eye_glance.asset));
    assert!(!rt.asset_pixels_resident(sprites.mouth.asset));

    // Re-opening Quick Status transparently re-decodes the sprite assets and renders again.
    rt.enqueue(Msg::ClockMenuClick);
    for _ in 0..16 {
        rt.enqueue(Msg::QuickMenuAnimTick);
    }
    rt.step().unwrap();
    assert!(rt.rendered_buffer("bar-quick-menu").is_some());
    assert!(rt.asset_pixels_resident(sprites.base));
}

#[test]
fn desktop_yuki_sprite_fades_rotates_on_workspace_switch_blinks_and_evicts_old_variants() {
    use waywi::prelude::*;

    use crate::app::Msg;
    use crate::app::desktop_sprite::{BlinkPhase, MouthFrame, SpriteFadePhase};

    let mut rt = synthetic_rt();
    rt.enqueue(Msg::TestLoadDesktopSprite);
    for _ in 0..45 {
        rt.enqueue(Msg::DesktopSpriteFadeTick);
    }
    rt.step().unwrap();

    assert_eq!(rt.state().desktop_sprite.active_idx, 0);
    assert!(matches!(
        rt.state().desktop_sprite.phase,
        SpriteFadePhase::Hold { .. }
    ));
    assert!(rt.rendered_buffer("desktop-yuki-sprite").is_some());

    let sp00_assets = rt.state().desktop_sprite.active_assets.unwrap();
    let sp00_norm_eye = sp00_assets.eye_open;
    let sp00_norm_mouth_open = sp00_assets.mouth_open.expect("SP00 has speaking mouth");
    let sp00_body = sp00_assets.body;

    // Incoming notification triggers desktop sprite mouth reaction (and respects DND).
    assert_eq!(
        rt.state().desktop_sprite.current_mouth_frame(),
        MouthFrame::Closed
    );
    rt.enqueue(Msg::NotificationEvent(Box::new(
        NotificationEvent::Received(Box::new(DesktopNotification {
            id: 77,
            app_name: String::from("notify-send"),
            summary: String::from("Ping"),
            body: String::from("Hello Yuki"),
            ..DesktopNotification::default()
        })),
    )));
    rt.step().unwrap();
    assert_eq!(
        rt.state().desktop_sprite.current_mouth_frame(),
        MouthFrame::HalfOpen
    );
    rt.enqueue(Msg::DesktopSpriteMouthTick);
    rt.step().unwrap();
    assert_eq!(
        rt.state().desktop_sprite.current_mouth_frame(),
        MouthFrame::Open
    );
    for _ in 0..8 {
        rt.enqueue(Msg::DesktopSpriteMouthTick);
    }
    rt.step().unwrap();
    assert_eq!(
        rt.state().desktop_sprite.current_mouth_frame(),
        MouthFrame::Closed
    );
    assert!(rt.state().desktop_sprite.mouth_step.is_none());

    // DND suppresses desktop sprite mouth reaction on incoming notification.
    rt.enqueue(Msg::NotifCenterToggleDnd);
    rt.enqueue(Msg::NotificationEvent(Box::new(
        NotificationEvent::Received(Box::new(DesktopNotification {
            id: 78,
            app_name: String::from("notify-send"),
            summary: String::from("Muted"),
            ..DesktopNotification::default()
        })),
    )));
    rt.step().unwrap();
    assert!(rt.state().desktop_sprite.mouth_step.is_none());
    rt.enqueue(Msg::NotifCenterToggleDnd);
    rt.step().unwrap();

    // Blink cycles cleanly across Open -> HalfClosing -> Closed -> HalfOpening -> Open.
    assert_eq!(BlinkPhase::Open.next(), BlinkPhase::HalfClosing);
    assert_eq!(BlinkPhase::HalfClosing.next(), BlinkPhase::Closed);
    assert_eq!(BlinkPhase::Closed.next(), BlinkPhase::HalfOpening);
    assert_eq!(BlinkPhase::HalfOpening.next(), BlinkPhase::Open);
    for _ in 0..4 {
        rt.enqueue(Msg::DesktopSpriteBlinkTick);
    }
    rt.step().unwrap();
    assert_eq!(rt.state().desktop_sprite.blink, BlinkPhase::Open);
    rt.enqueue(Msg::DesktopSpriteBlinkTick);
    rt.step().unwrap();
    assert_eq!(rt.state().desktop_sprite.blink, BlinkPhase::HalfClosing);

    // Switching Hyprland workspace swaps INSTANTLY to variant 1 (Glasses) with no fade delay
    // and evicts variant 0's unique eye pixels immediately.
    rt.enqueue(Msg::HyprEvent(HyprlandEvent::WorkspaceV2 {
        id: WorkspaceId::new(2),
        name: String::from("2"),
    }));
    rt.step().unwrap();
    assert_eq!(rt.state().desktop_sprite.active_idx, 1);
    assert!(matches!(
        rt.state().desktop_sprite.phase,
        SpriteFadePhase::Hold { elapsed_secs } if elapsed_secs <= 2
    ));
    assert!(!rt.asset_pixels_resident(sp00_norm_eye));
    assert!(!rt.asset_pixels_resident(sp00_norm_mouth_open));

    // Rotating instantly to variant 2 (SP01 angled) evicts the old SP00 body pixels cleanly.
    rt.enqueue(Msg::HyprEvent(HyprlandEvent::WorkspaceV2 {
        id: WorkspaceId::new(3),
        name: String::from("3"),
    }));
    rt.step().unwrap();
    assert_eq!(rt.state().desktop_sprite.active_idx, 2);
    assert!(!rt.asset_pixels_resident(sp00_body));

    // Idle rotation (after DESKTOP_SPRITE_HOLD_SECS) uses slow fade-out -> variant 3 (SP02 book) -> fade-in.
    for _ in 0..crate::app::desktop_sprite::DESKTOP_SPRITE_HOLD_SECS {
        rt.enqueue(Msg::TimeTick(waywi::system::TimeSnapshot {
            wall_clock: std::time::SystemTime::UNIX_EPOCH,
            monotonic: std::time::Instant::now(),
        }));
    }
    rt.step().unwrap();
    assert!(matches!(
        rt.state().desktop_sprite.phase,
        SpriteFadePhase::FadingOut { next_idx: 3, .. }
    ));
    for _ in 0..90 {
        rt.enqueue(Msg::DesktopSpriteFadeTick);
    }
    rt.step().unwrap();
    assert_eq!(rt.state().desktop_sprite.active_idx, 3);
    assert!(matches!(
        rt.state().desktop_sprite.phase,
        SpriteFadePhase::Hold { .. }
    ));
    let sp02_size = rt
        .surface_logical_size("desktop-yuki-sprite")
        .expect("SP02 surface rendered");
    assert!(sp02_size.height.get() > 800.0);
    assert!(sp02_size.width.get() > 600.0);
}
