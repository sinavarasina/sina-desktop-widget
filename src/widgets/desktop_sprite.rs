//! Desktop-layer Yuki Nagato (`yuki_nagato_gekitou_wii`) surface positioned above the
//! wallpaper (`Layer::Background`) and below all application windows (`Layer::Bottom`).

use waywi::prelude::*;

use crate::app::desktop_sprite::SliceRect;
use crate::app::state::{Msg, State};

pub fn build_desktop_sprite_surface(state: &State) -> Option<Surface<Msg>> {
    if !state.config.sprite.desktop_enabled {
        return None;
    }
    let assets = state.desktop_sprite.active_assets?;
    let eye_asset = state.desktop_sprite.current_eye_asset()?;
    let mouth_asset = state.desktop_sprite.current_mouth_asset()?;
    let geom = state.desktop_sprite.current_geometry();
    let alpha = state.desktop_sprite.opacity().clamp(0.0, 1.0);

    let scale_mult = f32::from(state.config.sprite.desktop_scale_pct) / 100.0;
    let (scale, sprite_w, sprite_h, margin_right) =
        state
            .desktop_sprite
            .layout_metrics_scaled(state.bar_width, state.screen_height, scale_mult);

    // Snap adjacent slice edges to shared integer coordinates so horizontal slices join with
    // zero subpixel seams at any scale factor.
    let slice_el = |key: &'static str, asset: ImageAsset, r: SliceRect| {
        let x0 = (r.x * scale).round();
        let y0 = (r.y * scale).round();
        let x1 = ((r.x + r.w) * scale).round();
        let y1 = ((r.y + r.h) * scale).round();
        let w = (x1 - x0).max(1.0);
        let h = (y1 - y0).max(1.0);
        Image::new(asset)
            .key(key)
            .position(Position::absolute(x0.logical_px(), y0.logical_px()))
            .size(Size::new(w.logical_px(), h.logical_px()))
            .filter(ImageFilter::Bicubic)
            .opacity(alpha)
    };

    let retention = if state.desktop_sprite.is_fading() {
        BufferRetention::Retain
    } else {
        BufferRetention::ReleaseAfterCommit
    };

    Some(
        Surface::floating("desktop-yuki-sprite", Output::All)
            .layer(Layer::Bottom)
            .ignore_exclusive_zone()
            .anchor(Anchor::BOTTOM | Anchor::RIGHT)
            .margin(Edges::new(
                0.logical_px(),
                margin_right.logical_px(),
                0.logical_px(),
                0.logical_px(),
            ))
            .size(Size::new(sprite_w.logical_px(), sprite_h.logical_px()))
            .input_mode(InputMode::PassThrough)
            .keyboard_interactivity(KeyboardInteractivity::None)
            .buffer_retention(retention)
            .background(Color::TRANSPARENT)
            .child(
                Group::new()
                    .key(("desktop-yuki-pose", state.desktop_sprite.active_idx))
                    .size(Size::new(sprite_w.logical_px(), sprite_h.logical_px()))
                    .child(slice_el("yuki-d-top", assets.top_hair, geom.top_hair))
                    .child(slice_el("yuki-d-eyes", eye_asset, geom.eyes))
                    .child(slice_el("yuki-d-mouth", mouth_asset, geom.mouth))
                    .child(slice_el("yuki-d-body", assets.body, geom.body)),
            ),
    )
}
