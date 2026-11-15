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
